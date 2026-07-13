// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::aliases::{MachineName, MachineNetworkInterface};

/// ID of a machine together with the name of one of its network interfaces.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TaggedMachineNetworkInterface {
    pub machine_name: MachineName,
    pub interface: MachineNetworkInterface,
}
