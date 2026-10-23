// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Protobuf wire encoding for the messages in `proto/tpt_hands.proto`.
//!
//! This is a small hand-written codec, not generated code. It avoids `protoc`
//! and `micropb-gen` for now, and it has no dependencies. It follows the
//! protobuf encoding rules that these messages use:
//!
//! - proto3: zero scalars, empty strings and unset messages are not written
//! - oneof: the chosen variant is always written, even when its fields are zero
//! - unknown fields are skipped, so newer senders stay compatible
//!
//! The field numbers must match the `.proto` file. A Go check decodes this
//! output with a real protobuf implementation (see todo.md).

use core::fmt;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

const WIRE_VARINT: u8 = 0;
const WIRE_FIXED64: u8 = 1;
const WIRE_LEN: u8 = 2;
const WIRE_FIXED32: u8 = 5;

/// Why a message could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The bytes end in the middle of a field.
    Truncated,
    /// A varint is too long or otherwise malformed.
    BadVarint,
    /// A field uses a wire type this codec does not handle, or the wrong one for its field.
    BadWireType {
        /// Field number that was read.
        field: u32,
        /// Wire type found on the wire.
        wire_type: u8,
    },
    /// A string field is not valid UTF-8.
    BadUtf8,
    /// An enum field holds a value the schema does not define.
    BadEnum {
        /// Field number that was read.
        field: u32,
        /// Value found on the wire.
        value: u64,
    },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "message is truncated"),
            Self::BadVarint => write!(f, "malformed varint"),
            Self::BadWireType { field, wire_type } => {
                write!(f, "field {field} has unexpected wire type {wire_type}")
            }
            Self::BadUtf8 => write!(f, "string field is not valid UTF-8"),
            Self::BadEnum { field, value } => {
                write!(f, "field {field} has undefined enum value {value}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WireError {}

// ---- Encoding helpers -----------------------------------------------------

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn put_tag(out: &mut Vec<u8>, field: u32, wire_type: u8) {
    put_varint(out, (u64::from(field) << 3) | u64::from(wire_type));
}

fn put_u32(out: &mut Vec<u8>, field: u32, v: u32) {
    if v != 0 {
        put_tag(out, field, WIRE_VARINT);
        put_varint(out, u64::from(v));
    }
}

fn put_u64(out: &mut Vec<u8>, field: u32, v: u64) {
    if v != 0 {
        put_tag(out, field, WIRE_VARINT);
        put_varint(out, v);
    }
}

fn put_bool(out: &mut Vec<u8>, field: u32, v: bool) {
    if v {
        put_tag(out, field, WIRE_VARINT);
        put_varint(out, 1);
    }
}

fn put_f32(out: &mut Vec<u8>, field: u32, v: f32) {
    // Compare bits, so that -0.0 is still written.
    if v.to_bits() != 0 {
        put_tag(out, field, WIRE_FIXED32);
        out.extend_from_slice(&v.to_le_bytes());
    }
}

fn put_str(out: &mut Vec<u8>, field: u32, s: &str) {
    if !s.is_empty() {
        put_bytes(out, field, s.as_bytes());
    }
}

fn put_bytes(out: &mut Vec<u8>, field: u32, bytes: &[u8]) {
    put_tag(out, field, WIRE_LEN);
    put_varint(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

// ---- Decoding helpers -----------------------------------------------------

/// One field's value, before it is interpreted for its declared type.
#[derive(Debug, Clone, Copy)]
enum Value<'a> {
    Varint(u64),
    Fixed32(u32),
    Bytes(&'a [u8]),
}

struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if self.buf.len() < n {
            return Err(WireError::Truncated);
        }
        let (head, tail) = self.buf.split_at(n);
        self.buf = tail;
        Ok(head)
    }

    fn varint(&mut self) -> Result<u64, WireError> {
        let mut v = 0u64;
        for i in 0..10 {
            let byte = self.take(1)?[0];
            // The tenth byte may only carry the top bit of a 64-bit value.
            if i == 9 && byte & 0x7f > 1 {
                return Err(WireError::BadVarint);
            }
            v |= u64::from(byte & 0x7f) << (7 * i);
            if byte < 0x80 {
                return Ok(v);
            }
        }
        Err(WireError::BadVarint)
    }

    fn next_field(&mut self) -> Result<Option<(u32, Value<'a>)>, WireError> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let key = self.varint()?;
        let field = u32::try_from(key >> 3).map_err(|_| WireError::BadVarint)?;
        if field == 0 {
            return Err(WireError::BadVarint);
        }
        let value = match (key & 7) as u8 {
            WIRE_VARINT => Value::Varint(self.varint()?),
            WIRE_FIXED64 => {
                // Not used by these messages. Skip the eight bytes.
                self.take(8)?;
                Value::Varint(0)
            }
            WIRE_LEN => {
                let len = usize::try_from(self.varint()?).map_err(|_| WireError::Truncated)?;
                Value::Bytes(self.take(len)?)
            }
            WIRE_FIXED32 => {
                let b = self.take(4)?;
                Value::Fixed32(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            }
            other => {
                return Err(WireError::BadWireType {
                    field,
                    wire_type: other,
                });
            }
        };
        Ok(Some((field, value)))
    }
}

fn bad_type(field: u32, value: &Value<'_>) -> WireError {
    let wire_type = match value {
        Value::Varint(_) => WIRE_VARINT,
        Value::Fixed32(_) => WIRE_FIXED32,
        Value::Bytes(_) => WIRE_LEN,
    };
    WireError::BadWireType { field, wire_type }
}

fn as_u64(field: u32, v: Value<'_>) -> Result<u64, WireError> {
    match v {
        Value::Varint(x) => Ok(x),
        other => Err(bad_type(field, &other)),
    }
}

fn as_u32(field: u32, v: Value<'_>) -> Result<u32, WireError> {
    u32::try_from(as_u64(field, v)?).map_err(|_| WireError::BadVarint)
}

fn as_bool(field: u32, v: Value<'_>) -> Result<bool, WireError> {
    Ok(as_u64(field, v)? != 0)
}

fn as_f32(field: u32, v: Value<'_>) -> Result<f32, WireError> {
    match v {
        Value::Fixed32(bits) => Ok(f32::from_bits(bits)),
        other => Err(bad_type(field, &other)),
    }
}

fn as_bytes(field: u32, v: Value<'_>) -> Result<&[u8], WireError> {
    match v {
        Value::Bytes(b) => Ok(b),
        other => Err(bad_type(field, &other)),
    }
}

fn as_string(field: u32, v: Value<'_>) -> Result<String, WireError> {
    String::from_utf8(as_bytes(field, v)?.to_vec()).map_err(|_| WireError::BadUtf8)
}

/// Runs `f` on each field in `buf`. Field numbers `f` does not handle must be
/// ignored by `f`, which keeps unknown fields compatible.
fn for_each_field(
    buf: &[u8],
    mut f: impl FnMut(u32, Value<'_>) -> Result<(), WireError>,
) -> Result<(), WireError> {
    let mut reader = Reader { buf };
    while let Some((field, value)) = reader.next_field()? {
        f(field, value)?;
    }
    Ok(())
}

// ---- Messages -------------------------------------------------------------

/// `PathwayState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathwayState {
    /// Working normally.
    #[default]
    Intact,
    /// Torn, and healing if `healing_progress` says so.
    Severed,
    /// Thickened by reinforcement.
    Hypertrophied,
}

impl PathwayState {
    fn to_wire(self) -> u64 {
        match self {
            Self::Intact => 0,
            Self::Severed => 1,
            Self::Hypertrophied => 2,
        }
    }

    fn from_wire(field: u32, v: u64) -> Result<Self, WireError> {
        match v {
            0 => Ok(Self::Intact),
            1 => Ok(Self::Severed),
            2 => Ok(Self::Hypertrophied),
            other => Err(WireError::BadEnum {
                field,
                value: other,
            }),
        }
    }
}

/// `PathwayHealth`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PathwayHealth {
    /// Pathway id (field 1).
    pub id: u32,
    /// State (field 2).
    pub state: PathwayState,
    /// Healing progress, 0.0 to 1.0 (field 3).
    pub healing_progress: f32,
    /// Measured resistance (field 4).
    pub resistance_ohms: f32,
    /// Maximum bandwidth (field 5).
    pub max_bandwidth_mbps: f32,
}

impl PathwayHealth {
    /// Encodes to wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_into(&mut out);
        out
    }

    fn encode_into(&self, out: &mut Vec<u8>) {
        put_u32(out, 1, self.id);
        if self.state != PathwayState::Intact {
            put_tag(out, 2, WIRE_VARINT);
            put_varint(out, self.state.to_wire());
        }
        put_f32(out, 3, self.healing_progress);
        put_f32(out, 4, self.resistance_ohms);
        put_f32(out, 5, self.max_bandwidth_mbps);
    }

    /// Decodes from wire bytes.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        let mut m = Self::default();
        for_each_field(buf, |field, v| {
            match field {
                1 => m.id = as_u32(field, v)?,
                2 => m.state = PathwayState::from_wire(field, as_u64(field, v)?)?,
                3 => m.healing_progress = as_f32(field, v)?,
                4 => m.resistance_ohms = as_f32(field, v)?,
                5 => m.max_bandwidth_mbps = as_f32(field, v)?,
                _ => {}
            }
            Ok(())
        })?;
        Ok(m)
    }
}

