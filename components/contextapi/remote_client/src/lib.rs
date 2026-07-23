// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::HeaderValue;
use axum::{
    body::Bytes,
    http::{HeaderMap, Method, Uri},
};
use db_interaction::models::aliases::MachineId;
use reqwest::header::CONTENT_LENGTH;
use reqwest::{Client, Response, StatusCode};
use tracing::{debug, error, instrument, warn};

/// A client configured to be used
/// to communicate with remote-hands.
#[derive(Debug, Clone)]
pub struct RemoteClient {
    /// Used for forwarding requests.
    pub client: Client,
}

impl Default for RemoteClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl RemoteClient {
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Send an HTTP request with the given parameters to a
    /// remote-hands peripheral
    #[instrument(skip_all)]
    pub async fn send_remote_request(
        &self,
        method: Method,
        url: Uri,
        machine_id: MachineId,
        mut headers: HeaderMap,
        body: Option<Bytes>,
        is_aux_request: bool,
    ) -> Result<Response, (StatusCode, &'static str)> {
        debug!(
            "Going to issue '{} {}' to remote-hands of machine '{}'",
            method, url, machine_id
        );

        // There are some cases where we don't want to set a body. Define a closure here to avoid duplication.
        let build_request_without_body = |method, headers| {
            self
                    .client
                    .request(method, format!("{url}"))
                    .headers(headers)
                    .build()
                    .map_err(|e| {
                        error!(?e, error.msg = %e, "failed to prepare request for remote-hands service");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error occurred while preparing request to be forwarded internally",
                        )
                    })
        };

        // Build the request to be forwarded
        let request = match body {
            None => build_request_without_body(method, headers),
            Some(body)
                if (body.is_empty() && (method == Method::GET || method == Method::DELETE)) =>
            {
                // TODO: Find out why we often end up here when forwarding GET requests (at least from the serial endpoint)
                warn!(
                    %method,
                    ?body,
                    "empty body found, but should not even be set for this method. Removing redundant request body before forwarding request"
                );
                build_request_without_body(method, headers)
            }
            Some(body) => {
                // The caller and/or middleware may have edited the content, but without re-adjusting the content length. We handle
                // that scenario here.
                match headers.get(CONTENT_LENGTH) {
                    Some(val) if val.as_bytes().ne(&HeaderValue::from(body.len())) => {
                        debug!(
                            "Content Length header does not match body length - recalculating now"
                        );
                        headers.insert(CONTENT_LENGTH, HeaderValue::from(body.len()));
                    }
                    Some(_) => {} /* Length matches */,
                    None => {
                        warn!("Content Length header was not set - setting it now");
                        headers.insert(CONTENT_LENGTH, HeaderValue::from(body.len()));
                    }
                };
                // TODO: Check if we need to go via axum::Body first?
                self
                        .client
                        .request(method, format!("{url}"))
                        .headers(headers)
                        .body(body)
                        .build()
                        .map_err(|e| {
                            error!(?e, error.msg = %e, "failed to prepare request for remote-hands service");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Error occurred while preparing request to be forwarded internally",
                            )
                        })
            }
        }?;

        let response = self.client.execute(request).await.map_err(|e| {
            warn!("remote-hands request failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error occurred while forwarding the request internally",
            )
        })?;

        if is_aux_request {
            // aux requests are forwarded in an unfiltered way
            Ok(response)
        } else if !response.status().is_server_error() {
            // normal remote-hands response handling for 100-499 status codes (all "not our error" responses)
            // TODO: To not leak internal remote-hands details, map the response if we have a error RH type schema
            Ok(response)
        } else {
            // remote-hands response handling for internal errors
            warn!(
                "remote-hands request failed (code {}): {:?}",
                response.status(),
                response
            );
            // Do not leak internal remote-hands details
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error occurred during HTTP communication with another internal component",
            ))
        }
    }
}
