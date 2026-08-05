// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::OperationOutput;
use aide::r#gen::GenContext;
use aide::openapi::Operation;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use connection_handler::ConnectionHandlerError;
use network_type_ids::IDParseError;
use switch::{SwitchApiError, SwitchSetupError};

#[derive(Debug, Clone)]
pub struct ExtApiError(StatusCode, String);

impl ExtApiError {
    pub fn new<S: Into<String>>(code: StatusCode, msg: S) -> Self {
        Self(code, msg.into())
    }
}

impl IntoResponse for ExtApiError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

impl OperationOutput for ExtApiError {
    type Inner = ExtApiError;

    fn inferred_responses(
        _ctx: &mut GenContext,
        _operation: &mut Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        let v = [
            (
                StatusCode::BAD_REQUEST,
                "Returned when invalid inputs were given in the request. \
                Fix: Check allowed range for inputs, the expected input types and ensure parses can be done correctly",
            ),
            (
                StatusCode::UNAUTHORIZED,
                "Returned when the user did not provide any login credentials. Fix: Provide credentials for authentication.",
            ),
            (
                StatusCode::NOT_FOUND,
                "Returned when a switch received an incorrect ID, e.g. an Aruba Switch received a port ID of form accepted by Mellanox. \
                Fix: Check available ports at the switch that is to be configured.",
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Returned when no correct request could be built to send to a switch. Fix: Check the IP provided or update API Version.",
            ),
            (
                StatusCode::BAD_GATEWAY,
                "Returned when a switch sent an unexpected response. \
                Fix: Check if switch is running and switching network is working as intended.",
            ),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Returned when during connection setup a lock on a shared resource was not obtainable. Fix: Try again later.",
            ),
            (
                StatusCode::GATEWAY_TIMEOUT,
                "Returned when the requested switch could not be reached. \
                Fix: Try again later, check if the switch failed or the physical connection is disrupted.",
            ),
        ];
        let res = v
            .iter()
            .map(|(c, d)| {
                (
                    Some(c.as_u16()),
                    aide::openapi::Response {
                        description: d.to_string(),
                        ..Default::default()
                    },
                )
            })
            .collect();
        res
    }
}

impl From<SwitchApiError> for ExtApiError {
    fn from(e: SwitchApiError) -> Self {
        match e {
            // Some switches require authentication to use their REST API for configuration, e.g. enabling/disabling ports
            SwitchApiError::Unauthorized => ExtApiError::new(
                StatusCode::UNAUTHORIZED,
                "No login credentials were provided. Required to access switches.",
            ),

            // Returned when a request to the switch timed out.
            SwitchApiError::DestinationUnreachable => ExtApiError::new(
                StatusCode::REQUEST_TIMEOUT,
                "Could not reach switch. Check if the switch failed or try again later.",
            ),

            // Some bounds checking will be done in FromParam of rocket or by the connection handler.
            // Is returned, when for example a port ID with form destined for a Mellanox switch gets sent to an Aruba switch.
            SwitchApiError::IDInvalid => ExtApiError::new(
                StatusCode::NOT_FOUND,
                "Please check your inputs and try again.",
            ),

            // Handle potential unexpected responses from the switch.
            SwitchApiError::UnexpectedResponseFromSwitch => ExtApiError::new(
                StatusCode::BAD_GATEWAY,
                "Ensure switch and switching network are available.",
            ),

            // Handle potential connection_handler occurring while building the requests to a switches REST API, e.g. can't parse URL used for a Reqwest Client
            SwitchApiError::BuiltFaultyRequestToSwitch => ExtApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Check the type of switch you try to access and it's details or try updating the API version.",
            ),
        }
    }
}

impl From<ConnectionHandlerError> for ExtApiError {
    fn from(e: ConnectionHandlerError) -> Self {
        match e {
            ConnectionHandlerError::SwitchNotFound => {
                ExtApiError::new(StatusCode::NOT_FOUND, "No switch matching description.")
            }
            ConnectionHandlerError::ConnectionSetupFailed(switch_api_error) => {
                switch_api_error.into()
            }
            ConnectionHandlerError::InvalidCacheEntry => ExtApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Switch API caching error",
            ),
            ConnectionHandlerError::EntryGone => ExtApiError::new(
                StatusCode::GONE,
                "There is currently no switch api. Please retry",
            ),
            ConnectionHandlerError::System(e) => {
                ExtApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}"))
            }
        }
    }
}

impl From<IDParseError> for ExtApiError {
    fn from(_: IDParseError) -> Self {
        ExtApiError::new(
            StatusCode::BAD_REQUEST,
            "Expected request body to contain a map with field vlan_id from interval [2..4093].",
        )
    }
}

impl From<SwitchSetupError> for ExtApiError {
    fn from(e: SwitchSetupError) -> Self {
        ExtApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Setup failed: {:?}", e),
        )
    }
}
