// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Syncs `PhysiologicalState` to tpt-soma.
//!
//! Soma's device-state ingest is Arrow Flight `DoPut` on the `device_state`
//! subject. This crate builds the batches soma expects and sends them over
//! `tpt-hands-grpc`. The Arrow columnar and IPC encoding comes from
//! `tpt-soma-flight`, which is MIT OR Apache-2.0 and has no arrow crate.
//!
//! Every batch is checked here against the rules soma enforces, so a snapshot that
//! soma would refuse is refused before anything is sent. Soma commits each batch
//! as it goes, so a refusal after the first batch would leave a partial write.
//!
//! Semantics, stated plainly:
//! - Each call is at-least-once. Soma has no idempotency key, so a retry after a
//!   timeout that followed a successful send can write the rows twice. This crate
//!   never retries on its own.
//! - `healing_progress` is 0.0 unless the pathway is severed. Soma's column is not
//!   nullable, so 0.0 means "not healing" only for non-severed states.
//! - Values are widened from `f32` to `f64` without rounding.
//!
//! Not verified here: soma's own write path (authentication, audit and the database
//! insert) needs a running database, so it is tested on the soma side only.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use tpt_hands_grpc::{Client, Code, Status};
use tpt_hands_sdk::{PathwayState, PhysiologicalState};
use tpt_soma_flight::columnar::{Column, DataType, Field, RecordBatch, Schema};
use tpt_soma_flight::data::{batch_flight_data, schema_flight_data};
use tpt_soma_flight::pb::{FlightData, FlightDescriptor, Message, PutResult};

/// Subject soma's `DoPut` accepts for device state.
pub const SUBJECT: &str = "device_state";
/// Full method path of Flight `DoPut`.
pub const DO_PUT_PATH: &str = "/arrow.flight.protocol.FlightService/DoPut";
/// Rows per record batch. Soma's own limit is 10,000. A snapshot is chunked below it.
pub const MAX_ROWS_PER_BATCH: usize = 5_000;
/// Longest tool id accepted. Soma requires only non-empty; this is a local bound.
pub const MAX_TOOL_ID_LEN: usize = 128;

