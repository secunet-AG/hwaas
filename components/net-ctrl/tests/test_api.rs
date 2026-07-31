// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use inventory::{InventoryDummyBackend, SwitchMapping, SwitchModelDetail};
use net_ctrl_lib::{get_router, SetupData};
use network_type_ids::{Credentials, CriticalPorts, PortID, SwitchDetails, SwitchID, VlanID};
use reqwest::StatusCode;
use std::net::SocketAddr;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Arc;
use switch::SwitchModel;
use test_log::test;
use tokio::net::TcpListener;

// SPDX-SnippetBegin
// SPDX-FileCopyrightText: 2019-2025 axum Contributors
// SPDX-FileCopyrightText: 2026 cyberhar7an <andreas.hartmann@cyberus-technology.de>
// SPDX-License-Identifier: MIT
// SPDX-FileComment: Based on an internal impl by `axum`, reexported under sketchy copyright in
//                   axum-test-helper <https://github.com/cloudwalk/axum-test-helper/tree/main>
//                   and adapted from version 0.4.0 for use here by cyberhar7an
pub struct TestClient {
    client: reqwest::Client,
    addr: SocketAddr,
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

async fn get_test_client() -> (TestClient, Arc<SwitchMapping>) {
    let sel123 = SwitchDetails::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        Some(Credentials {
            username: "manager".to_string(),
            password: "123".to_string(),
        }),
        CriticalPorts {
            mgmt_ports: vec![PortID::new("0".to_string())],
            trunk_ports: vec![PortID::new("42".to_string())],
        },
        VlanID::new(1).unwrap(),
        VlanID::new(2).unwrap(),
    );

    let sel456 = SwitchDetails::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        Some(Credentials {
            username: "manager".to_string(),
            password: "123".to_string(),
        }),
        CriticalPorts {
            mgmt_ports: vec![PortID::new("0".to_string())],
            trunk_ports: vec![PortID::new("42".to_string())],
        },
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

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_get_switches() {
    let (client, mapping) = get_test_client().await;
    let res = client.get("/switches").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<SwitchMapping>(&res.bytes().await.unwrap()).unwrap(),
        *mapping
    );

    let res = client.get("/switches/456").await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client.get("/switches/111").await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_setup_switch() {
    let (client, _) = get_test_client().await;
    let res = client
        .post_with(
            "/switches/456/setup",
            SetupData {
                vlan_id_range: 1..10,
            },
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post_with(
            "/switches/456/setup",
            SetupData {
                vlan_id_range: 1..12,
            },
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post_with(
            "/switches/456/setup",
            SetupData {
                vlan_id_range: 0..3,
            },
        )
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = client
        .post_with(
            "/switches/111/setup",
            SetupData {
                vlan_id_range: 1..3,
            },
        )
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_handle_ports() {
    let (client, mapping) = get_test_client().await;

    // test enable
    let res = client
        .put_with("/switches/456/ports/3", VlanID::new(3).unwrap())
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    // idempotent enable
    let res = client
        .put_with("/switches/456/ports/3", VlanID::new(3).unwrap())
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    // test disable
    let res = client.delete("/switches/456/ports/3").await;
    assert_eq!(res.status(), StatusCode::OK);

    // idempotent disable
    let res = client.delete("/switches/456/ports/3").await;
    assert_eq!(res.status(), StatusCode::OK);

    // test enable critical port
    let crit_port = mapping
        .get(&SwitchID::new("456".to_string()))
        .unwrap()
        .details
        .critical_ports
        .trunk_ports
        .first()
        .unwrap();
    let res = client
        .delete(format!("/switches/456/ports/{}", crit_port).as_str())
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
