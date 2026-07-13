// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::app_state::AppState;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, warn};

pub(crate) async fn route_call_stats<B>(
    State(state): State<Arc<AppState>>,
    req: Request<B>,
    next: Next<B>,
) -> Response
where
    B: Send,
{
    // get URL id
    let id = req.uri().to_string();

    // push stats
    if !state.stats.contains_key(&id) {
        if let Some(v) = state.stats.insert(id.clone(), Value::from(0)) {
            warn!("there was already a statistic for {}: {:?}", &id, v)
        }
    };

    state.stats.alter(&id, |_, v| match v {
        Value::Number(n) => Value::from(n.as_u64().unwrap() + 1),
        _ => {
            error!("Unexpected value type: {:?}", v);
            v
        }
    });

    debug!("route_call_stats middleware done");
    next.run(req).await
}
