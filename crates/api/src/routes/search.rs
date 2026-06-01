use std::sync::Arc;

use axum::{extract::State, Json};
use product_rag_common::{
    parse_search_query, payload_matches_constraints, PayloadView, COLLECTION_NAME,
};
use qdrant_client::qdrant::{Condition, Filter, Range, SearchPointsBuilder};
use serde::{Deserialize, Serialize};

use crate::payload::{get_bool, get_f64, get_i64, get_str};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub filter: Option<SearchFilter>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
pub struct SearchFilter {
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub score: f32,
    pub product_id: String,
    pub sku: String,
    pub title: String,
    pub category: String,
    pub chunk_index: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waterproof: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AppliedFilters {
    pub semantic_query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_waterproof: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title_contains: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub filters_applied: AppliedFilters,
    pub hits: Vec<SearchHit>,
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (axum::http::StatusCode, String)> {
    let query = body.query.trim();
    if query.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "query is required".into()));
    }

    let constraints = parse_search_query(query);
    let embed_text = if constraints.semantic_query.is_empty() {
        query
    } else {
        constraints.semantic_query.as_str()
    };

    let vector = state
        .embedder
        .lock()
        .await
        .embed_query(embed_text)
        .map_err(internal_error)?;

    let fetch_limit = (body.limit.saturating_mul(5).max(32)) as u64;
    let mut search = SearchPointsBuilder::new(COLLECTION_NAME, vector, fetch_limit)
        .with_payload(true);

    let mut conditions = Vec::new();

    if let Some(max) = constraints.max_price_usd {
        conditions.push(Condition::range(
            "price_usd",
            Range {
                lte: Some(max),
                ..Default::default()
            },
        ));
    }

    match constraints.require_waterproof {
        Some(true) => conditions.push(Condition::matches("waterproof", true)),
        Some(false) => conditions.push(Condition::matches("waterproof", false)),
        None => {}
    }

    if let Some(filter) = body.filter {
        if let Some(category) = filter.category.filter(|c| !c.trim().is_empty()) {
            conditions.push(Condition::matches("category", category));
        }
    }

    if !conditions.is_empty() {
        search = search.filter(Filter::must(conditions));
    }

    let results = state
        .qdrant
        .search_points(search)
        .await
        .map_err(internal_error)?;

    let hits = results
        .result
        .into_iter()
        .filter_map(|point| {
            let payload = point.payload;
            let title = get_str(&payload, "title")?;
            let view = PayloadView {
                title: &title,
                price_usd: get_f64(&payload, "price_usd"),
                waterproof: get_bool(&payload, "waterproof"),
            };
            if !payload_matches_constraints(&view, &constraints) {
                return None;
            }

            Some(SearchHit {
                score: point.score,
                product_id: get_str(&payload, "product_id")?,
                sku: get_str(&payload, "sku")?,
                title,
                category: get_str(&payload, "category")?,
                chunk_index: get_i64(&payload, "chunk_index")?,
                text: get_str(&payload, "text")?,
                price_usd: get_f64(&payload, "price_usd"),
                waterproof: get_bool(&payload, "waterproof"),
            })
        })
        .take(body.limit)
        .collect();

    Ok(Json(SearchResponse {
        query: query.to_string(),
        filters_applied: AppliedFilters {
            semantic_query: constraints.semantic_query,
            max_price_usd: constraints.max_price_usd,
            require_waterproof: constraints.require_waterproof,
            title_contains: constraints.title_contains,
        },
        hits,
    }))
}

fn internal_error(err: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}
