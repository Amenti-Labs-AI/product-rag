use anyhow::{Context, Result};
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, PointStruct, VectorParamsBuilder};
use qdrant_client::{Payload, Qdrant};
use uuid::Uuid;

use crate::ProductRecord;

pub const COLLECTION_NAME: &str = "products";
pub const VECTOR_DIM: u64 = 384;

#[derive(Clone, Debug)]
pub struct QdrantConfig {
    pub url: String,
}

impl QdrantConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6334".into()),
        }
    }

    pub async fn connect(&self) -> Result<Qdrant> {
        Qdrant::from_url(&self.url)
            .build()
            .with_context(|| format!("connect qdrant at {}", self.url))
    }
}

pub async fn ensure_collection(client: &Qdrant, recreate: bool) -> Result<()> {
    if recreate && client.collection_exists(COLLECTION_NAME).await? {
        client
            .delete_collection(COLLECTION_NAME)
            .await
            .context("delete collection")?;
    }

    if !client.collection_exists(COLLECTION_NAME).await? {
        client
            .create_collection(
                CreateCollectionBuilder::new(COLLECTION_NAME).vectors_config(
                    VectorParamsBuilder::new(VECTOR_DIM, Distance::Cosine),
                ),
            )
            .await
            .context("create collection")?;
    }

    Ok(())
}

pub fn point_id(product_id: &str, chunk_index: usize) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("product-rag:{product_id}:{chunk_index}").as_bytes(),
    )
}

pub fn build_point(
    product: &ProductRecord,
    chunk_index: usize,
    text: &str,
    vector: Vec<f32>,
) -> PointStruct {
    let mut payload_json = serde_json::json!({
        "product_id": product.id,
        "sku": product.sku,
        "title": product.title,
        "category": product.category,
        "chunk_index": chunk_index,
        "text": text,
    });

    if let Some(price) = product.price_usd {
        payload_json["price_usd"] = serde_json::json!(price);
    }
    if let Some(waterproof) = product.attribute_bool("waterproof") {
        payload_json["waterproof"] = serde_json::json!(waterproof);
    }

    let payload: Payload = payload_json.try_into().expect("valid payload");

    PointStruct::new(
        point_id(&product.id, chunk_index).to_string(),
        vector,
        payload,
    )
}
