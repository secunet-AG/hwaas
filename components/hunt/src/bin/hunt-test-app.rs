// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use hunt::HuntBuilder;
use tracing::info;

fn main() {
    let _ = HuntBuilder::new()
        .verbosity(2)
        .append_filters(vec!["hunt_test_app::enabled_log"])
        .fallback_name(env!("CARGO_PKG_NAME"))
        .fallback_version(env!("CARGO_PKG_VERSION"))
        .build();

    info!("starting hunt test app");
    enabled_log::do_log();
    not_enabled_log::do_log();
}

mod enabled_log {
    use tracing::info;

    pub(crate) fn do_log() {
        info!("Hello World")
    }
}

mod not_enabled_log {
    use tracing::info;

    pub(crate) fn do_log() {
        info!("foo bar")
    }
}
