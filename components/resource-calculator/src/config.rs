// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct SimValues {
    pub scenarios: Vec<TestScenario>,
    pub hardware: HashMap<String, HardwareAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub avg_job_schedule_time: f64,
    pub expected_jobs_per_day: f64,
    pub mean_job_exec_time: f64,
    pub hw_impacts: Vec<HwImpact>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HardwareAttributes {
    pub price: f64,
    #[serde(default)]
    pub virtual_node: bool,
    pub hw_impacts: Vec<HwImpact>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HwImpact {
    pub on_name: String,
    pub factor: f64,
}

impl Default for SimValues {
    fn default() -> Self {
        let mut hardware = HashMap::new();

        hardware.insert(
            String::from("server"),
            HardwareAttributes {
                price: 10_000.0,
                virtual_node: false,
                hw_impacts: vec![
                    HwImpact {
                        on_name: "raspberry".to_string(),
                        factor: 2.0,
                    },
                    HwImpact {
                        on_name: "switch-test-net".to_string(),
                        factor: 2.0 / 48.0,
                    },
                ],
            },
        );

        hardware.insert(
            String::from("cn"),
            HardwareAttributes {
                price: 30_000.0,
                virtual_node: false,
                hw_impacts: vec![HwImpact {
                    on_name: "switch-mgmt-net".to_string(),
                    factor: 2.0 / 48.0,
                }],
            },
        );

        hardware.insert(
            String::from("switch-test-net"),
            HardwareAttributes {
                price: 3_000.0,
                virtual_node: false,
                hw_impacts: vec![],
            },
        );

        hardware.insert(
            String::from("switch-mgmt-net"),
            HardwareAttributes {
                price: 3_000.0,
                virtual_node: false,
                hw_impacts: vec![],
            },
        );

        hardware.insert(
            String::from("raspberry"),
            HardwareAttributes {
                price: 80.0,
                virtual_node: false,
                hw_impacts: vec![HwImpact {
                    on_name: "switch-mgmt-net".to_string(),
                    factor: 1.0,
                }],
            },
        );

        let hw_impact_l1 = vec![
            HwImpact {
                on_name: "cn".to_string(),
                factor: 0.02,
            }, // A main-actors needs 2% of a CN
            HwImpact {
                on_name: "server".to_string(),
                factor: 1.0,
            }, // L1 test: 1 node
        ];

        let hw_impact_l2 = vec![
            HwImpact {
                on_name: "cn".to_string(),
                factor: 0.05,
            }, // A main-actors needs 5% of a CN
            HwImpact {
                on_name: "server".to_string(),
                factor: 4.0,
            }, // // L2 test: 2+ nodes
        ];

        let scenarios = vec![
            TestScenario {
                name: String::from("basicL1Test"),
                avg_job_schedule_time: 1.0 * 60.0 * 60.0, // jobs scheduled after 1h
                expected_jobs_per_day: 2000.0,
                mean_job_exec_time: 30.0 * 60.0, // job duration mean time 30min
                hw_impacts: hw_impact_l1,
            },
            TestScenario {
                name: String::from("basicL2Test"),
                avg_job_schedule_time: 2.0 * 60.0 * 60.0, // jobs scheduled after 2h
                expected_jobs_per_day: 20.0,
                mean_job_exec_time: 1.0 * 60.0 * 60.0, // job duration mean time 1h
                hw_impacts: hw_impact_l2,
            },
        ];

        Self {
            scenarios,
            hardware,
        }
    }
}
