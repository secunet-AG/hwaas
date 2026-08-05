// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::NetCtrlClient;
use crate::app_config::ContextApiConfig;
use crate::context_manager::ContextManagerMessage;
use crate::context_reservation::ContextCreationApiState;
use crate::inventory::InventoryApiState;
use crate::remote_client::RemoteClient;
use crate::single_context_api::{
    ContextManagementApiState, DrivesApiState, MachineApiState, NetworkApiState,
};
use axum::extract::FromRef;
use db_interaction::connection::DbFacade;
use image_api::{ImageHandler, IntoImageHandler};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct AppState {
    net_api_state: NetworkApiState,
    machines_api_state: MachineApiState,
    context_managament_api_state: ContextManagementApiState,
    drives_api_state: DrivesApiState,
    context_creation_api_state: ContextCreationApiState,
    inventory_api_state: InventoryApiState,
    pub(crate) context_manager_tx: Sender<ContextManagerMessage>,
}

impl AppState {
    pub(crate) fn new(
        config: Arc<ContextApiConfig>,
        net_ctrl: NetCtrlClient,
        remote_client: RemoteClient,
        image_handler: ImageHandler,
        db_facade: Arc<DbFacade>,
        context_manager_tx: Sender<ContextManagerMessage>,
    ) -> Self {
        let net_api_state = NetworkApiState::new(db_facade.clone(), net_ctrl, config.clone());
        let machines_api_state = MachineApiState::new(db_facade.clone(), remote_client);
        let drives_api_state = DrivesApiState::new(db_facade.clone(), image_handler);
        let context_managament_api_state = ContextManagementApiState::new(
            config.clone(),
            db_facade.clone(),
            context_manager_tx.clone(),
        );
        let inventory_api_state = InventoryApiState::new(db_facade.clone());
        let context_creation_api_state = ContextCreationApiState::new(
            db_facade.clone(),
            config.clone(),
            context_manager_tx.clone(),
        );
        Self {
            net_api_state,
            machines_api_state,
            context_managament_api_state,
            drives_api_state,
            context_creation_api_state,
            inventory_api_state,
            context_manager_tx,
        }
    }
}

impl FromRef<AppState> for NetworkApiState {
    fn from_ref(input: &AppState) -> Self {
        input.net_api_state.clone()
    }
}

impl FromRef<AppState> for MachineApiState {
    fn from_ref(input: &AppState) -> Self {
        input.machines_api_state.clone()
    }
}

impl FromRef<AppState> for ContextCreationApiState {
    fn from_ref(input: &AppState) -> Self {
        input.context_creation_api_state.clone()
    }
}

impl FromRef<AppState> for DrivesApiState {
    fn from_ref(input: &AppState) -> Self {
        input.drives_api_state.clone()
    }
}

impl FromRef<AppState> for ContextManagementApiState {
    fn from_ref(input: &AppState) -> Self {
        input.context_managament_api_state.clone()
    }
}

impl FromRef<AppState> for InventoryApiState {
    fn from_ref(input: &AppState) -> Self {
        input.inventory_api_state.clone()
    }
}

impl IntoImageHandler for AppState {
    fn get_image_handler(&self) -> ImageHandler {
        self.drives_api_state.get_image_handler()
    }
}
/// Trivial state that is only used when one wants to generate the Open API specification.
#[derive(Clone)]
pub(crate) struct UnImplementedState {}

macro_rules! unimplemented_from_ref {
    ($($substate:ty),*) => {
        $(impl FromRef<UnImplementedState> for $substate {
            fn from_ref(_input: &UnImplementedState) -> Self {
                unimplemented!(
                    "UnImplementedState is not meant for serving requests. Use AppState instead!"
                )
            }

        }
        )*
    }
}

unimplemented_from_ref! {
    NetworkApiState,
    MachineApiState,
    ContextCreationApiState,
    DrivesApiState,
    ContextManagementApiState,
    InventoryApiState
}

impl IntoImageHandler for UnImplementedState {
    fn get_image_handler(&self) -> ImageHandler {
        unimplemented!(
            "UnImplemented state is not meant for serving requests. Use AppState instead!"
        )
    }
}
