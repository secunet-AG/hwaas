// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! # HWaaS user facing data structures.

#[cfg(feature = "aliases")]
pub mod aliases;
#[cfg(feature = "machine_properties")]
pub mod machine_properties;
#[cfg(feature = "network_setup")]
pub mod network;
#[cfg(feature = "rsd")]
pub mod rsd;
