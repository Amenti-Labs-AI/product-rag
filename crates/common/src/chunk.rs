#[derive(Debug, Clone)]
pub struct TextChunk {
    pub index: usize,
    pub text: String,
}

/// Simple character-window chunking (~512 chars, ~80 overlap).
pub fn chunk_text(document: &str, max_chars: usize, overlap_chars: usize) -> Vec<TextChunk> {
    let trimmed = document.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.len() <= max_chars {
        return vec![TextChunk {
            index: 0,
            text: trimmed.to_string(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut index = 0usize;

    while start < trimmed.len() {
        let end = (start + max_chars).min(trimmed.len());
        let slice = trimmed[start..end].trim();
        if !slice.is_empty() {
            chunks.push(TextChunk {
                index,
                text: slice.to_string(),
            });
            index += 1;
        }
        if end >= trimmed.len() {
            break;
        }
        start = end.saturating_sub(overlap_chars);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chunk_when_short() {
        let chunks = chunk_text("hello", 100, 10);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn multiple_chunks_when_long() {
        let doc = "word ".repeat(200);
        let chunks = chunk_text(&doc, 100, 20);
        assert!(chunks.len() > 1);
    }
}
