// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize, Debug)]
struct ErrorReason {
    reason: String,
}

struct ExternalError {
    code: StatusCode,
    reason: ErrorReason,
    _description: &'static str,
}

impl ExternalError {
    fn new(code: StatusCode, reason: &'static str, description: &'static str) -> Self {
        ExternalError {
            code,
            reason: ErrorReason {
                reason: reason.to_string(),
            },
            _description: description,
        }
    }
}

impl IntoResponse for ExternalError {
    fn into_response(self) -> Response {
        (self.code, Json(self.reason)).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Unable to get interface '<?>' to forward packets")]
    InterfaceError,
    #[error("Unable to use interface '<?>' to forward packets")]
    TransientError,
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        match self {
            ProxyError::InterfaceError => {
                ExternalError::new(StatusCode::INTERNAL_SERVER_ERROR,
                                   "Unable to get interface for forwarding packets",
                                   "unable to access interface via AF_PACKET (permission mismatch or interface not found)")
                    .into_response()
            }
            ProxyError::TransientError => {
                ExternalError::new(StatusCode::CONFLICT,
                                   "Currently unable to start forwarding packets",
                                   "could not get lock for interface task - retry later")
                    .into_response()
            }
        }.into_response()
    }
}
