// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum_test_helper::TestClient;
use net_ctrl_lib::network_type_ids::{
    Credentials, CriticalPorts, PortID, SwitchDetails, SwitchID, VlanID,
};
use net_ctrl_lib::switch::SwitchModel;
use net_ctrl_lib::{InventoryDummyBackend, SwitchMapping, SwitchModelDetail};
use net_ctrl_lib::{SetupData, get_router};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Arc;
use test_log::test;

async fn get_test_client_with_ports(
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
    (TestClient::new(router).await, mapping)
}

async fn get_default_test_client() -> (TestClient, Arc<SwitchMapping>) {
    get_test_client_with_ports(&["1/1/1"], &["1/1/2"]).await
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_get_switches() {
    let (client, mapping) = get_default_test_client().await;
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
    let (client, _) = get_default_test_client().await;
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

async fn test_port_type(port: &str, mgmt_ports: &[&str], trunk_ports: &[&str]) {
    let (client, mapping) = get_test_client_with_ports(mgmt_ports, trunk_ports).await;

    let path = format!("/switches/456/ports/{port}");

    // test enable
    let res = client
        .put(&path)
        .json(&VlanID::new(3).unwrap())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK, "failed to enable port {port}");

    // idempotent enable
    let res = client
        .put(&path)
        .json(&VlanID::new(3).unwrap())
        .send()
        .await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "failed idempotent enable for port {port}"
    );

    // test disable
    let res = client.delete(&path).send().await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "failed to disable port {port}"
    );

    // idempotent disable
    let res = client.delete(&path).send().await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "failed idempotent disable for port {port}"
    );

    // test enable critical port
    let critical_port = mapping
        .get(&SwitchID::new("456".to_string()))
        .unwrap()
        .details
        .critical_ports
        .trunk_ports
        .first()
        .unwrap();
    let res = client
        .delete(format!("/switches/456/ports/{critical_port}").as_str())
        .send()
        .await;
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "critical port {critical_port} was not protected"
    );
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_handle_ports_slash_separated() {
    test_port_type("1/1/3", &["1/1/1"], &["1/1/2"]).await;
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_handle_ports_letter_number() {
    test_port_type("A5", &[], &["A1", "A2", "A3", "A4"]).await;
}

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_handle_ports_numeric() {
    test_port_type("10", &["1", "2"], &["48"]).await;
}
