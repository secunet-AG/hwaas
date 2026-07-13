// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module provides types for working with the Network API.

mod machine_interface_set;
mod network_setup;
mod patch;
mod tagged_machine_network_interface;
pub use network_setup::*;
pub use patch::*;
pub use tagged_machine_network_interface::TaggedMachineNetworkInterface;
#[cfg(test)]
mod test_fixtures;
