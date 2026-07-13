// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::axum::ApiRouter;
use axum::extract::{FromRef, FromRequestParts};

use crate::single_context_api::network_api;

use super::context_management::context_management_api_router;
use super::drives_api::get_drives_router;
use super::machines_api::get_machine_api_router;
use super::{
    ContextManagementApiState, ContextManagerTx, DrivesApiState, GuardedContext, MachineApiState,
    NetworkApiState,
};

/// Create the router for the per context API
pub(crate) fn single_context_router<S>(request_limit: usize, state: S) -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    DrivesApiState: FromRef<S>,
    NetworkApiState: FromRef<S>,
    MachineApiState: FromRef<S>,
    ContextManagementApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
    ContextManagerTx: FromRef<S>,
{
    ApiRouter::new()
        .nest("/", context_management_api_router())
        .nest(
            "/machines",
            get_machine_api_router::<S>(request_limit, state),
        )
        .nest("/networks", network_api::network_api_router())
        .nest("/drives", get_drives_router::<S>())
}
