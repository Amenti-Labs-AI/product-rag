use std::path::PathBuf;

use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use product_rag_common::VECTOR_DIM;

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn open(cache_dir: Option<PathBuf>) -> Result<Self> {
        let mut options = InitOptions::new(EmbeddingModel::BGESmallENV15);
        if let Some(dir) = cache_dir {
            options = options.with_cache_dir(dir);
        }

        let model = TextEmbedding::try_new(options).context("init fastembed BGE-small")?;
        Ok(Self { model })
    }

    pub fn embed_passages(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("passage: {t}"))
            .collect();
        self.embed_raw(&prefixed)
    }

    pub fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
        let prefixed = format!("query: {query}");
        let mut vectors = self.embed_raw(&[prefixed])?;
        vectors
            .pop()
            .context("expected one query embedding")
    }

    fn embed_raw(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let embeddings = self
            .model
            .embed(texts.to_vec(), None)
            .context("fastembed inference")?;

        for row in &embeddings {
            if row.len() as u64 != VECTOR_DIM {
                anyhow::bail!(
                    "unexpected embedding dim {} (expected {VECTOR_DIM})",
                    row.len()
                );
            }
        }

        Ok(embeddings)
    }
}
