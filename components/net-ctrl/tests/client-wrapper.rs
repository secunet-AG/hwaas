// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod common;
use common::get_default_test_client;
use db_interaction::models::machines::SwitchPort;

#[tokio::test]
async fn enable_ports() {
    let (client, _mapping) = get_default_test_client().await;

    let wrapper = net_ctrl_client_wrapper::NetCtrlClient::new(format!(
        "http://{}:{}",
        client.addr.ip(),
        client.addr.port()
    ));

    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/3".to_string(),
            },
            3,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/4".to_string(),
            },
            3,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/3".to_string(),
            },
            4,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/4".to_string(),
            },
            4,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "456".to_string(),
                port: "1/1/3".to_string(),
            },
            3,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "456".to_string(),
                port: "1/1/4".to_string(),
            },
            3,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn protected_ports() {
    let (client, _mapping) = get_default_test_client().await;

    let wrapper = net_ctrl_client_wrapper::NetCtrlClient::new(format!(
        "http://{}:{}",
        client.addr.ip(),
        client.addr.port()
    ));

    // Cannot access trunk port without authorization
    let response = wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/1".to_string(),
            },
            3,
        )
        .await;
    assert!(response.is_err());
}

#[tokio::test]
async fn disable_ports() {
    let (client, _mapping) = get_default_test_client().await;

    let wrapper = net_ctrl_client_wrapper::NetCtrlClient::new(format!(
        "http://{}:{}",
        client.addr.ip(),
        client.addr.port()
    ));

    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/3".to_string(),
            },
            3,
        )
        .await
        .unwrap();
    wrapper
        .enable_port(
            &SwitchPort {
                id: 0,
                switch: "123".to_string(),
                port: "1/1/4".to_string(),
            },
            3,
        )
        .await
        .unwrap();
    wrapper
        .disable_port(&SwitchPort {
            id: 0,
            switch: "123".to_string(),
            port: "1/1/3".to_string(),
        })
        .await
        .unwrap();
    wrapper
        .disable_port(&SwitchPort {
            id: 0,
            switch: "123".to_string(),
            port: "1/1/4".to_string(),
        })
        .await
        .unwrap();
}
