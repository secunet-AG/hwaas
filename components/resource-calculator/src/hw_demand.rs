// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::config::{HardwareAttributes, HwImpact, SimValues, TestScenario};
use daggy::petgraph::data::DataMap;
use daggy::petgraph::visit::Bfs;
use daggy::{Dag, NodeIndex, Walker};
use error_stack::{Context, Report, ResultExt};
use std::cmp::max_by;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum HwDemandError {
    UnusedHardware(String),
    Cycle,
    InsertedTwice,
}

impl fmt::Display for HwDemandError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HwDemandError::UnusedHardware(node) => {
                fmt.write_fmt(format_args!("Hardware {} is never referenced", node))
            }
            HwDemandError::Cycle => fmt.write_str("Dependencies are not posing a DAG"),
            HwDemandError::InsertedTwice => fmt.write_str("A Hardware was inserted more than once"),
        }
    }
}

// It's also possible to implement `Error` instead.
impl Context for HwDemandError {}

type DemandDag = Dag<f64, f64>;

#[derive(Debug, Clone)]
pub(crate) struct HwDemand {
    /// Maps a hardware to a DAG node
    hw_map: HashMap<String, NodeIndex>,

    /// Maps scenarios to a DAG node
    sc_map: HashMap<String, NodeIndex>,

    /// A DAG posing the Hardware dependencies
    dag: DemandDag,

    /// virtual start node
    start: NodeIndex,
}

impl HwDemand {
    pub fn new(sim: &SimValues) -> Result<Self, Report<HwDemandError>> {
        let mut demands = Self {
            hw_map: Default::default(),
            sc_map: Default::default(),
            dag: Default::default(),
            start: Default::default(),
        };

        demands.parse_dependencies(sim)?;

        Ok(demands)
    }

    pub fn for_each<F, R>(&self, sim: &SimValues, f: F) -> Vec<R>
    where
        F: Fn(&String, &HardwareAttributes, f64) -> Option<R>,
    {
        sim.hardware
            .iter()
            .filter_map(|(name, attr)| {
                let val = self
                    .dag
                    .node_weight(*self.hw_map.get(name).unwrap())
                    .unwrap();
                f(name, attr, *val)
            })
            .collect()
    }

    pub fn for_each_scenario<F, R>(&self, sim: &SimValues, f: F) -> Vec<R>
    where
        F: Fn(&String, f64) -> Option<R>,
    {
        sim.scenarios
            .iter()
            .filter_map(|s| {
                let val = self
                    .dag
                    .node_weight(*self.sc_map.get(s.name.as_str()).unwrap())
                    .unwrap();
                f(&s.name, *val)
            })
            .collect()
    }

    fn parse_dependencies(&mut self, sim: &SimValues) -> Result<(), Report<HwDemandError>> {
        self.start = self.dag.add_node(1.0);

        self.add_hardware_to_dag(sim)?;
        self.add_scenarios_to_dag(sim)?;
        self.parse_impacts(sim)?;

        self.check_nodes()?;

        self.calc_demands()?;

        Ok(())
    }

    fn check_nodes(&self) -> Result<(), HwDemandError> {
        // TODO: Check for nodes without relations

        // only check for HW nodes. Scenario nodes without impacts (relations) are not added.
        for (name, idx) in &self.hw_map {
            let num_neighbours = self
                .dag
                .children(*idx)
                .iter(&self.dag)
                .collect::<Vec<_>>()
                .len()
                + self
                    .dag
                    .parents(*idx)
                    .iter(&self.dag)
                    .collect::<Vec<_>>()
                    .len();

            if num_neighbours == 0 {
                return Err(HwDemandError::UnusedHardware(name.to_string()));
            }
        }
        Ok(())
    }

    fn parse_impacts(&mut self, sim: &SimValues) -> Result<(), Report<HwDemandError>> {
        for s in &sim.scenarios {
            let s_node = *self.sc_map.get(s.name.as_str()).unwrap();
            if !s.hw_impacts.is_empty() {
                self.parse_single_impacts_list(&s.hw_impacts, s_node)?;
            }
        }

        for (hw_name, attr) in &sim.hardware {
            let node_a = *self.hw_map.get(hw_name.as_str()).unwrap();
            self.parse_single_impacts_list(&attr.hw_impacts, node_a)?;
        }

        Ok(())
    }

