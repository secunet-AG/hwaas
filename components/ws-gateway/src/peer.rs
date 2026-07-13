// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::Request;
use axum_extra::headers::{Error, Header, HeaderName, HeaderValue};
use schemars::JsonSchema;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower_http::request_id::{MakeRequestId, RequestId};

// A `MakeRequestId` that increments an atomic counter
#[derive(Clone, Default, JsonSchema)]
pub struct Peer {
    counter: Arc<AtomicU64>,
}

impl MakeRequestId for Peer {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        self.counter
            .fetch_add(1, Ordering::SeqCst)
            .to_string()
            .parse()
            .map(RequestId::new)
            .ok()
    }
}

// Static header name used by the MakeRequestId<Peer> middleware to set
// an unique request id.
pub static PEER_HEADER_NAME: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Debug, Clone, JsonSchema)]
pub struct PeerID(String);

impl Display for PeerID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Peer[{}]", self.0))
    }
}

impl Header for PeerID {
    fn name() -> &'static HeaderName {
        &PEER_HEADER_NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(Error::invalid)?;

        let id = value
            .to_str()
            .map(|v| v.to_string())
            .map_err(|_| Error::invalid())?;
        Ok(PeerID(id))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        if let Ok(value) = HeaderValue::from_str(self.0.as_str()) {
            values.extend(std::iter::once(value));
        }
    }
}
