// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use image_api::ImageApiSettings;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

/// ContextAPI config file format
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextApiConfig {
    /// the base url for the NetCtrl
    pub net_ctrl_base_path: String,

    /// configuration for the image api
    pub image_api_settings: ImageApiSettings,

    /// settings for websocket-network connections
    pub network_gateway: WsGatewaySettings,

    /// Path to the openAPI specification of the remote-hands
    #[serde(default)]
    pub remote_oas_paths: Vec<String>,

    /// Max request size to forward to remote-hands.
    /// The request is fully recieved befor frowarding it.
    #[serde(default)]
    pub remote_max_request_size: MaxRequestSizeSetting,

    /// Request timeout for sub-APIs.
    /// For usage see [`ContextApiRequestTimeoutConfig`].
    /// If this field is not specified all default timeouts are used.
    #[serde(default)]
    pub request_timeouts: ContextApiRequestTimeoutConfig,

    /// The default lifetime for a context
    #[serde(default)]
    pub context_lifetime: ContextLifetimeSetting,

    /// The maximum lifetime for a context. Users cannot extend beyond
    /// this time range.
    #[serde(default)]
    pub context_max_lifetime: ContextMaxLifetimeSetting,

    /// The url of the database
    pub db_file_path: String,

    #[serde(default)]
    pub max_db_connections: MaxDatabaseConnections,
}

/// The maximum number of database connections.
/// By default this will be a quarter of the available parallelism.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct MaxDatabaseConnections(pub u32);

impl Default for MaxDatabaseConnections {
    fn default() -> Self {
        let available_parallelism: u32 = std::thread::available_parallelism()
            .map(usize::from)
            .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
            .unwrap_or(1);
        Self(std::cmp::max(available_parallelism / 4, 1))
    }
}

/// Setting for the maximum allowed request size
#[derive(Debug, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct MaxRequestSizeSetting(usize);

impl Default for MaxRequestSizeSetting {
    fn default() -> Self {
        Self(64 * 1024 * 1024)
    }
}

impl From<MaxRequestSizeSetting> for usize {
    fn from(val: MaxRequestSizeSetting) -> Self {
        val.0
    }
}

/// ContextAPI timeout Config
///
/// Usage:
/// - don't specify to use default timeouts
/// - specify one or more to overwrite the given timeout(s) (int; in milliseconds)
/// - disable a single or all timeouts by setting it to value `null`
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[serde(default)]
pub struct ContextApiRequestTimeoutConfig {
    /// timeout in milliseconds for the ImageAPI
    pub image_api: Option<u64>,

    /// timeout in milliseconds for the SingleContextAPI
    pub single_context_api: Option<u64>,

    /// timeout in milliseconds for the ContextManagementAPI
    pub context_management_api: Option<u64>,
}

impl Default for ContextApiRequestTimeoutConfig {
    fn default() -> Self {
        Self {
            image_api: Some(120_000),
            single_context_api: Some(60_000),
            context_management_api: Some(120_000),
        }
    }
}

/// Settings for handling websocket connections providing access to user networks
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct WsGatewaySettings {
    /// Base url for the ws-gateway
    pub ws_gateway_url: String,
}

/// Duration in seconds for restricting context lifetime
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct ContextLifetimeSetting(u64);

impl Default for ContextLifetimeSetting {
    fn default() -> Self {
        ContextLifetimeSetting(3_600)
    }
}

impl From<ContextLifetimeSetting> for Duration {
    fn from(context_lifetime: ContextLifetimeSetting) -> Duration {
        Duration::from_secs(context_lifetime.0)
    }
}

/// Duration in seconds for restricting context lifetime
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct ContextMaxLifetimeSetting(u64);

impl Default for ContextMaxLifetimeSetting {
    fn default() -> Self {
        ContextMaxLifetimeSetting(7_200)
    }
}

impl From<ContextMaxLifetimeSetting> for Duration {
    fn from(context_max_lifetime: ContextMaxLifetimeSetting) -> Duration {
        Duration::from_secs(context_max_lifetime.0)
    }
}

#[cfg(test)]
mod test {
    use crate::app_config::ContextApiRequestTimeoutConfig;
    use crate::ContextApiConfig;
    use serde_json::json;

    #[test]
    fn timeout_emtpy_to_default() {
        let cfg: ContextApiConfig = serde_json::from_value(json!({
            "db_file_path": "test.db",
            "net_ctrl_base_path": "http://localhost:${net_ctrl_port}/",
            "image_api_settings": {
              "store": "/",
              "max_file_size": "128MiB"
            },
            "network_gateway": {
                "ws_gateway_url": "ws://127.0.0.1:1234",
            }
        }))
        .unwrap();

        assert_eq!(
            cfg.request_timeouts,
            ContextApiRequestTimeoutConfig::default()
        );
    }

    #[test]
    fn timeout_missing_to_default() {
        let cfg: ContextApiConfig = serde_json::from_value(json!({
            "db_file_path": "test.db",
            "net_ctrl_base_path": "http://localhost:${net_ctrl_port}/",
            "image_api_settings": {
              "store": "/",
              "max_file_size": "128MiB"
            },
            "request_timeouts": {
                "image_api": 12000
            },
            "network_gateway": {
                "ws_gateway_url": "ws://127.0.0.1:1234",
            }
        }))
        .unwrap();

        assert_eq!(
            cfg.request_timeouts,
            ContextApiRequestTimeoutConfig {
                image_api: Some(12000),
                ..Default::default()
            }
        );
    }

    #[test]
    fn timeout_disable() {
        let cfg: ContextApiConfig = serde_json::from_value(json!({
            "db_file_path": "test.db",
            "net_ctrl_base_path": "http://localhost:${net_ctrl_port}/",
            "image_api_settings": {
              "store": "/",
              "max_file_size": "128MiB"
            },
            "request_timeouts": {
                "single_context_api": null
            },
            "network_gateway": {
                "ws_gateway_url": "ws://127.0.0.1:1234",
            }
        }))
        .unwrap();

        assert_eq!(
            cfg.request_timeouts,
            ContextApiRequestTimeoutConfig {
                single_context_api: None,
                ..Default::default()
            }
        );
    }
}
