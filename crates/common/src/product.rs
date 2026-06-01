use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRecord {
    pub id: String,
    pub sku: String,
    pub title: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub price_usd: Option<f64>,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

impl ProductRecord {
    pub fn attribute_bool(&self, key: &str) -> Option<bool> {
        self.attributes.get(key).and_then(|v| v.as_bool())
    }

    pub fn to_document_text(&self) -> String {
        let attrs = if self.attributes.is_null() {
            String::new()
        } else {
            format!("\nAttributes: {}", self.attributes)
        };
        let price = self
            .price_usd
            .map(|p| format!("\nPrice (USD): {p:.2}"))
            .unwrap_or_default();

        format!(
            "SKU: {}\nTitle: {}\nCategory: {}\nDescription: {}{}{}",
            self.sku, self.title, self.category, self.description, price, attrs
        )
    }
}

#[derive(Debug, Deserialize)]
struct CatalogFile {
    products: Vec<ProductRecord>,
}

pub fn load_products_from_dir(data_dir: &Path) -> Result<Vec<ProductRecord>> {
    let mut products = Vec::new();

    for entry in std::fs::read_dir(data_dir)
        .with_context(|| format!("read data dir {}", data_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)
            .with_context(|| format!("parse JSON {}", path.display()))?;

        if let Ok(catalog) = serde_json::from_value::<CatalogFile>(parsed.clone()) {
            products.extend(catalog.products);
            continue;
        }

        if let Ok(one) = serde_json::from_value::<ProductRecord>(parsed.clone()) {
            products.push(one);
            continue;
        }

        if let Some(arr) = parsed.as_array() {
            for item in arr {
                let record: ProductRecord = serde_json::from_value(item.clone())
                    .with_context(|| format!("parse product in {}", path.display()))?;
                products.push(record);
            }
            continue;
        }

        anyhow::bail!(
            "unsupported JSON shape in {} (expected catalog, product, or array)",
            path.display()
        );
    }

    products.sort_by(|a, b| a.id.cmp(&b.id));
    products.dedup_by(|a, b| a.id == b.id);
    Ok(products)
}