/// Why the adapter refused a snapshot, or a call failed.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    EmptyToolId,
    ToolIdTooLong(usize),
    NegativeTimestamp,
    TimestampOutOfRange,
    NonFinite(&'static str),
    OutOfRange(&'static str),
    /// Batches could not be built (their schema or lengths are inconsistent).
    Batch(String),
    /// There is nothing to send.
    NothingToSend,
    /// The bearer token is empty or contains characters that cannot go in a header.
    BadToken,
    /// Soma rejected the token (expired or revoked). Get a new one and retry.
    Unauthenticated(String),
    /// The token is valid but not allowed to write device state.
    PermissionDenied(String),
    /// Soma refused the data (for example a batch failed its validation).
    Rejected(String),
    /// Soma could not be reached in time. The write may or may not have happened.
    Unavailable(String),
    /// Any other gRPC status.
    Server(Status),
    /// Soma's reply did not have the expected shape.
    BadResponse(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyToolId => write!(f, "tool id is empty"),
            Self::ToolIdTooLong(n) => {
                write!(f, "tool id is {n} bytes, the limit is {MAX_TOOL_ID_LEN}")
            }
            Self::NegativeTimestamp => write!(f, "timestamp is before the Unix epoch"),
            Self::TimestampOutOfRange => write!(f, "timestamp does not fit in 64-bit microseconds"),
            Self::NonFinite(n) => write!(f, "`{n}` is not finite"),
            Self::OutOfRange(n) => write!(f, "`{n}` is outside 0 to 1"),
            Self::Batch(m) => write!(f, "batch error: {m}"),
            Self::NothingToSend => write!(f, "no batches to send"),
            Self::BadToken => write!(f, "bearer token is empty or not visible ASCII"),
            Self::Unauthenticated(m) => write!(f, "unauthenticated: {m}"),
            Self::PermissionDenied(m) => write!(f, "permission denied: {m}"),
            Self::Rejected(m) => write!(f, "rejected by soma: {m}"),
            Self::Unavailable(m) => write!(f, "unavailable: {m}"),
            Self::Server(s) => write!(f, "{s}"),
            Self::BadResponse(m) => write!(f, "bad response from soma: {m}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// The column layout soma's `device_state` write path expects. Every column is
/// non-nullable.
pub fn schema() -> Schema {
    Schema::new(vec![
        Field::new("tool_id", DataType::Utf8, false),
        Field::new("timestamp_us", DataType::Int64, false),
        Field::new("structural_integrity", DataType::Float64, false),
        Field::new("pathway_id", DataType::Int32, false),
        Field::new("pathway_state", DataType::Utf8, false),
        Field::new("healing_progress", DataType::Float64, false),
        Field::new("resistance_ohms", DataType::Float64, false),
        Field::new("max_bandwidth_mbps", DataType::Float64, false),
    ])
}

/// The string soma stores for a pathway state. Exhaustive on purpose: a new
/// variant in the SDK fails to compile here until it is given a code.
pub fn state_code(state: &PathwayState) -> &'static str {
    match state {
        PathwayState::Intact => "intact",
        PathwayState::Severed { .. } => "severed",
        PathwayState::Hypertrophied => "hypertrophied",
    }
}

/// Converts a time to the microseconds since the Unix epoch that soma stores.
pub fn timestamp_us(now: SystemTime) -> Result<i64, AdapterError> {
    let since = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AdapterError::NegativeTimestamp)?;
    i64::try_from(since.as_micros()).map_err(|_| AdapterError::TimestampOutOfRange)
}

fn check_tool_id(tool_id: &str) -> Result<(), AdapterError> {
    if tool_id.is_empty() {
        return Err(AdapterError::EmptyToolId);
    }
    if tool_id.len() > MAX_TOOL_ID_LEN {
        return Err(AdapterError::ToolIdTooLong(tool_id.len()));
    }
    Ok(())
}

fn finite(name: &'static str, v: f32) -> Result<f64, AdapterError> {
    if v.is_finite() {
        Ok(f64::from(v))
    } else {
        Err(AdapterError::NonFinite(name))
    }
}

fn unit(name: &'static str, v: f32) -> Result<f64, AdapterError> {
    let x = finite(name, v)?;
    if (0.0..=1.0).contains(&x) {
        Ok(x)
    } else {
        Err(AdapterError::OutOfRange(name))
    }
}

/// One validated row, in the order of `schema()`.
struct Row {
    tool_id: String,
    timestamp_us: i64,
    structural_integrity: f64,
    pathway_id: i32,
    pathway_state: &'static str,
    healing_progress: f64,
    resistance_ohms: f64,
    max_bandwidth_mbps: f64,
}

/// Builds the record batches for one snapshot, one row per pathway, chunked at
/// `MAX_ROWS_PER_BATCH`. Returns no batches for a snapshot with no pathways.
///
/// Every value is checked first. Anything soma would refuse is an error here, so
/// nothing that soma would reject is sent.
pub fn batches(
    tool_id: &str,
    timestamp_us: i64,
    state: &PhysiologicalState,
) -> Result<Vec<RecordBatch>, AdapterError> {
    check_tool_id(tool_id)?;
    if timestamp_us < 0 {
        return Err(AdapterError::NegativeTimestamp);
    }
    let integrity = unit("structural_integrity", state.structural_integrity)?;

    let mut rows = Vec::with_capacity(state.pathways.len());
    for p in &state.pathways {
        let healing_progress = match p.state {
            PathwayState::Severed { healing_progress } => {
                unit("healing_progress", healing_progress)?
            }
            PathwayState::Intact | PathwayState::Hypertrophied => 0.0,
        };
        rows.push(Row {
            tool_id: tool_id.to_string(),
            timestamp_us,
            structural_integrity: integrity,
            pathway_id: i32::from(p.id.0),
            pathway_state: state_code(&p.state),
            healing_progress,
            resistance_ohms: finite("resistance_ohms", p.resistance_ohms)?,
            max_bandwidth_mbps: finite("max_bandwidth_mbps", p.max_bandwidth_mbps)?,
        });
    }

    let schema = Arc::new(schema());
    rows.chunks(MAX_ROWS_PER_BATCH)
        .map(|chunk| batch_of(&schema, chunk))
        .collect()
}

fn batch_of(schema: &Arc<Schema>, rows: &[Row]) -> Result<RecordBatch, AdapterError> {
    let columns = vec![
        Column::utf8(rows.iter().map(|r| r.tool_id.clone())),
        Column::int64(rows.iter().map(|r| r.timestamp_us)),
        Column::float64(rows.iter().map(|r| r.structural_integrity)),
        Column::int32(rows.iter().map(|r| r.pathway_id)),
        Column::utf8(rows.iter().map(|r| r.pathway_state)),
        Column::float64(rows.iter().map(|r| r.healing_progress)),
        Column::float64(rows.iter().map(|r| r.resistance_ohms)),
        Column::float64(rows.iter().map(|r| r.max_bandwidth_mbps)),
    ];
    RecordBatch::try_new(Arc::clone(schema), columns)
        .map_err(|e| AdapterError::Batch(e.to_string()))
}

/// The `DoPut` request messages for a put: the schema message (which carries the
/// `device_state` descriptor, and only it does), then one message per batch.
/// Each returned `Vec<u8>` is one protobuf `FlightData`, ready to be framed.
pub fn put_messages(batches: &[RecordBatch]) -> Result<Vec<Vec<u8>>, AdapterError> {
    let Some(first) = batches.first() else {
        return Err(AdapterError::NothingToSend);
    };
    let schema = first.schema();

    let mut head = schema_flight_data(&schema);
    head.flight_descriptor = Some(FlightDescriptor::new_cmd(Bytes::from_static(
        SUBJECT.as_bytes(),
    )));
    let mut messages = vec![head.encode_to_vec()];

    for batch in batches {
        if batch.schema() != schema {
            return Err(AdapterError::Batch(
                "batches in one put must share a schema".into(),
            ));
        }
        let data: FlightData =
            batch_flight_data(batch).map_err(|e| AdapterError::Batch(e.to_string()))?;
        messages.push(data.encode_to_vec());
    }
    Ok(messages)
}

/// A bearer token for `authorization`. It is checked for header safety, and it
/// never appears in `Debug` output.
#[derive(Clone)]
pub struct BearerToken(String);

impl BearerToken {
    pub fn new(raw: impl Into<String>) -> Result<Self, AdapterError> {
        let raw = raw.into();
        if raw.is_empty() || !raw.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(AdapterError::BadToken);
        }
        Ok(Self(raw))
    }
}

impl fmt::Debug for BearerToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BearerToken(<redacted>)")
    }
}

