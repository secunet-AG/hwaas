// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

// @generated automatically by Diesel CLI.

diesel::table! {
    context_lifetimes (context_id) {
        context_id -> Binary,
        created -> Timestamp,
        timeout -> Timestamp,
    }
}

diesel::table! {
    contexts (id) {
        id -> Binary,
    }
}

diesel::table! {
    drives (id) {
        id -> Text,
        name -> Text,
        context_id -> Binary,
    }
}

diesel::table! {
    enabled_ports (id) {
        id -> Integer,
        net_id -> SmallInt,
    }
}

diesel::table! {
    machine_reservations (id) {
        id -> Integer,
        context_id -> Binary,
        machine_name -> Text,
    }
}

diesel::table! {
    machine_resets (id) {
        id -> Integer,
        started -> Timestamp,
    }
}

diesel::table! {
    machines (id) {
        id -> Integer,
        platform -> Text,
        remote_usb -> Text,
        remote_power -> Text,
        remote_serial -> Nullable<Text>,
        remote_auxiliary -> Nullable<Text>,
        state -> SmallInt,
        remote_mjpeg -> Nullable<Text>,
    }
}

diesel::table! {
    network_identifiers (id) {
        id -> SmallInt,
    }
}

diesel::table! {
    networks (id) {
        id -> SmallInt,
        context_id -> Binary,
        name -> Text,
    }
}

diesel::table! {
    switch_connections (id) {
        id -> Integer,
        interface -> Text,
        machine_id -> Integer,
    }
}

diesel::table! {
    switch_ports (id) {
        id -> Integer,
        switch -> Text,
        port -> Text,
    }
}

diesel::joinable!(context_lifetimes -> contexts (context_id));
diesel::joinable!(drives -> contexts (context_id));
diesel::joinable!(enabled_ports -> networks (net_id));
diesel::joinable!(enabled_ports -> switch_ports (id));
diesel::joinable!(machine_reservations -> contexts (context_id));
diesel::joinable!(machine_reservations -> machines (id));
diesel::joinable!(machine_resets -> machines (id));
diesel::joinable!(networks -> contexts (context_id));
diesel::joinable!(networks -> network_identifiers (id));
diesel::joinable!(switch_connections -> machines (machine_id));
diesel::joinable!(switch_connections -> switch_ports (id));

diesel::allow_tables_to_appear_in_same_query!(
    context_lifetimes,
    contexts,
    drives,
    enabled_ports,
    machine_reservations,
    machine_resets,
    machines,
    network_identifiers,
    networks,
    switch_connections,
    switch_ports,
);
