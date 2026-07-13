// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::axum::routing::get;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::{Info, OpenApi};
use axum::error_handling::HandleErrorLayer;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::{Method, StatusCode, Uri};
use axum::{Extension, Json, Router};
use db_interaction::connection::DbFacade;
use error_stack::ResultExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::BoxError;
use tracing::{debug, warn};

use crate::api_merge_remote::merge_remote_oas;
use crate::app_state::{AppState, UnImplementedState};
use crate::context_manager::{self, ContextManagerCommunication};
use crate::context_reservation::{context_creation_api_router, ContextCreationApiState};
use crate::inventory::InventoryApiState;
use crate::remote_client::RemoteClient;
use crate::single_context_api::{
    ContextManagementApiState, ContextManagerTx, DrivesApiState, GuardedContext, MachineApiState,
    NetworkApiState,
};
use crate::{inventory, single_context_api, ContextApiConfig, NetCtrlClient, API_VERSION};
use image_api::{get_image_api_router, ImageHandler, IntoImageHandler};

pub fn get_api(app_conf: ContextApiConfig) -> OpenApi {
    router_with_api(UnImplementedState {}, &app_conf).1
}

fn router_with_api<S>(app_state: S, config: &ContextApiConfig) -> (Router<()>, OpenApi)
where
    S: IntoImageHandler + Send + Sync + Clone + 'static,
    DrivesApiState: FromRef<S>,
    NetworkApiState: FromRef<S>,
    MachineApiState: FromRef<S>,
    ContextManagementApiState: FromRef<S>,
    ContextCreationApiState: FromRef<S>,
    InventoryApiState: FromRef<S>,
    GuardedContext: FromRequestParts<S>,
    ContextManagerTx: FromRef<S>,
{
    let image_api_max_file_size = config.image_api_settings.max_file_size;
    let optional_timeout = |option_timeout: Option<u64>| {
        ServiceBuilder::new()
            // `timeout` will produce an error if the handler takes
            // too long, so we must handle those
            .layer(HandleErrorLayer::new(handle_error))
            .option_layer(
                option_timeout
                    .map(Duration::from_millis)
                    .map(|dur| TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, dur)),
            )
    };

    let single_context_router = single_context_api::get_router::single_context_router::<S>(
        config.remote_max_request_size.into(),
        app_state.clone(),
    )
    .layer(optional_timeout(config.request_timeouts.single_context_api));

    let images_api_router = get_image_api_router::<S>(image_api_max_file_size)
        .layer(optional_timeout(config.request_timeouts.image_api));

    let mut api = OpenApi {
        info: Info {
            title: "HWaaS Context API".to_string(),
            description: Some(
                "The REST-API to access the Hardware as a Service implementation.".to_string(),
            ),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Info::default()
        },
        openapi: "3.1.0".into(),
        ..Default::default()
    };

    let context_management_router = context_creation_api_router().layer(optional_timeout(
        config.request_timeouts.context_management_api,
    ));

    let api_router = ApiRouter::new()
        .nest("/contexts", context_management_router)
        .nest("/contexts/:ctx_id", single_context_router)
        .nest("/inventory", inventory::inventory_api_router::<S>())
        .nest("/images", images_api_router)
        .route("/openapi.json", get(serve_api))
        .fallback(fallback_handler)
        .layer(Extension(api.clone()))
        .finish_api_with(&mut api, |t| {
            config
                .remote_oas_paths
                .iter()
                .cloned()
                .fold(t, |t, path| merge_remote_oas(path, t))
        });

    // TODO: This removes trailing forward slashes from paths displayed in the OAS. This is a workaround as it should be fixed upstream in aide:
    // https://github.com/tamasfe/aide/issues/134.
    if let Some(paths) = api.paths.as_mut().map(|this| &mut this.paths) {
        let paths_for_modification = std::mem::take(paths);
        paths.extend(
            paths_for_modification
                .into_iter()
                .map(|(path, ref_or_item)| (path.trim_end_matches('/').to_string(), ref_or_item)),
        );
    }

    let versioned_api_router = Router::new()
        .nest(format!("/{}", API_VERSION).as_str(), api_router)
        .fallback(version_fallback_handler)
        .with_state(app_state);
    (versioned_api_router, api)
}

pub struct App {
    /// The axum router that should be turned into a service.
    pub router: Router,
    /// The open api specification of the service.
    pub api: OpenApi,
    /// Join handle for the context manager task. Can be used to abort
    /// the task when shutting down the application.
    pub context_manager_join_handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct AppPreparationError;
impl std::fmt::Display for AppPreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "context api preparation failed")
    }
}
impl std::error::Error for AppPreparationError {}

impl App {
    /// Prepares a new app.
    ///
    /// # Side effects
    ///
    /// A context manager actor responsible for context cleanup and providing
    /// exclusive access to individual contexts gets launched in the background
    /// when this method is called. Moreover all pending database migrations will also
    /// run.
    pub async fn prepare(
        config: ContextApiConfig,
    ) -> Result<Self, error_stack::Report<AppPreparationError>> {
        let db_url = &config.db_file_path;
        let max_db_connections = config.max_db_connections.0;
        let db_facade = DbFacade::new(db_url, max_db_connections)
            .await
            .change_context(AppPreparationError)
            .attach_printable("could not establish database facade")?;
        let db_facade = Arc::new(db_facade);

        let image_api_settings = config.image_api_settings.clone();
        let image_handler = ImageHandler::new(image_api_settings.store)
            .map_err(|_| AppPreparationError)
            .attach_printable("could not construct image handler")?;

        let net_ctrl = NetCtrlClient::new(config.net_ctrl_base_path.clone());
        let remote_client = RemoteClient::default();

        let ContextManagerCommunication {
            context_manager_tx,
            join_handle,
        } = context_manager::spawn_context_manager(
            db_facade.clone(),
            net_ctrl.clone(),
            remote_client.clone(),
            image_handler.clone(),
        )
        .await
        .change_context(AppPreparationError)
        .attach_printable("could not spawn the context manager task")?;

        let app_state = AppState::new(
            Arc::new(config.clone()),
            net_ctrl,
            remote_client,
            image_handler,
            db_facade,
            context_manager_tx,
        );
        let (router, api) = router_with_api(app_state, &config);

        Ok(Self {
            router,
            api,
            context_manager_join_handle: join_handle,
        })
    }
}
// Note that this clones the document on each request.
// To be more efficient, we could wrap it into an Arc,
// or even store it as a serialized string.
async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

async fn fallback_handler(method: Method, uri: Uri) -> (StatusCode, String) {
    debug!("Unhandled route: {}", uri);
    let code = StatusCode::NOT_FOUND;
    let msg = if uri.path().ends_with('/') {
        String::from("The API does not accept routes ending with a trailing '/'")
    } else {
        format!("No route for `{} {}`", method, uri)
    };
    (code, msg)
}

async fn version_fallback_handler(method: Method, uri: Uri) -> (StatusCode, String) {
    debug!("Unhandled API version: {}", uri);
    (
        StatusCode::NOT_FOUND,
        format!("API version not available for `{} {}`", method, uri),
    )
}

pub(crate) async fn handle_error(
    _method: Method,
    _uri: Uri,
    err: BoxError,
) -> (StatusCode, String) {
    if err.is::<tower_http::timeout::TimeoutError>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        warn!("Unhandled internal error: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", err),
        )
    }
}
