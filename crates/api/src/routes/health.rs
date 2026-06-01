use std::sync::Arc;

use axum::{extract::State, Json};
use product_rag_common::COLLECTION_NAME;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub qdrant: bool,
    pub ollama: bool,
    pub collection: String,
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let qdrant_ok = state
        .qdrant
        .collection_exists(COLLECTION_NAME)
        .await
        .unwrap_or(false);

    let ollama_ok = check_ollama(&state.ollama_url).await;

    let status = if qdrant_ok && ollama_ok {
        "ok"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status,
        qdrant: qdrant_ok,
        ollama: ollama_ok,
        collection: COLLECTION_NAME.to_string(),
    })
}

async fn check_ollama(base_url: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
