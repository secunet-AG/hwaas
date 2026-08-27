// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod api;
mod aruba;
mod aruba_aos_cx;
mod dummy;
mod fs_n8550;
mod switch_api_errors;
mod switch_setup_error;

pub use api::{SwitchAPI, SwitchBackend, SwitchModel};
pub use dummy::dummy_test_switch::DummyTestSwitch;
pub use switch_api_errors::SwitchApiError;
pub use switch_setup_error::SwitchSetupError;