/// `PhysiologicalUpdate`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhysiologicalUpdate {
    /// Tool id (field 1).
    pub tool_id: String,
    /// Structural integrity, 0.0 to 1.0 (field 2).
    pub structural_integrity: f32,
    /// Pathway health (field 3, repeated).
    pub pathways: Vec<PathwayHealth>,
    /// Timestamp in microseconds (field 4).
    pub timestamp_us: u64,
}

impl PhysiologicalUpdate {
    /// Encodes to wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_str(&mut out, 1, &self.tool_id);
        put_f32(&mut out, 2, self.structural_integrity);
        for p in &self.pathways {
            // Repeated messages are written even when empty, so each element is kept.
            put_bytes(&mut out, 3, &p.encode());
        }
        put_u64(&mut out, 4, self.timestamp_us);
        out
    }

    /// Decodes from wire bytes.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        let mut m = Self::default();
        for_each_field(buf, |field, v| {
            match field {
                1 => m.tool_id = as_string(field, v)?,
                2 => m.structural_integrity = as_f32(field, v)?,
                3 => m.pathways.push(PathwayHealth::decode(as_bytes(field, v)?)?),
                4 => m.timestamp_us = as_u64(field, v)?,
                _ => {}
            }
            Ok(())
        })?;
        Ok(m)
    }
}