/// Maps a gRPC status from soma to an adapter error. Authentication and
/// permission failures are kept separate, so a caller can refresh a token.
pub fn map_status(status: Status) -> AdapterError {
    match status.code {
        Code::Unauthenticated => AdapterError::Unauthenticated(status.message),
        Code::PermissionDenied => AdapterError::PermissionDenied(status.message),
        Code::InvalidArgument => AdapterError::Rejected(status.message),
        Code::Unavailable | Code::DeadlineExceeded => AdapterError::Unavailable(status.message),
        _ => AdapterError::Server(status),
    }
}

/// Reads the row count from soma's single `PutResult` reply.
pub fn parse_put_result(messages: &[Vec<u8>]) -> Result<u64, AdapterError> {
    let [one] = messages else {
        return Err(AdapterError::BadResponse(format!(
            "expected one PutResult, got {}",
            messages.len()
        )));
    };
    let result =
        PutResult::decode_raw(one).map_err(|e| AdapterError::BadResponse(e.to_string()))?;
    let meta: serde_json::Value = serde_json::from_slice(&result.app_metadata)
        .map_err(|e| AdapterError::BadResponse(e.to_string()))?;
    meta["rows"]
        .as_u64()
        .ok_or_else(|| AdapterError::BadResponse("PutResult has no `rows` count".into()))
}

/// Sends device-state snapshots to one soma endpoint.
pub struct SomaClient {
    client: Client,
    token: BearerToken,
}

impl SomaClient {
    /// `addr` is `host:port`. Use loopback or a trusted network: the token travels in
    /// cleartext, because `tpt-hands-grpc` has no TLS yet.
    pub fn new(addr: &str, token: BearerToken) -> Self {
        Self {
            client: Client::new(addr),
            token,
        }
    }

