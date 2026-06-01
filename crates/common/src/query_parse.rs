#[derive(Debug, Clone, PartialEq)]
pub struct SearchConstraints {
    /// Query text used for embedding (price/noise phrases stripped).
    pub semantic_query: String,
    pub max_price_usd: Option<f64>,
    pub require_waterproof: Option<bool>,
    /// Title must contain each term (e.g. "jacket" for "jackets").
    pub title_contains: Vec<String>,
}

pub fn parse_search_query(query: &str) -> SearchConstraints {
    let lower = query.to_lowercase();
    let max_price_usd = extract_max_price(&lower);
    let require_waterproof = parse_waterproof_requirement(&lower);
    let title_contains = extract_title_terms(&lower);
    let semantic_query = build_semantic_query(query, max_price_usd);

    SearchConstraints {
        semantic_query,
        max_price_usd,
        require_waterproof,
        title_contains,
    }
}

fn parse_waterproof_requirement(lower: &str) -> Option<bool> {
    if lower.contains("not waterproof")
        || lower.contains("non-waterproof")
        || lower.contains("non waterproof")
    {
        return Some(false);
    }
    if lower.contains("waterproof") {
        return Some(true);
    }
    None
}

fn extract_title_terms(lower: &str) -> Vec<String> {
    const TERMS: &[(&str, &str)] = &[
        ("jackets", "jacket"),
        ("jacket", "jacket"),
        ("boots", "boot"),
        ("boot", "boot"),
        ("shoes", "shoe"),
        ("shoe", "shoe"),
        ("tents", "tent"),
        ("tent", "tent"),
        ("gloves", "glove"),
        ("glove", "glove"),
        ("backpacks", "backpack"),
        ("backpack", "backpack"),
        ("packs", "pack"),
    ];

    let mut out = Vec::new();
    for (needle, normalized) in TERMS {
        if lower.contains(needle) {
            let norm = (*normalized).to_string();
            if !out.contains(&norm) {
                out.push(norm);
            }
        }
    }
    out
}

fn extract_max_price(lower: &str) -> Option<f64> {
    const MARKERS: &[&str] = &[
        "under $",
        "under ",
        "below $",
        "below ",
        "less than $",
        "less than ",
        "max $",
        "max ",
        "maximum $",
        "maximum ",
    ];

    for marker in MARKERS {
        if let Some(idx) = lower.find(marker) {
            let rest = lower[idx + marker.len()..].trim_start();
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = num.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn build_semantic_query(original: &str, max_price: Option<f64>) -> String {
    let mut q = original.to_string();
    for marker in [
        "under $", "under ", "below $", "below ", "less than $", "less than ",
        "max $", "max ", "maximum $", "maximum ", "dlls", "dollars", "usd",
    ] {
        q = q.replace(marker, " ");
    }
    if let Some(max) = max_price {
        q = q.replace(&max.to_string(), " ");
        if max.fract() == 0.0 {
            q = q.replace(&format!("{:.0}", max), " ");
        }
    }
    let cleaned: String = q.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        original.trim().to_string()
    } else {
        cleaned
    }
}

#[derive(Debug, Clone)]
pub struct PayloadView<'a> {
    pub title: &'a str,
    pub price_usd: Option<f64>,
    pub waterproof: Option<bool>,
}

pub fn payload_matches_constraints(payload: &PayloadView, c: &SearchConstraints) -> bool {
    let title_lower = payload.title.to_lowercase();

    for term in &c.title_contains {
        if !title_lower.contains(term) {
            return false;
        }
    }

    if let Some(max) = c.max_price_usd {
        match payload.price_usd {
            Some(price) if price > max => return false,
            None => return false,
            _ => {}
        }
    }

    match c.require_waterproof {
        Some(true) if payload.waterproof == Some(false) => return false,
        Some(false) if payload.waterproof == Some(true) => return false,
        _ => {}
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_waterproof_jacket_price_query() {
        let c = parse_search_query("waterproof jackets under $50 dlls");
        assert_eq!(c.max_price_usd, Some(50.0));
        assert_eq!(c.require_waterproof, Some(true));
        assert!(c.title_contains.contains(&"jacket".to_string()));
    }

    #[test]
    fn filters_non_jacket_and_over_price() {
        let c = parse_search_query("waterproof jackets under $50");
        let jacket = PayloadView {
            title: "StormShield Rain Jacket",
            price_usd: Some(149.0),
            waterproof: Some(true),
        };
        let boot = PayloadView {
            title: "Canyon Lite Approach Shoe",
            price_usd: Some(119.0),
            waterproof: Some(false),
        };
        let glove = PayloadView {
            title: "ThermalGrip Insulated Gloves",
            price_usd: Some(45.0),
            waterproof: None,
        };

        assert!(!payload_matches_constraints(&jacket, &c));
        assert!(!payload_matches_constraints(&boot, &c));
        assert!(!payload_matches_constraints(&glove, &c));
    }
}
