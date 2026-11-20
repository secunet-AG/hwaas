// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde_json::json;

pub fn context_api_config(db_file_path: &str, net_ctrl_base_path: &str) -> serde_json::Value {
    json!(
      {
        "db_file_path": db_file_path,
        "net_ctrl_base_path": net_ctrl_base_path,
        "image_api_settings": {
          "max_file_size": "128MiB",
          "store": "/tmp/unused"
        },
        "network_gateway": {
          "ws_gateway_url": "http://example.com/unused"
        },
        "request_timeouts": {
          "single_context_api": 42,
          "context_management_api": 42
        },
        "max_db_connections": 42,
        "context_lifetime": 42,
      }
    )
}
