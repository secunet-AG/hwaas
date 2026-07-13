// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum_test_helper::TestClient;
use inventory::{InventoryDummyBackend, SwitchMapping, SwitchModelDetail};
use net_ctrl_lib::{get_router, SetupData};
use network_type_ids::{Credentials, CriticalPorts, PortID, SwitchDetails, SwitchID, VlanID};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Arc;
use switch::SwitchModel;
use test_log::test;

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
    (TestClient::new(router).await, mapping)
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_get_switches() {
    let (client, mapping) = get_test_client().await;
    let res = client.get("/switches").send().await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.json::<SwitchMapping>().await, *mapping);

    let res = client.get("/switches/456").send().await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client.get("/switches/111").send().await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_setup_switch() {
    let (client, _) = get_test_client().await;
    let res = client
        .post("/switches/456/setup")
        .json(&SetupData {
            vlan_id_range: 1..10,
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post("/switches/456/setup")
        .json(&SetupData {
            vlan_id_range: 1..12,
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .post("/switches/456/setup")
        .json(&SetupData {
            vlan_id_range: 0..3,
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = client
        .post("/switches/111/setup")
        .json(&SetupData {
            vlan_id_range: 1..3,
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_handle_ports() {
    let (client, mapping) = get_test_client().await;

    // test enable
    let res = client
        .put("/switches/456/ports/3")
        .json(&VlanID::new(3).unwrap())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    // idempotent enable
    let res = client
        .put("/switches/456/ports/3")
        .json(&VlanID::new(3).unwrap())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    // test disable
    let res = client.delete("/switches/456/ports/3").send().await;
    assert_eq!(res.status(), StatusCode::OK);

    // idempotent disable
    let res = client.delete("/switches/456/ports/3").send().await;
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
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
