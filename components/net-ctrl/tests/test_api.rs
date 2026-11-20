// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;
use common::{TestClient, get_default_test_client, get_test_client_with_ports};

use net_ctrl_lib::SetupData;
use net_ctrl_lib::SwitchMapping;
use net_ctrl_lib::network_type_ids::{SwitchID, VlanID};
use reqwest::StatusCode;
use test_log::test;

#[test(tokio::test(flavor = "multi_thread"))]
async fn test_get_switches() {
    let (client, mapping) = get_default_test_client().await;
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
    let (client, _) = get_default_test_client().await;
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

async fn test_port_type(port: &str, mgmt_ports: &[&str], trunk_ports: &[&str]) {
    let (client, mapping) = get_test_client_with_ports(mgmt_ports, trunk_ports).await;

    let path = format!("/switches/456/ports/{port}");

    // test enable
    let res = client.put_with(&path, VlanID::new(3).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK, "failed to enable port {port}");

    // idempotent enable
    let res = client.put_with(&path, VlanID::new(3).unwrap()).await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "failed idempotent enable for port {port}"
    );

    // test disable
    let res = client.delete(&path).await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "failed to disable port {port}"
    );

    // idempotent disable
    let res = client.delete(&path).await;
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
