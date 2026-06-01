use product_rag_embed::Embedder;
use qdrant_client::Qdrant;
use tokio::sync::Mutex;

pub struct AppState {
    pub qdrant: Qdrant,
    pub embedder: Mutex<Embedder>,
    pub ollama_url: String,
    pub ollama_model: String,
    pub rag_top_k: usize,
}