    /// Writes the batches in one `DoPut` call and returns the rows soma reports.
    /// All batches are validated before the call. Soma writes them in order.
    pub async fn put(
        &self,
        batches: &[RecordBatch],
        timeout: Option<Duration>,
    ) -> Result<u64, AdapterError> {
        let messages = put_messages(batches)?;
        let authorization = format!("Bearer {}", self.token.0);
        let replies = self
            .client
            .call_buffered(
                DO_PUT_PATH,
                &[("authorization", authorization.as_str())],
                &messages,
                timeout,
            )
            .await
            .map_err(map_status)?;
        parse_put_result(&replies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_hands_sdk::{PathwayHealth, PathwayId};
    use tpt_soma_flight::data::flight_data_to_batches;
    use tpt_soma_flight::pb::{FlightData as PbFlightData, Message};

    fn pathway(id: u16, state: PathwayState, resistance: f32) -> PathwayHealth {
        PathwayHealth {
            id: PathwayId(id),
            state,
            resistance_ohms: resistance,
            max_bandwidth_mbps: 100.0,
        }
    }

    fn snapshot(pathways: Vec<PathwayHealth>, integrity: f32) -> PhysiologicalState {
        PhysiologicalState {
            pathways,
            structural_integrity: integrity,
            estimated_recovery_time_ms: 0,
            active_healing_protocols: Vec::new(),
        }
    }

    #[test]
    fn schema_matches_soma_contract() {
        let s = schema();
        let names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "tool_id",
                "timestamp_us",
                "structural_integrity",
                "pathway_id",
                "pathway_state",
                "healing_progress",
                "resistance_ohms",
                "max_bandwidth_mbps",
            ]
        );
        assert!(s.fields.iter().all(|f| !f.nullable));
        let types: Vec<DataType> = s.fields.iter().map(|f| f.data_type).collect();
        assert_eq!(
            types,
            [
                DataType::Utf8,
                DataType::Int64,
                DataType::Float64,
                DataType::Int32,
                DataType::Utf8,
                DataType::Float64,
                DataType::Float64,
                DataType::Float64,
            ]
        );
    }

    #[test]
    fn state_codes_are_the_soma_strings() {
        assert_eq!(state_code(&PathwayState::Intact), "intact");
        assert_eq!(
            state_code(&PathwayState::Severed {
                healing_progress: 0.5
            }),
            "severed"
        );
        assert_eq!(state_code(&PathwayState::Hypertrophied), "hypertrophied");
    }

