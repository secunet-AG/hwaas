// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

pub fn load_config<T>(config_path: &str) -> T
where
    T: for<'a> Deserialize<'a>,
{
    let config_file = match std::fs::read_to_string(config_path) {
        Ok(config_file) => config_file,
        Err(e) => panic!("Unable to read the config file {config_path}: {e:?}"),
    };

    serde_json::from_str::<T>(&config_file)
        .expect("Provided config file was not the expected JSON format.")
}
