// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

// You're dunk clippy, go home.
#![allow(dead_code)]

use net_ctrl_lib::get_router;
use net_ctrl_lib::network_type_ids::{
    Credentials, CriticalPorts, PortID, SwitchDetails, SwitchID, VlanID,
};
use net_ctrl_lib::switch::SwitchModel;
use net_ctrl_lib::{InventoryDummyBackend, SwitchMapping, SwitchModelDetail};
use std::net::SocketAddr;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;

// SPDX-SnippetBegin
// SPDX-FileCopyrightText: 2019-2025 axum Contributors
// SPDX-FileCopyrightText: 2026 cyberhar7an <andreas.hartmann@cyberus-technology.de>
// SPDX-License-Identifier: MIT
// SPDX-FileComment: Based on an internal impl by `axum`, first introduced there around rev
//                   axum-v0.5.1. Used here to replace the sketchy axum-test-helper crate (see
//                   <https://github.com/cloudwalk/axum-test-helper>)
pub struct TestClient {
    pub client: reqwest::Client,
    pub addr: SocketAddr,
}

impl TestClient {
    pub async fn new(router: axum::Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let server = axum::serve(listener, router);
            server.await.expect("server error");
        });
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        TestClient { client, addr }
    }

    pub async fn get(&self, url: &str) -> reqwest::Response {
        self.client
            .get(format!("http://{}{url}", self.addr))
            .send()
            .await
            .unwrap()
    }

    pub async fn post_with<T: serde::Serialize>(&self, url: &str, obj: T) -> reqwest::Response {
        self.client
            .post(format!("http://{}{url}", self.addr))
            .body(serde_json::to_vec(&obj).unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap()
    }

    pub async fn put_with<T: serde::Serialize>(&self, url: &str, obj: T) -> reqwest::Response {
        self.client
            .put(format!("http://{}{url}", self.addr))
            .body(serde_json::to_vec(&obj).unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap()
    }

    pub async fn delete(&self, url: &str) -> reqwest::Response {
        self.client
            .delete(format!("http://{}{url}", self.addr))
            .send()
            .await
            .unwrap()
    }
}
// SPDX-SnippetEnd

pub async fn get_test_client_with_ports(
    mgmt_ports: &[&str],
    trunk_ports: &[&str],
) -> (TestClient, Arc<SwitchMapping>) {
    let critical_ports = CriticalPorts {
        mgmt_ports: mgmt_ports
            .iter()
            .map(|port| PortID::new((*port).to_string()))
            .collect(),
        trunk_ports: trunk_ports
            .iter()
            .map(|port| PortID::new((*port).to_string()))
            .collect(),
    };

    let sel123 = SwitchDetails::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        Some(Credentials {
            username: "manager".to_string(),
            password: "123".to_string(),
        }),
        critical_ports.clone(),
        VlanID::new(1).unwrap(),
        VlanID::new(2).unwrap(),
    );

    let sel456 = SwitchDetails::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        Some(Credentials {
            username: "manager".to_string(),
            password: "123".to_string(),
        }),
        critical_ports.clone(),
        VlanID::new(1).unwrap(),
        VlanID::new(2).unwrap(),
    );

    let mapping = Arc::new(SwitchMapping::from_iter(vec![
        (
            SwitchID::from_str("123").unwrap(),
            SwitchModelDetail {
                details: sel123,
                model: SwitchModel::Dummy,
            },
        ),
        (
            SwitchID::from_str("456").unwrap(),
            SwitchModelDetail {
                details: sel456,
                model: SwitchModel::Dummy,
            },
        ),
    ]));

    let ib = InventoryDummyBackend::new(mapping.clone());
    let router = get_router::<()>(ib.into()).await.expect("get_router");
    let test_client = TestClient::new(router).await;

    (test_client, mapping)
}

pub async fn get_default_test_client() -> (TestClient, Arc<SwitchMapping>) {
    get_test_client_with_ports(&["1/1/1"], &["1/1/2"]).await
}
