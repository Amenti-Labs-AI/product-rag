mod payload;
mod routes;
mod state;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use product_rag_embed::Embedder;
use routes::{chat, health, search};
use state::AppState;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "product-rag-api", about = "Product catalog semantic search and RAG chat API")]
struct Args {
    #[arg(long, env = "API_HOST", default_value = "0.0.0.0")]
    host: String,

    #[arg(long, env = "API_PORT", default_value_t = 8080)]
    port: u16,

    #[arg(long, env = "EMBED_CACHE_DIR", default_value = "/cache/fastembed")]
    embed_cache_dir: PathBuf,

    #[arg(long, env = "OLLAMA_URL", default_value = "http://127.0.0.1:11434")]
    ollama_url: String,

    #[arg(long, env = "OLLAMA_MODEL", default_value = "llama3.2:3b")]
    ollama_model: String,

    #[arg(long, env = "RAG_TOP_K", default_value_t = 3)]
    rag_top_k: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let qdrant = product_rag_common::QdrantConfig::from_env();
    let qdrant_client = qdrant.connect().await?;
    let embedder = Embedder::open(Some(args.embed_cache_dir))?;

    let state = Arc::new(AppState {
        qdrant: qdrant_client,
        embedder: Mutex::new(embedder),
        ollama_url: args.ollama_url,
        ollama_model: args.ollama_model,
        rag_top_k: args.rag_top_k,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/search", axum::routing::post(search))
        .route("/v1/chat", axum::routing::post(chat))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
