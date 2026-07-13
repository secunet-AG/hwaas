// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::config::SimValues;
use crate::hw_demand::HwDemand;
use cli_table::{format::Justify, print_stdout, Table, WithTitle};
use error_stack::{Context, Report, ResultExt};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::fs::File;
use std::ops::Add;
use std::path::PathBuf;

#[derive(Debug)]
pub struct TableError;
impl fmt::Display for TableError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("App error")
    }
}

impl Context for TableError {}

#[derive(Table, Clone, Serialize, Deserialize)]
struct HwDemandTableItem {
    #[table(title = "HW", justify = "Justify::Right")]
    name: String,
    #[table(title = "Amount", justify = "Justify::Center")]
    number: f64,
    #[table(
        title = "price (unit)",
        justify = "Justify::Right",
        display_fn = "table_float_display"
    )]
    price_one: f64,
    #[table(
        title = "price (sum)",
        justify = "Justify::Right",
        display_fn = "table_float_display"
    )]
    price_sum: f64,
}

#[derive(Table, Clone, Serialize, Deserialize)]
struct ScenarioTableItem {
    #[table(title = "HW", justify = "Justify::Right")]
    name: String,
    #[table(title = "Weight", justify = "Justify::Center")]
    weight: f64,
}

fn table_float_display(val: &f64) -> String {
    format!("{:.2} €", val)
}

impl Eq for HwDemandTableItem {}

impl PartialEq<Self> for HwDemandTableItem {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl PartialOrd<Self> for HwDemandTableItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HwDemandTableItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Add for HwDemandTableItem {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self::Output {
        self.number += rhs.number;
        self.price_sum += rhs.price_sum;
        self
    }
}

pub(crate) struct DemandTable {
    demand_table: Vec<HwDemandTableItem>,
    scenario_table: Vec<ScenarioTableItem>,
}

impl DemandTable {
    pub(crate) fn new(values: &SimValues, demand: &HwDemand) -> Self {
        Self {
            demand_table: Self::build_demand_table(values, demand),
            scenario_table: Self::build_scenarios_table(values, demand),
        }
    }

    fn build_scenarios_table(values: &SimValues, demand: &HwDemand) -> Vec<ScenarioTableItem> {
        demand.for_each_scenario(values, |name, weight| {
            Some(ScenarioTableItem {
                name: name.to_string(),
                weight,
            })
        })
    }

    fn build_demand_table(values: &SimValues, demand: &HwDemand) -> Vec<HwDemandTableItem> {
        let mut hw_demand_table = demand.for_each(values, |name, attr, val| {
            let number = if val > 0.0 {
                val.floor() + 1.0
            } else {
                eprintln!("Skipping entry {} because amount is 0 ", name);
                0.0
            };

            if !attr.virtual_node && number > 0.0 {
                Some(HwDemandTableItem {
                    name: name.to_string(),
                    number,
                    price_one: attr.price,
                    price_sum: attr.price * number,
                })
            } else {
                None
            }
        });
        hw_demand_table.sort();

        let sum_row = hw_demand_table.iter().fold(
            HwDemandTableItem {
                name: "Sum".to_string(),
                number: 0.0,
                price_one: 0.0,
                price_sum: 0.0,
            },
            |e, a| e.add(a.clone()),
        );

        hw_demand_table.push(sum_row);
        hw_demand_table
    }

    pub fn print(&self) -> Result<(), Report<TableError>> {
        print_stdout(self.scenario_table.with_title()).change_context(TableError)?;

        print_stdout(self.demand_table.with_title()).change_context(TableError)
    }

    pub fn export_csv(&self, path: PathBuf) -> Result<(), Report<TableError>> {
        let w = File::options()
            .create(true)
            .truncate(true)
            .append(false)
            .write(true)
            .read(false)
            .open(path)
            .change_context(TableError)?;
        let mut wtr = csv::Writer::from_writer(w);

        for e in &self.demand_table {
            wtr.serialize(e).change_context(TableError)?;
        }

        Ok(())
    }
}
