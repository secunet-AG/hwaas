// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aruba_structs::login_sessions::RestLoginSessions;
use aruba_structs::port::Port;
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) type AppStateStats = DashMap<String, Value>;
pub(crate) type AppStatePorts = DashSet<Port>;
pub(crate) type AppStateLogins = DashMap<RestLoginSessions, Vec<String>>;

#[derive(Serialize, Deserialize)]
pub(crate) struct AppState {
    pub(crate) stats: AppStateStats,
    pub(crate) logins: AppStateLogins,
    pub(crate) ports: AppStatePorts,
    pub(crate) max_sessions: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stats: Default::default(),
            ports: (1..17)
                .map(|i| Port {
                    id: i.to_string(),
                    name: format!("Port-{}", i),
                    ..Default::default()
                })
                .collect(),
            logins: DashMap::from_iter::<Vec<(RestLoginSessions, Vec<String>)>>(vec![(
                RestLoginSessions {
                    user_name: "hwaas".to_string(),
                    password: "hwaas".to_string(),
                },
                vec![],
            )]),
            max_sessions: 5,
        }
    }
}
