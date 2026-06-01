mod chunk;
mod product;
mod qdrant;
mod query_parse;

pub use chunk::{chunk_text, TextChunk};
pub use product::{load_products_from_dir, ProductRecord};
pub use qdrant::{
    build_point, ensure_collection, point_id, QdrantConfig, COLLECTION_NAME, VECTOR_DIM,
};
pub use query_parse::{parse_search_query, payload_matches_constraints, PayloadView, SearchConstraints};
