use std::sync::Arc;

use axum::{extract::State, Json};
use rig::client::{CompletionClient, Nothing};
use rig::completion::Prompt;
use rig::providers::ollama;
use serde::{Deserialize, Serialize};

use crate::routes::search::{SearchFilter, SearchRequest};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub filter: Option<SearchFilter>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub answer: String,
    pub session_id: Option<String>,
    pub sources: Vec<ChatSource>,
}

#[derive(Debug, Serialize)]
pub struct ChatSource {
    pub sku: String,
    pub title: String,
    pub score: f32,
    pub excerpt: String,
}

const SYSTEM_PREAMBLE: &str = "You are a product catalog assistant. Use ONLY the catalog excerpts below.\n\
Rules:\n\
- If excerpts contain enough information to answer, give a direct answer and cite the SKU.\n\
- Do not say you cannot find information that appears in the excerpts.\n\
- Do not hedge or contradict yourself (never say \"I can't find\" and then answer anyway).\n\
- Simple numeric comparisons are allowed (e.g. \"under 3 lbs\" satisfies \"under 3.5 lbs\").\n\
- If excerpts truly lack the answer, reply exactly: \"Not in catalog.\"\n\
- Keep answers under 4 sentences.";

const NOT_IN_CATALOG_MSG: &str =
    "I could not find relevant products in the catalog for that question.";

fn answer_indicates_not_in_catalog(answer: &str) -> bool {
    let normalized = answer.trim().trim_end_matches('.').to_lowercase();
    normalized == "not in catalog" || normalized.contains("not in the catalog")
}

pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (axum::http::StatusCode, String)> {
    let message = body.message.trim();
    if message.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "message is required".into()));
    }

    let search = super::search::search(
        State(state.clone()),
        Json(SearchRequest {
            query: message.to_string(),
            limit: state.rag_top_k,
            filter: body.filter,
        }),
    )
    .await?;

    let hits = search.0.hits;
    if hits.is_empty() {
        return Ok(Json(ChatResponse {
            answer: NOT_IN_CATALOG_MSG.to_string(),
            session_id: body.session_id,
            sources: vec![],
        }));
    }

    let context = hits
        .iter()
        .enumerate()
        .map(|(i, h)| {
            format!(
                "[{}] SKU={} Title={} Category={} Score={:.3}\n{}",
                i + 1,
                h.sku,
                h.title,
                h.category,
                h.score,
                h.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let preamble = format!("{SYSTEM_PREAMBLE}\n\n## Catalog excerpts\n{context}");

    let client: ollama::Client = ollama::Client::builder()
        .api_key(Nothing)
        .base_url(state.ollama_url.trim_end_matches('/'))
        .build()
        .map_err(|e| internal_error(e.to_string()))?;

    let agent = client
        .agent(&state.ollama_model)
        .preamble(&preamble)
        .temperature(0.2)
        .build();

    let user_prompt = format!(
        "Customer question: {message}\n\n\
         Answer from the excerpts only. If they support an answer, state it directly with SKU."
    );

    let answer = agent
        .prompt(&user_prompt)
        .await
        .map_err(|e| internal_error(format!("ollama: {e}")))?;

    if answer_indicates_not_in_catalog(&answer) {
        return Ok(Json(ChatResponse {
            answer: NOT_IN_CATALOG_MSG.to_string(),
            session_id: body.session_id,
            sources: vec![],
        }));
    }

    let sources = hits
        .into_iter()
        .map(|h| ChatSource {
            sku: h.sku,
            title: h.title,
            score: h.score,
            excerpt: truncate(&h.text, 240),
        })
        .collect();

    Ok(Json(ChatResponse {
        answer,
        session_id: body.session_id,
        sources,
    }))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}…", &s[..max.saturating_sub(1)])
}

fn internal_error(msg: String) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
}
