// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::axum::ApiRouter;
use aide::openapi::{Info, License, OpenApi};
use axum::Router;
use axum::http::{Method, StatusCode, Uri};
use std::future::Future;
use tracing::debug;

/// Prepare a Router while augmenting an OpenApi description
pub async fn api_router<F, R, E, S>(
    description: &str,
    version: &str,
    f: F,
) -> Result<(Router<S>, OpenApi), E>
where
    F: FnOnce(ApiRouter<S>) -> R,
    R: Future<Output = Result<ApiRouter<S>, E>>,
    S: Clone + Sync + Send + 'static,
{
    let mut api = OpenApi {
        info: Info {
            description: Some(description.to_string()),
            version: version.to_string(),
            license: Some(License {
                name: "Apache-2.0".to_string(),
                ..Default::default()
            }),
            ..Info::default()
        },
        ..Default::default()
    };

    let router = ApiRouter::new();
    let router = f(router).await?;
    let router = router.fallback(fallback_handler).finish_api(&mut api);
    Ok((router, api))
}

async fn fallback_handler(method: Method, uri: Uri) -> (StatusCode, String) {
    debug!("unhandled route: {}", uri);
    (
        StatusCode::NOT_FOUND,
        format!("No route for `{} {}`", method, uri),
    )
}