    #[test]
    fn one_row_per_pathway_with_the_expected_values() {
        let state = snapshot(
            vec![
                pathway(
                    0,
                    PathwayState::Severed {
                        healing_progress: 0.25,
                    },
                    480.0,
                ),
                pathway(65535, PathwayState::Intact, 10.0),
                pathway(2, PathwayState::Hypertrophied, 12.0),
            ],
            0.5,
        );
        let b = batches("hand-0", 1_000, &state).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].num_rows(), 3);
        assert_eq!(b[0].schema().fields.len(), 8);
        let rows = b[0].column(3).as_int32().unwrap().to_vec();
        assert_eq!(rows, vec![Some(0), Some(65535), Some(2)]);
        let progress = b[0].column(5).as_float64().unwrap().to_vec();
        // Healing progress is 0.0 unless the pathway is severed.
        assert_eq!(progress, vec![Some(0.25), Some(0.0), Some(0.0)]);
    }

    #[test]
    fn snapshot_without_pathways_sends_nothing() {
        assert!(
            batches("hand-0", 1, &snapshot(Vec::new(), 1.0))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn refusals_happen_before_anything_is_built() {
        let ok = snapshot(vec![pathway(0, PathwayState::Intact, 10.0)], 1.0);
        assert_eq!(batches("", 1, &ok), Err(AdapterError::EmptyToolId));
        let long = "x".repeat(MAX_TOOL_ID_LEN + 1);
        assert_eq!(
            batches(&long, 1, &ok),
            Err(AdapterError::ToolIdTooLong(MAX_TOOL_ID_LEN + 1))
        );
        assert_eq!(
            batches("hand-0", -1, &ok),
            Err(AdapterError::NegativeTimestamp)
        );
        assert_eq!(
            batches("hand-0", 1, &snapshot(vec![], f32::NAN)),
            Err(AdapterError::NonFinite("structural_integrity"))
        );
        assert_eq!(
            batches("hand-0", 1, &snapshot(vec![], 1.5)),
            Err(AdapterError::OutOfRange("structural_integrity"))
        );
        let bad_resistance = snapshot(vec![pathway(0, PathwayState::Intact, f32::INFINITY)], 1.0);
        assert_eq!(
            batches("hand-0", 1, &bad_resistance),
            Err(AdapterError::NonFinite("resistance_ohms"))
        );
        let bad_healing = snapshot(
            vec![pathway(
                0,
                PathwayState::Severed {
                    healing_progress: 2.0,
                },
                1.0,
            )],
            1.0,
        );
        assert_eq!(
            batches("hand-0", 1, &bad_healing),
            Err(AdapterError::OutOfRange("healing_progress"))
        );
    }

    #[test]
    fn large_snapshots_are_chunked() {
        let n = MAX_ROWS_PER_BATCH * 2 + 7;
        let pathways = (0..n)
            .map(|i| pathway((i % 60000) as u16, PathwayState::Intact, 10.0))
            .collect();
        let b = batches("hand-0", 1, &snapshot(pathways, 1.0)).unwrap();
        let sizes: Vec<usize> = b.iter().map(RecordBatch::num_rows).collect();
        assert_eq!(sizes, vec![MAX_ROWS_PER_BATCH, MAX_ROWS_PER_BATCH, 7]);
    }

    #[test]
    fn timestamps_convert_and_refuse_pre_epoch() {
        let t = UNIX_EPOCH + Duration::from_micros(1_234_567);
        assert_eq!(timestamp_us(t), Ok(1_234_567));
        assert_eq!(
            timestamp_us(UNIX_EPOCH - Duration::from_secs(1)),
            Err(AdapterError::NegativeTimestamp)
        );
    }

    #[test]
    fn put_messages_put_the_descriptor_on_the_first_message_only() {
        let state = snapshot(
            vec![
                pathway(0, PathwayState::Intact, 10.0),
                pathway(
                    1,
                    PathwayState::Severed {
                        healing_progress: 0.5,
                    },
                    480.0,
                ),
            ],
            0.75,
        );
        let b = batches("hand-0", 42, &state).unwrap();
        let messages = put_messages(&b).unwrap();
        assert_eq!(messages.len(), 1 + b.len());

        let first = PbFlightData::decode_raw(&messages[0]).unwrap();
        let descriptor = first
            .flight_descriptor
            .expect("first message carries the descriptor");
        assert_eq!(&descriptor.cmd[..], SUBJECT.as_bytes());
        for later in &messages[1..] {
            assert!(
                PbFlightData::decode_raw(later)
                    .unwrap()
                    .flight_descriptor
                    .is_none()
            );
        }
    }

    #[test]
    fn put_messages_round_trip_through_soma_decoder() {
        let state = snapshot(
            vec![
                pathway(0, PathwayState::Intact, 10.0),
                pathway(
                    7,
                    PathwayState::Severed {
                        healing_progress: 0.5,
                    },
                    480.0,
                ),
            ],
            0.75,
        );
        let b = batches("hand-0", 42, &state).unwrap();
        let wire = put_messages(&b).unwrap();
        let decoded: Vec<PbFlightData> = wire
            .iter()
            .map(|m| PbFlightData::decode_raw(m).unwrap())
            .collect();
        let back = flight_data_to_batches(&decoded).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn nothing_to_send_is_an_error() {
        assert_eq!(put_messages(&[]), Err(AdapterError::NothingToSend));
    }

    #[test]
    fn token_is_checked_and_redacted() {
        assert!(BearerToken::new("{\"subject\":\"x\"}").is_ok());
        assert!(matches!(BearerToken::new(""), Err(AdapterError::BadToken)));
        assert!(matches!(
            BearerToken::new("tok\nen"),
            Err(AdapterError::BadToken)
        ));
        let t = BearerToken::new("secret-value").unwrap();
        let shown = format!("{t:?}");
        assert!(!shown.contains("secret-value"), "{shown}");
    }

    #[test]
    fn statuses_map_to_distinct_errors() {
        assert_eq!(
            map_status(Status::new(Code::Unauthenticated, "expired")),
            AdapterError::Unauthenticated("expired".into())
        );
        assert_eq!(
            map_status(Status::new(Code::PermissionDenied, "scope")),
            AdapterError::PermissionDenied("scope".into())
        );
        assert_eq!(
            map_status(Status::new(Code::InvalidArgument, "bad batch")),
            AdapterError::Rejected("bad batch".into())
        );
        assert_eq!(
            map_status(Status::new(Code::DeadlineExceeded, "slow")),
            AdapterError::Unavailable("slow".into())
        );
    }

    #[test]
    fn put_result_reads_the_row_count() {
        let reply = PutResult {
            app_metadata: Bytes::from_static(br#"{"rows":3}"#),
        }
        .encode_to_vec();
        assert_eq!(parse_put_result(&[reply]), Ok(3));
        assert!(matches!(
            parse_put_result(&[]),
            Err(AdapterError::BadResponse(_))
        ));
        let no_rows = PutResult {
            app_metadata: Bytes::from_static(b"{}"),
        }
        .encode_to_vec();
        assert!(matches!(
            parse_put_result(&[no_rows]),
            Err(AdapterError::BadResponse(_))
        ));
    }
}
