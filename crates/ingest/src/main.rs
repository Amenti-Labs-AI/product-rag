use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use product_rag_common::{
    build_point, chunk_text, ensure_collection, load_products_from_dir, ProductRecord,
    QdrantConfig,
};
use product_rag_embed::Embedder;
use qdrant_client::qdrant::UpsertPointsBuilder;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "product-rag-ingest", about = "Batch ingest product catalog into Qdrant")]
struct Args {
    #[arg(long, env = "DATA_DIR", default_value = "data/products")]
    data_dir: PathBuf,

    #[arg(long, env = "EMBED_CACHE_DIR", default_value = "/cache/fastembed")]
    embed_cache_dir: PathBuf,

    #[arg(long, env = "INGEST_RECREATE", default_value_t = true)]
    recreate: bool,

    #[arg(long, env = "INGEST_BATCH_SIZE", default_value_t = 32)]
    batch_size: usize,
}

struct PendingChunk {
    product: ProductRecord,
    chunk_index: usize,
    text: String,
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
    let qdrant_cfg = QdrantConfig::from_env();
    let client = qdrant_cfg.connect().await?;

    ensure_collection(&client, args.recreate).await?;

    let products = load_products_from_dir(&args.data_dir)
        .with_context(|| format!("load products from {}", args.data_dir.display()))?;
    info!(count = products.len(), "loaded products");

    let mut embedder = Embedder::open(Some(args.embed_cache_dir))?;

    let mut pending = Vec::new();
    for product in &products {
        let doc = product.to_document_text();
        for chunk in chunk_text(&doc, 512, 80) {
            pending.push(PendingChunk {
                product: product.clone(),
                chunk_index: chunk.index,
                text: chunk.text,
            });
        }
    }

    info!(chunks = pending.len(), "chunked catalog");
    let mut upserted = 0usize;

    for batch in pending.chunks(args.batch_size) {
        let texts: Vec<String> = batch.iter().map(|c| c.text.clone()).collect();
        let vectors = embedder.embed_passages(&texts)?;

        let points: Vec<_> = batch
            .iter()
            .zip(vectors)
            .map(|(chunk, vector)| build_point(&chunk.product, chunk.chunk_index, &chunk.text, vector))
            .collect();

        client
            .upsert_points(UpsertPointsBuilder::new(
                product_rag_common::COLLECTION_NAME,
                points,
            ))
            .await
            .context("upsert points")?;

        upserted += batch.len();
        info!(upserted, total = pending.len(), "upserted batch");
    }

    info!(upserted, "ingest complete");
    Ok(())
}
