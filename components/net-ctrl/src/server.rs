// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use connection_handler::ConnectionHandler;
use external::api::{disable_port, enable_port, get_switch_info, get_switches, setup_switch};
use external::api::{
    okapi_add_operation_for_disable_port_, okapi_add_operation_for_enable_port_,
    okapi_add_operation_for_get_switch_info_, okapi_add_operation_for_get_switches_,
    okapi_add_operation_for_setup_switch_,
};
use external::{ExtApiError, ExtApiErrorContent};
use inventory::{InventoryBackend, InventoryConnector};

use crate::ConnectionHandlerShutdownFairing;

static VLAN_ID_ERROR_MSG: &str =
    "Expected request body to contain a map with field vlan_id from interval [2..4093].";

fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "/openapi.json".to_string(),
        ..Default::default()
    }
}

#[catch(422)]
async fn parse_error(_req: &Request<'_>) -> ExtApiError {
    ExtApiError::BadRequest(Json(ExtApiErrorContent {
        detail: VLAN_ID_ERROR_MSG.to_string(),
    }))
}

#[catch(400)]
async fn parse_bad_request(_req: &Request<'_>) -> ExtApiError {
    ExtApiError::BadRequest(Json(ExtApiErrorContent {
        detail: VLAN_ID_ERROR_MSG.to_string(),
    }))
}

#[cfg(test)]
mod golden_test {
    use pretty_assertions::assert_eq;

    use crate::openapi;

    #[test]
    fn golden_test() {
        /* Note:
            The "better" way would be to deserialize ref_data to type OpenApi.
            This would allow us to ignore the exact formatting of the file and compare the
            OpenApi's only by content.
            However currently the deserialization is not working as expected
            due to an error with flattening some 'extensions' fields in structs of okapi::openapi3.
        */
        let ref_data = std::fs::read_to_string("../tests/expected/openapi.json").unwrap();
        let current_str = openapi::gen_openapi_json();
        assert_eq!(ref_data, current_str);
    }
}