/// `FluidType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FluidType {
    /// Not a valid fluid. proto3 needs a zero value, and this is it.
    #[default]
    Unspecified,
    /// Self-healing elastomer precursor.
    SelfHealingElastomer,
    /// Liquid metal.
    LiquidMetal,
    /// Hydrogel.
    Hydrogel,
}

impl FluidType {
    fn to_wire(self) -> u64 {
        match self {
            Self::Unspecified => 0,
            Self::SelfHealingElastomer => 1,
            Self::LiquidMetal => 2,
            Self::Hydrogel => 3,
        }
    }

    fn from_wire(field: u32, v: u64) -> Result<Self, WireError> {
        match v {
            0 => Ok(Self::Unspecified),
            1 => Ok(Self::SelfHealingElastomer),
            2 => Ok(Self::LiquidMetal),
            3 => Ok(Self::Hydrogel),
            other => Err(WireError::BadEnum {
                field,
                value: other,
            }),
        }
    }
}

/// `HealPathwayAction`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HealPathwayAction {
    /// Pathway to heal (field 1).
    pub pathway_id: u32,
    /// Apply mild heat (field 2).
    pub heat_assist: bool,
}

/// `SproutConduitAction`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SproutConduitAction {
    /// Origin channel (field 1).
    pub origin: u32,
    /// Destination channel (field 2).
    pub destination: u32,
    /// Fluid to use (field 3).
    pub fluid_type: FluidType,
}

