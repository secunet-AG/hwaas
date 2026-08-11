// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::{
    axum::{
        ApiRouter,
        routing::{ApiMethodRouter, get_with},
    },
    operation::{OperationHandler, OperationInput, OperationOutput},
    transform::{TransformOperation, TransformPathItem},
};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path};
use axum::handler::Handler;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{instrument, warn};
use urlencoding::encode;

use crate::api::ExtractAux;
use crate::api::reverse_proxy::send_aux_request;
use crate::app_state::AppState;

/// Helper function to specify all supported methods for the given handler.
fn get_all_handlers<H, I, O, T>(handler: H) -> ApiMethodRouter<AppState>
where
    H: Handler<T, AppState> + OperationHandler<I, O>,
    I: OperationInput,
    O: OperationOutput,
    T: 'static,
{
    get_with(handler.clone(), handle_aux_request_doc)
        .post_with(handler.clone(), handle_aux_request_doc)
        .put_with(handler.clone(), handle_aux_request_doc)
        .delete_with(handler.clone(), handle_aux_request_doc)
}

/// Router for requests to be forwarded to a single auxiliary device
pub fn aux_device_api_router() -> ApiRouter<AppState> {
    let supported_method_handlers = get_all_handlers(handle_aux_request);
    let supported_method_handlers_with_path = get_all_handlers(handle_aux_request_with_path);
    ApiRouter::new()
        .api_route_with(
            "/auxiliaries/:device_id/api",
            supported_method_handlers,
            aux_api_doc,
        )
        .api_route_with(
            "/auxiliaries/:device_id/api/*path_suffix",
            supported_method_handlers_with_path,
            aux_api_with_path_doc,
        )
        .layer(DefaultBodyLimit::disable())
}

/// The tag for the AuxiliaryAPI. Used for (un-)folding this API in the UI.
fn aux_api_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Auxiliary Device API")
}

/// The `/*path_suffix` route is hidden in the OpenAPI Spec because
/// AIDE turns it into a `{path_suffix+}` parameter which is actually
/// no valid OpenAPI.
fn aux_api_with_path_doc(op: TransformPathItem) -> TransformPathItem {
    op.tag("Auxiliary Device API").hidden(true)
}

/// Documentation for the forwarding handler.
/// Potential errors are unknown due to the nature of forwarding.
fn handle_aux_request_doc(op: TransformOperation) -> TransformOperation {
    op.summary("The auxiliary device specific API")
        .description(
            "Any requests send to this endpoint will be reverse proxied to the auxiliary device. \
            Refer to the API documentation of the specific auxiliary device for more information.",
        )
        .response_with::<200, (), _>(|op| {
            op.description("Proxied from the auxiliary service. Response could be anything!")
        })
}

/// Path segment labels will be matched with struct field name
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuxiliaryDevicePath {
    /// The part of the URI that will be appended
    pub path_suffix: String,
}

/// Modify a request without any path that was sent to the AuxiliaryApi and forward
/// it to a Auxiliary Device instance.
#[instrument(skip(inner, method, headers, body))]
async fn handle_aux_request(
    method: Method,
    ExtractAux {
        state: inner,
        query,
    }: ExtractAux,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    let url = create_url(&inner.aux_config.url, None, query)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Bad URL format"))?;

    send_aux_request(method, url, headers, body).await
}

/// Modify a request with additional path that was sent to the AuxiliaryApi and
/// forward it to a Auxiliary Device instance.
#[instrument(skip(inner, method, headers, body))]
async fn handle_aux_request_with_path(
    method: Method,
    ExtractAux {
        state: inner,
        query,
    }: ExtractAux,
    Path(AuxiliaryDevicePath { path_suffix }): Path<AuxiliaryDevicePath>,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Result<Response, (StatusCode, &'static str)> {
    let url = create_url(&inner.aux_config.url, Some(path_suffix), query)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Bad URL format"))?;

    send_aux_request(method, url, headers, body).await
}

/// For a given url, suffix and query list, return the resulting URI
fn create_url(
    url: &str,
    path_suffix: Option<String>,
    query: HashMap<String, String>,
) -> Result<Uri, ()> {
    match path_suffix {
        None => url_with_query(url.to_string(), query).parse::<Uri>(),
        Some(suf) => url_with_query(format!("{}/{}", url, suf), query).parse::<Uri>(),
    }
    .map_err(|e| {
        warn!("could not build URL: {}", e);
    })
}

/// Append query parameter to URL.
fn url_with_query(url: String, query: HashMap<String, String>) -> String {
    let mut url_query = url;
    url_query = format!("{url_query}?");
    for (p, v) in query {
        let encoded_p = encode(&p);
        let encoded_v = encode(&v);
        url_query = format!("{url_query}{encoded_p}={encoded_v}&");
    }
    let mut chars = url_query.chars();
    // either remove last '&' or '?' if no query parameter are present
    chars.next_back();
    chars.as_str().to_string()
}
