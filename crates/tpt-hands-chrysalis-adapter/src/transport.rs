// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! HTTP/1.1 client for `POST /api/v1/device-commands`.
//!
//! This is built against the proposed interface in `docs/ECOSYSTEM_INTEGRATION.md`,
//! not the real tpt-chrysalis service. That service does not have this endpoint
//! yet. Plain `http://` only. TLS is not built in.

use std::time::Duration;

use bytes::Bytes;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use tpt_hands_sdk::MorphogenesisRequest;

use crate::{
    Ack, DEVICE_COMMAND_PATH, DeviceCommandBody, Outcome, ack_for, interpret, to_device_command,
};

/// How long one submission may take, end to end, unless changed.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Sends device commands to tpt-chrysalis.
pub struct ChrysalisClient {
    http: Client<HttpConnector, Full<Bytes>>,
    base: String,
    token: Option<String>,
    timeout: Duration,
}

impl ChrysalisClient {
    /// `base` is the service root, such as `http://127.0.0.1:8080`. Without a
    /// token, every submission fails before it is sent.
    pub fn new(base: &str, token: Option<String>) -> Self {
        Self {
            http: Client::builder(TokioExecutor::new()).build_http(),
            base: base.trim_end_matches('/').to_string(),
            token,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Changes the end-to-end timeout for one submission.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sends one command and reads chrysalis's answer.
    pub async fn submit(&self, body: &DeviceCommandBody) -> Outcome {
        let Some(token) = &self.token else {
            return Outcome::Failed {
                reason: "no token configured".to_string(),
            };
        };
        let json = match serde_json::to_vec(body) {
            Ok(j) => j,
            Err(e) => {
                return Outcome::Failed {
                    reason: format!("encode body: {e}"),
                };
            }
        };
        let request = match Request::post(format!("{}{}", self.base, DEVICE_COMMAND_PATH))
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Full::new(Bytes::from(json)))
        {
            Ok(r) => r,
            Err(e) => {
                return Outcome::Failed {
                    reason: format!("build request: {e}"),
                };
            }
        };

        let exchange = async {
            let resp = self
                .http
                .request(request)
                .await
                .map_err(|e| format!("transport: {e}"))?;
            let status = resp.status().as_u16();
            let bytes = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| format!("read body: {e}"))?
                .to_bytes();
            Ok::<_, String>((status, String::from_utf8_lossy(&bytes).into_owned()))
        };

        match tokio::time::timeout(self.timeout, exchange).await {
            Ok(Ok((status, text))) => interpret(status, &text),
            Ok(Err(reason)) => Outcome::Failed { reason },
            Err(_) => Outcome::Failed {
                reason: format!("no answer within {:?}", self.timeout),
            },
        }
    }
}

/// Maps a request, sends it, and returns the ack. A request that cannot be
/// mapped is answered locally and never sent.
pub async fn submit_request(client: &ChrysalisClient, id: &str, req: &MorphogenesisRequest) -> Ack {
    match to_device_command(id, req) {
        Ok(body) => ack_for(id, &client.submit(&body).await),
        Err(e) => ack_for(
            id,
            &Outcome::Invalid {
                reason: e.to_string(),
            },
        ),
    }
}
