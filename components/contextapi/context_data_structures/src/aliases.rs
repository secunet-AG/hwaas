// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use uuid::Uuid;

/// The user chosen name of a drive.
pub type DriveName = String;

/// User chosen name of a machine, must be unique in the context.
pub type MachineName = String;

/// See [`MachineName`].
pub type MachineNameStr = str;

/// Name of a network, must be unique in the context.
pub type NetworkName = String;

/// See [`NetworkName`].
pub type NetworkNameStr = str;

/// Id of a context.
pub type ContextId = Uuid;

/// Host name of a device in the network. Can hold either an IP address or a DNS name.
pub type HostName = String;

/// Name of a network interface of a machine.
pub type MachineNetworkInterface = String;

/// See [`MachineNetworkInterface`].
pub type MachineNetworkInterfaceStr = str;