    fn parse_single_impacts_list(
        &mut self,
        impacts: &Vec<HwImpact>,
        node_a: NodeIndex,
    ) -> Result<(), Report<HwDemandError>> {
        for impact in impacts {
            let node_b = *self.hw_map.get(impact.on_name.as_str()).unwrap();
            println!(
                "{} adds impact of {:.3} to {}",
                self.resolve_node_name(node_a),
                impact.factor,
                self.resolve_node_name(node_b)
            );
            self.dag
                .add_edge(node_a, node_b, impact.factor)
                .change_context(HwDemandError::Cycle)?;
        }
        Ok(())
    }

    fn add_hardware_to_dag(&mut self, sim: &SimValues) -> Result<(), HwDemandError> {
        for hw_name in sim.hardware.keys() {
            if self.hw_map.contains_key(hw_name.as_str()) {
                continue;
            }

            let node = self.dag.add_node(0.0);
            if self.hw_map.insert(hw_name.to_string(), node).is_some() {
                return Err(HwDemandError::InsertedTwice);
            }
        }

        Ok(())
    }

    fn add_scenarios_to_dag(&mut self, sim: &SimValues) -> Result<(), HwDemandError> {
        for s in &sim.scenarios {
            let TestScenario {
                name,
                expected_jobs_per_day,
                avg_job_schedule_time,
                mean_job_exec_time,
                ..
            } = s;

            // Little's law
            // L = lambda * W
            // L := #jobs in steady state (scheduled_jobs)
            // lambda := arrival rate
            // W := time spend in system (service_time)

            let arrival_rate = expected_jobs_per_day / (24 * 60 * 60) as f64;
            let service_time = avg_job_schedule_time + mean_job_exec_time;
            let scheduled_jobs = arrival_rate * service_time;

            if self.sc_map.contains_key(name.as_str()) {
                continue;
            }

            let (_, node) = self.dag.add_child(self.start, 1.0, scheduled_jobs);
            if self.sc_map.insert(name.to_string(), node).is_some() {
                return Err(HwDemandError::InsertedTwice);
            }
        }

        Ok(())
    }

    fn calc_demands(&mut self) -> Result<(), HwDemandError> {
        let mut final_dag = self.dag.clone();

        for s_idx in self.sc_map.values() {
            // make a working copy for traversing the DAG basing on this scenario
            let mut dag_working_copy = self.dag.clone();

            // Traverse and update weights on the working copy
            let mut bfs = Bfs::new(&self.dag, *s_idx);
            while let Some(nx) = bfs.next(&self.dag) {
                self.update_child_node_weights(&mut dag_working_copy, nx)
            }

            // update final
            for (a, b) in final_dag
                .node_weights_mut()
                .zip(dag_working_copy.node_weights_mut())
            {
                *a += *b
            }
        }

        self.dag = final_dag;

        Ok(())
    }

    fn update_child_node_weights(&self, dag: &mut DemandDag, node: NodeIndex) {
        let parent_weight = *dag.node_weight(node).unwrap();
        for (edge_idx, child_idx) in dag.children(node).iter(&dag.clone()) {
            let e_weight = *dag.edge_weight(edge_idx).unwrap();
            let weight_to_add = e_weight * max_by(parent_weight, 1.0, |a, b| a.total_cmp(b));
            assert!(weight_to_add >= 0.0);
            println!(
                "{} adds {:.3} (E:{:.3}, P:{:.3}) to {}",
                self.resolve_node_name(node),
                weight_to_add,
                e_weight,
                parent_weight,
                self.resolve_node_name(child_idx)
            );
            *dag.node_weight_mut(child_idx).unwrap() += weight_to_add;
        }
    }

    fn resolve_node_name(&self, idx: NodeIndex) -> String {
        let fc = |(n, i): (_, &NodeIndex)| {
            if *i == idx {
                Some(n)
            } else {
                None
            }
        };

        let find_in_scenario = self.sc_map.iter().find_map(fc);
        let find_in_hw = self.hw_map.iter().find_map(fc);

        match (find_in_scenario, find_in_hw) {
            (Some(s), None) => {
                format!("SCENARIO:{}", s)
            }
            (None, Some(s)) => {
                format!("HW:{}", s)
            }
            _ => {
                eprintln!("Invalid name resolution for {:?}", idx);
                "INVALID_NOE_NAME".to_string()
            }
        }
    }
}