/// `ReinforceTendonAction`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReinforceTendonAction {
    /// Tendon to reinforce (field 1).
    pub tendon_id: u32,
    /// Hydrogel volume in millilitres (field 2).
    pub volume_ml: f32,
}

/// `SafetyEnvelope`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SafetyEnvelope {
    /// Maximum pressure in kPa (field 1).
    pub max_pressure_kpa: f32,
    /// Maximum temperature in Celsius (field 2).
    pub max_temperature_c: f32,
    /// Maximum volume in millilitres (field 3).
    pub max_volume_ml: f32,
}

/// The `action` oneof of `MorphogenesisRequest`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MorphAction {
    /// Field 10.
    Heal(HealPathwayAction),
    /// Field 11.
    Sprout(SproutConduitAction),
    /// Field 12.
    Reinforce(ReinforceTendonAction),
}

/// `MorphogenesisRequest`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MorphogenesisRequest {
    /// Tool the action applies to (field 1).
    pub tool_id: String,
    /// Id used to match the ack (field 2). See the GAP note in the `.proto`.
    pub request_id: String,
    /// The action (fields 10 to 12, oneof).
    pub action: Option<MorphAction>,
    /// Safety limits (field 99).
    pub safety_limits: Option<SafetyEnvelope>,
}

impl MorphogenesisRequest {
    /// Encodes to wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_str(&mut out, 1, &self.tool_id);
        put_str(&mut out, 2, &self.request_id);

        // A oneof writes its chosen field even when every value inside is zero.
        match self.action {
            Some(MorphAction::Heal(a)) => {
                let mut b = Vec::new();
                put_u32(&mut b, 1, a.pathway_id);
                put_bool(&mut b, 2, a.heat_assist);
                put_bytes(&mut out, 10, &b);
            }
            Some(MorphAction::Sprout(a)) => {
                let mut b = Vec::new();
                put_u32(&mut b, 1, a.origin);
                put_u32(&mut b, 2, a.destination);
                if a.fluid_type != FluidType::Unspecified {
                    put_tag(&mut b, 3, WIRE_VARINT);
                    put_varint(&mut b, a.fluid_type.to_wire());
                }
                put_bytes(&mut out, 11, &b);
            }
            Some(MorphAction::Reinforce(a)) => {
                let mut b = Vec::new();
                put_u32(&mut b, 1, a.tendon_id);
                put_f32(&mut b, 2, a.volume_ml);
                put_bytes(&mut out, 12, &b);
            }
            None => {}
        }

        if let Some(s) = self.safety_limits {
            let mut b = Vec::new();
            put_f32(&mut b, 1, s.max_pressure_kpa);
            put_f32(&mut b, 2, s.max_temperature_c);
            put_f32(&mut b, 3, s.max_volume_ml);
            put_bytes(&mut out, 99, &b);
        }
        out
    }

    /// Decodes from wire bytes. Later occurrences of the oneof or of
    /// `safety_limits` replace earlier ones, as protobuf requires.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        let mut m = Self::default();
        for_each_field(buf, |field, v| {
            match field {
                1 => m.tool_id = as_string(field, v)?,
                2 => m.request_id = as_string(field, v)?,
                10 => m.action = Some(MorphAction::Heal(decode_heal(as_bytes(field, v)?)?)),
                11 => m.action = Some(MorphAction::Sprout(decode_sprout(as_bytes(field, v)?)?)),
                12 => {
                    m.action = Some(MorphAction::Reinforce(decode_reinforce(as_bytes(
                        field, v,
                    )?)?))
                }
                99 => m.safety_limits = Some(decode_safety(as_bytes(field, v)?)?),
                _ => {}
            }
            Ok(())
        })?;
        Ok(m)
    }
}

