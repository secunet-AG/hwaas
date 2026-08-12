// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct FSN8550VlansModule {
    #[serde(rename = "vlans:vlans")]
    pub(crate) data: FSN8550VlansContainer,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550VlansContainer {
    pub(crate) vlan_id: Vec<FSN8550VlansVlanIdContainer>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FSN8550VlansVlanIdContainer {
    pub(crate) id: u16,
    pub(crate) vlan_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) l3_interface: Option<String>,
}

#[cfg(test)]
mod test {
    use crate::switch::fs_n8550::fs_vlans::{
        FSN8550VlansContainer, FSN8550VlansModule, FSN8550VlansVlanIdContainer,
    };
    use serde_json::json;

    #[test]
    fn test_format() {
        let sample = json!({
            "vlans:vlans": {
                "vlan-id": [
                    {
                        "id": 42,
                        "vlan-name": "vlan42",
                        "l3-interface": "vlanIface42"
                    },
                    {
                        "id": 3,
                        "vlan-name": "vlan3",
                    }
                ]
            }
        });

        let to_test = FSN8550VlansModule {
            data: FSN8550VlansContainer {
                vlan_id: vec![
                    FSN8550VlansVlanIdContainer {
                        id: 42,
                        vlan_name: "vlan42".to_string(),
                        l3_interface: Some("vlanIface42".to_string()),
                    },
                    FSN8550VlansVlanIdContainer {
                        id: 3,
                        vlan_name: "vlan3".to_string(),
                        l3_interface: None,
                    },
                ],
            },
        };

        assert_eq!(sample, serde_json::to_value(to_test).unwrap());
    }
}
