// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod port_id;
mod port_representation;
mod switch_details;
mod switch_id;
mod vlan_id;

pub use port_id::PortID;
pub use port_representation::PortRepresentation;
pub use switch_details::{Credentials, CriticalPorts, SwitchDetails};
pub use switch_id::SwitchID;
pub use vlan_id::{IDParseError, VlanID};
