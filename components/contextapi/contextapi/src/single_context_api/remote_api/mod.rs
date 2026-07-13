// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

mod aux;
mod power;
mod serial;
mod usb;

use std::marker::PhantomData;

use aide::axum::ApiRouter;
use axum::async_trait;
use axum::extract::{DefaultBodyLimit, FromRef, FromRequestParts};
use axum::handler::Handler;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use tracing::error;
use tracing::instrument;

use self::aux::handle_aux_root;
use self::power::handle_power_specialization;
use self::{
    aux::handle_aux_specialization,
    power::handle_power_root,
    serial::{handle_serial_root, handle_serial_specialization},
    usb::{get_usb_router, handle_usb_specialization},
};
use tower_http::limit::RequestBodyLimitLayer;

use super::{ContextManagerTx, DrivesApiState, GuardedContext, MachineApiState};

pub fn get_machine_remote_api_router<S>(request_limit: usize, state: S) -> ApiRouter<S>
where
    S: Send + Sync + Clone + 'static,
    DrivesApiState: FromRef<S>,
    MachineApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
    ContextManagerTx: FromRef<S>,
    PowerEndpointSpecialization: FromRequestParts<S>,
    SerialEndpointSpecialization: FromRequestParts<S>,
{
    ApiRouter::new()
        .route_service(
            "/serial/*rest",
            handle_serial_specialization.with_state(state.clone()),
        )
        .route_service("/serial", handle_serial_root.with_state(state.clone()))
        .route_service(
            "/power/*rest",
            handle_power_specialization.with_state(state.clone()),
        )
        .route_service("/power", handle_power_root.with_state(state.clone()))
        .route_service(
            "/auxiliaries/*rest",
            handle_aux_specialization.with_state(state.clone()),
        )
        .route_service("/auxiliaries", handle_aux_root.with_state(state.clone()))
        .route_service(
            "/usb/*rest",
            handle_usb_specialization.with_state(state.clone()),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(request_limit))
        .merge(get_usb_router())
}

/// Holds everything following "/{peripheral class}/" in an URL that gets handled by the machine_remote_api_router
/// defined in this module.
#[derive(Debug, Clone)]
pub(crate) struct EndPointSpecialization<PeripheralClass> {
    specialization: String,
    _phantom: PhantomData<PeripheralClass>,
}

/// Workaround for Rust's currently very minimal const generics support.
/// We use this to parameterize peripheral class names.
pub(crate) trait PeripheralClassName {
    const PREFIX: &'static str;
}

#[derive(Debug, Clone)]
pub(crate) struct Power;

#[derive(Debug, Clone)]
pub(crate) struct Serial;

#[derive(Debug, Clone)]
pub(crate) struct Usb;

#[derive(Debug, Clone)]
pub(crate) struct Auxiliary;

impl PeripheralClassName for Power {
    const PREFIX: &'static str = "/power/";
}

impl PeripheralClassName for Serial {
    const PREFIX: &'static str = "/serial/";
}

impl PeripheralClassName for Usb {
    const PREFIX: &'static str = "/usb/";
}

impl PeripheralClassName for Auxiliary {
    const PREFIX: &'static str = "/auxiliaries/";
}

/// Provides a string representation of everything following the "/power/" in the uri passed to the
/// router returned by [`get_machine_remote_api_router`].
pub(crate) type PowerEndpointSpecialization = EndPointSpecialization<Power>;

/// Provides a string representation of everything following the "/serial/" in the uri passed to the
/// router returned by [`get_machine_remote_api_router`].
pub(crate) type SerialEndpointSpecialization = EndPointSpecialization<Serial>;

/// Provides a string representation of everything following the "/usb/" in the uri passed to the
/// router returned by [`get_machine_remote_api_router`].
pub(crate) type UsbEndpointSpecialization = EndPointSpecialization<Usb>;

/// Provides a string representation of everything following the "/auxiliaries/" in the uri passed to the
/// router returned by [`get_machine_remote_api_router`].
pub(crate) type AuxiliaryEndpointSpecialization = EndPointSpecialization<Auxiliary>;

#[async_trait]
impl<S, PeripheralClass> FromRequestParts<S> for EndPointSpecialization<PeripheralClass>
where
    S: Send + Sync,
    PeripheralClass: PeripheralClassName,
{
    type Rejection = Response;
    #[instrument(skip_all)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let uri = parts.uri.to_string();
        let Some(specialization) = uri.strip_prefix(PeripheralClass::PREFIX) else {
            error!(expected_prefix = PeripheralClass::PREFIX, extracted_uri = %uri, "BUG: the extracted uri did not start with expected prefix");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Malformed Url").into_response());
        };
        Ok(Self {
            specialization: specialization.to_owned(),
            _phantom: Default::default(),
        })
    }
}