fn decode_heal(buf: &[u8]) -> Result<HealPathwayAction, WireError> {
    let mut m = HealPathwayAction::default();
    for_each_field(buf, |field, v| {
        match field {
            1 => m.pathway_id = as_u32(field, v)?,
            2 => m.heat_assist = as_bool(field, v)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(m)
}

fn decode_sprout(buf: &[u8]) -> Result<SproutConduitAction, WireError> {
    let mut m = SproutConduitAction::default();
    for_each_field(buf, |field, v| {
        match field {
            1 => m.origin = as_u32(field, v)?,
            2 => m.destination = as_u32(field, v)?,
            3 => m.fluid_type = FluidType::from_wire(field, as_u64(field, v)?)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(m)
}

fn decode_reinforce(buf: &[u8]) -> Result<ReinforceTendonAction, WireError> {
    let mut m = ReinforceTendonAction::default();
    for_each_field(buf, |field, v| {
        match field {
            1 => m.tendon_id = as_u32(field, v)?,
            2 => m.volume_ml = as_f32(field, v)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(m)
}

fn decode_safety(buf: &[u8]) -> Result<SafetyEnvelope, WireError> {
    let mut m = SafetyEnvelope::default();
    for_each_field(buf, |field, v| {
        match field {
            1 => m.max_pressure_kpa = as_f32(field, v)?,
            2 => m.max_temperature_c = as_f32(field, v)?,
            3 => m.max_volume_ml = as_f32(field, v)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(m)
}

/// `MorphogenesisAck`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MorphogenesisAck {
    /// Request this answers (field 1).
    pub request_id: String,
    /// Whether tpt-chrysalis approved it (field 2).
    pub approved: bool,
    /// Job id tracked by tpt-chrysalis (field 3).
    pub chrysalis_job_id: String,
}

impl MorphogenesisAck {
    /// Encodes to wire bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_str(&mut out, 1, &self.request_id);
        put_bool(&mut out, 2, self.approved);
        put_str(&mut out, 3, &self.chrysalis_job_id);
        out
    }

    /// Decodes from wire bytes.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        let mut m = Self::default();
        for_each_field(buf, |field, v| {
            match field {
                1 => m.request_id = as_string(field, v)?,
                2 => m.approved = as_bool(field, v)?,
                3 => m.chrysalis_job_id = as_string(field, v)?,
                _ => {}
            }
            Ok(())
        })?;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_update() -> PhysiologicalUpdate {
        PhysiologicalUpdate {
            tool_id: "hand-0".into(),
            structural_integrity: 0.5,
            pathways: vec![
                PathwayHealth {
                    id: 0,
                    state: PathwayState::Severed,
                    healing_progress: 0.25,
                    resistance_ohms: 480.0,
                    max_bandwidth_mbps: 100.0,
                },
                PathwayHealth::default(),
            ],
            timestamp_us: 1_234_567,
        }
    }

    #[test]
    fn physiological_update_round_trips() {
        let m = sample_update();
        assert_eq!(PhysiologicalUpdate::decode(&m.encode()).unwrap(), m);
    }

    #[test]
    fn default_message_encodes_to_nothing() {
        assert!(PhysiologicalUpdate::default().encode().is_empty());
        assert!(MorphogenesisAck::default().encode().is_empty());
    }

    #[test]
    fn negative_zero_float_is_kept() {
        let m = PathwayHealth {
            resistance_ohms: -0.0,
            ..Default::default()
        };
        let back = PathwayHealth::decode(&m.encode()).unwrap();
        assert_eq!(back.resistance_ohms.to_bits(), (-0.0_f32).to_bits());
    }

    #[test]
    fn each_morph_action_round_trips() {
        let actions = [
            MorphAction::Heal(HealPathwayAction {
                pathway_id: 3,
                heat_assist: true,
            }),
            MorphAction::Sprout(SproutConduitAction {
                origin: 1,
                destination: 2,
                fluid_type: FluidType::LiquidMetal,
            }),
            MorphAction::Reinforce(ReinforceTendonAction {
                tendon_id: 4,
                volume_ml: 1.5,
            }),
        ];
        for action in actions {
            let m = MorphogenesisRequest {
                tool_id: "hand-0".into(),
                request_id: "req-1".into(),
                action: Some(action),
                safety_limits: Some(SafetyEnvelope {
                    max_pressure_kpa: 200.0,
                    max_temperature_c: 60.0,
                    max_volume_ml: 5.0,
                }),
            };
            assert_eq!(MorphogenesisRequest::decode(&m.encode()).unwrap(), m);
        }
    }

    #[test]
    fn oneof_with_all_zero_fields_is_still_written() {
        let m = MorphogenesisRequest {
            action: Some(MorphAction::Heal(HealPathwayAction::default())),
            ..Default::default()
        };
        let back = MorphogenesisRequest::decode(&m.encode()).unwrap();
        assert_eq!(
            back.action,
            Some(MorphAction::Heal(HealPathwayAction::default()))
        );
    }

    #[test]
    fn ack_round_trips() {
        let m = MorphogenesisAck {
            request_id: "req-1".into(),
            approved: true,
            chrysalis_job_id: "job-77".into(),
        };
        assert_eq!(MorphogenesisAck::decode(&m.encode()).unwrap(), m);
    }

    #[test]
    fn unknown_fields_are_skipped() {
        // Field 50, varint 7, then the real ack fields.
        let mut bytes = Vec::new();
        put_u32(&mut bytes, 50, 7);
        bytes.extend(
            MorphogenesisAck {
                request_id: "r".into(),
                approved: true,
                chrysalis_job_id: String::new(),
            }
            .encode(),
        );
        let back = MorphogenesisAck::decode(&bytes).unwrap();
        assert_eq!(back.request_id, "r");
        assert!(back.approved);
    }

    #[test]
    fn truncated_input_is_an_error() {
        // Every prefix of a valid message must be handled without a panic. Some
        // prefixes are valid messages in their own right, so only the panic
        // freedom is asserted here.
        let bytes = sample_update().encode();
        for cut in 0..bytes.len() {
            let _ = PhysiologicalUpdate::decode(&bytes[..cut]);
        }
        // A cut inside a string field must fail, not silently truncate.
        assert_eq!(
            PhysiologicalUpdate::decode(&[0x0A, 0x05, b'a']),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn undefined_enum_value_is_rejected() {
        // PathwayHealth field 2 (state) with value 9.
        assert_eq!(
            PathwayHealth::decode(&[0x10, 0x09]),
            Err(WireError::BadEnum { field: 2, value: 9 })
        );
    }

    #[test]
    fn wrong_wire_type_is_rejected() {
        // Field 1 (tool_id) sent as a varint instead of a string.
        assert_eq!(
            PhysiologicalUpdate::decode(&[0x08, 0x01]),
            Err(WireError::BadWireType {
                field: 1,
                wire_type: WIRE_VARINT
            })
        );
    }

    #[test]
    fn bad_utf8_string_is_rejected() {
        assert_eq!(
            PhysiologicalUpdate::decode(&[0x0A, 0x01, 0xFF]),
            Err(WireError::BadUtf8)
        );
    }
}
