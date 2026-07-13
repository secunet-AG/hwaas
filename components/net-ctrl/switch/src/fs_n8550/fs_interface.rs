// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use schemars::Map;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct FSN8550InterfacesModule {
    #[serde(rename = "interface:interface")]
    pub(crate) data: FSN8550InterfacesNode,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550InterfacesNode {
    pub(crate) gigabit_ethernet: Vec<FSN8550InterfaceGigabitEthernetContainer>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550InterfaceGigabitEthernetContainer {
    pub(crate) name: String,
    pub(crate) disable: bool,
    pub(crate) family: FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer {
    pub(crate) ethernet_switching: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer {
    pub(crate) native_vlan_id: u16,
    pub(crate) port_mode: PortMode,
    pub(crate) vlan: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer {
    pub(crate) members: Vec<VlanMember>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct VlanMember {
    pub(crate) id: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) untagged: Option<Map<u16, u16>>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PortMode {
    Trunk,
    #[default]
    Access,
}

#[cfg(test)]
mod test {
    use super::{
        FSN8550InterfaceGigabitEthernetContainer,
        FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer,
        FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer,
        FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer,
        FSN8550InterfacesModule, FSN8550InterfacesNode, PortMode, VlanMember,
    };
    use serde_json::json;

    #[test]
    fn test_format() {
        let sample = json!({
            "interface:interface": {
                "gigabit-ethernet": [
                    {
                        "name": "xe-1/1/1",
                        "disable": false,
                        "family": {
                        "ethernet-switching": {
                            "native-vlan-id": 3,
                            "port-mode": "trunk",
                            "vlan": {
                                "members": [
                                    {
                                        "id": 1000
                                    },
                                    {
                                        "id": 3,
                                        "untagged": {}
                                    }
                                ]
                            }
                        }
                    }
                    }
                ]
            }
        });

        let to_test = FSN8550InterfacesModule {
            data: FSN8550InterfacesNode { gigabit_ethernet: vec![
                FSN8550InterfaceGigabitEthernetContainer {
                    name: "xe-1/1/1".to_string(),
                    disable: false,
                    family: FSN8550InterfaceGigabitEthernetFamilyEthernetFamilyContainer {
                        ethernet_switching: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingContainer {
                            native_vlan_id: 3,
                            port_mode: PortMode::Trunk,
                            vlan: FSN8550InterfaceGigabitEthernetFamilyEthernetSwitchingVlanContainer {
                                members: vec![
                                    VlanMember {
                                        id: 1000,
                                        untagged: None,
                                    },
                                    VlanMember {
                                        id: 3,
                                        untagged: Some(Default::default()),
                                    }
                                ],
                            },
                        }
                    },
                }
            ] },
        };

        assert_eq!(sample, serde_json::to_value(to_test).unwrap());
    }
}
