use serde::{Deserialize, Serialize};

/// Configuration for the text chunker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Maximum chunk size in characters
    pub chunk_size: usize,
    /// Overlap between consecutive chunks in characters
    pub chunk_overlap: usize,
    /// Separator used to split text
    pub separator: String,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 64,
            separator: "\n\n".to_string(),
        }
    }
}

/// A single chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The text content of this chunk
    pub content: String,
    /// Zero-based index of this chunk in the original text
    pub index: usize,
    /// Start character offset in the original text
    pub start_char: usize,
    /// End character offset in the original text (exclusive)
    pub end_char: usize,
}

/// Text chunker for splitting documents into RAG-compatible chunks.
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }

    /// Create a chunker with default configuration.
    pub fn default_config() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Split text into chunks, returning just the content strings.
    pub fn chunk(&self, text: &str) -> Vec<String> {
        self.chunk_with_metadata(text)
            .into_iter()
            .map(|c| c.content)
            .collect()
    }

    /// Split text into chunks with full metadata (index, start/end offsets).
    ///
    /// The algorithm works as follows:
    /// 1. Convert text to a vec of chars for safe indexing on multi-byte UTF-8.
    /// 2. Find all separator boundaries in the text (start/end char positions).
    /// 3. Slide a window of `chunk_size` characters across the text, snapping
    ///    chunk boundaries to the nearest separator boundary when possible.
    /// 4. Respect `chunk_overlap` so consecutive chunks share some context.
    ///
    /// **Key invariants**:
    /// - Every character in the original text appears in at least one chunk.
    /// - Separators are never silently dropped; they belong to the chunk that
    ///   precedes them (trailing) or follows them (leading), depending on split
    ///   position.
    /// - The first chunk always starts at character 0.
    pub fn chunk_with_metadata(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();

        if total_chars <= self.config.chunk_size {
            return vec![Chunk {
                content: text.to_string(),
                index: 0,
                start_char: 0,
                end_char: total_chars,
            }];
        }

        // Pre-compute character positions where separators occur.
        // Each entry is the char index where a separator starts.
        let sep_chars: Vec<char> = self.config.separator.chars().collect();
        let sep_len = sep_chars.len();
        let mut separator_starts: Vec<usize> = Vec::new();

        if sep_len > 0 {
            for i in 0..=total_chars - sep_len {
                if chars[i..i + sep_len] == sep_chars[..] {
                    separator_starts.push(i);
                }
            }
        }

        let mut chunks = Vec::new();
        let mut chunk_start: usize = 0;
        let overlap = self.config.chunk_overlap;
        let chunk_size = self.config.chunk_size;

        while chunk_start < total_chars {
            let ideal_end = std::cmp::min(chunk_start + chunk_size, total_chars);

            // Snap the end to the nearest separator boundary if one is nearby.
            let chunk_end = if ideal_end < total_chars {
                self.find_split_point(&separator_starts, chunk_start, ideal_end, &chars)
            } else {
                total_chars
            };

            let content: String = chars[chunk_start..chunk_end].iter().collect();
            chunks.push(Chunk {
                content,
                index: chunks.len(),
                start_char: chunk_start,
                end_char: chunk_end,
            });

            if chunk_end >= total_chars {
                break;
            }

            // Advance the start position for the next chunk.
            // Step forward by (chunk_end - chunk_start - overlap), ensuring at least 1 char progress.
            let current_len = chunk_end - chunk_start;
            let step = current_len.saturating_sub(overlap).max(1);
            chunk_start += step;

            // Snap the new start to a separator boundary to avoid cutting in the middle of a separator.
            if !separator_starts.is_empty() {
                // Find the first separator that starts at or after chunk_start.
                if let Some(&sep_start) = separator_starts.iter().find(|&&s| s >= chunk_start) {
                    // Check if the separator is close enough to our desired start.
                    // Only snap if the separator start is within a reasonable distance
                    // (no more than overlap/2 chars away) to avoid making chunks too large.
                    let max_snap = (overlap / 2).max(1);
                    if sep_start <= chunk_start + max_snap {
                        // Start just after the separator so the separator belongs to the
                        // previous chunk as trailing whitespace.
                        let after_sep = sep_start + sep_len;
                        if after_sep < chunk_end {
                            // Only snap if we don't go backwards.
                            chunk_start = after_sep;
                        }
                    }
                }
            }
        }

        chunks
    }

    /// Given an ideal end position, find the best split point by looking for
    /// a separator boundary.  We prefer to split *after* a separator so the
    /// separator stays with the text that precedes it.
    ///
    /// If no separator is found near the ideal end, we simply return `ideal_end`.
    fn find_split_point(
        &self,
        separator_starts: &[usize],
        chunk_start: usize,
        ideal_end: usize,
        _chars: &[char],
    ) -> usize {
        let sep_len = self.config.separator.chars().count();

        // We look for a separator whose end (start + sep_len) is at or before ideal_end.
        // The separator "belongs" to the chunk before it.
        let mut best_end = ideal_end;

        // Search backwards from ideal_end for a separator that ends at or before ideal_end.
        for &sep_start in separator_starts.iter().rev() {
            let sep_end = sep_start + sep_len;
            if sep_end > ideal_end {
                continue;
            }
            if sep_start < chunk_start {
                break;
            }
            // Found a separator that ends at or before ideal_end.
            // Split right after the separator so it's included in the current chunk.
            best_end = sep_end;
            break;
        }

        best_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChunkerConfig::default();
        assert_eq!(config.chunk_size, 512);
        assert_eq!(config.chunk_overlap, 64);
        assert_eq!(config.separator, "\n\n");
    }

    #[test]
    fn test_chunk_empty_text() {
        let chunker = Chunker::default_config();
        let result = chunker.chunk("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_chunk_short_text() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 512,
            chunk_overlap: 64,
            separator: "\n\n".to_string(),
        });
        let result = chunker.chunk("Hello, world!");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Hello, world!");
    }

    #[test]
    fn test_chunk_with_separator() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 100,
            chunk_overlap: 10,
            separator: "\n\n".to_string(),
        });
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let result = chunker.chunk(text);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 100,
            chunk_overlap: 10,
            separator: "\n\n".to_string(),
        });
        let text = "Hello world";
        let chunks = chunker.chunk_with_metadata(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].start_char, 0);
        assert_eq!(chunks[0].end_char, 11);
        assert_eq!(chunks[0].content, "Hello world");
    }

    #[test]
    fn test_chunk_long_text_splits() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 20,
            chunk_overlap: 5,
            separator: "\n\n".to_string(),
        });
        let text = "abcdefghijklmnopqrst"; // 20 chars, fits in one chunk exactly
        let chunks = chunker.chunk_with_metadata(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content.len(), 20);
    }

    #[test]
    fn test_chunk_oversized_text_with_overlap() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 10,
            chunk_overlap: 3,
            separator: "\n\n".to_string(),
        });
        // 30 chars, no separator — should split into multiple chunks with overlap
        let text = "abcdefghijklmnopqrstuvwxyzabcd";
        let chunks = chunker.chunk_with_metadata(text);
        assert!(chunks.len() > 1);
        // First chunk should be 10 chars
        assert_eq!(chunks[0].content.len(), 10);
        // Subsequent chunks should have overlap
        if chunks.len() > 1 {
            let overlap_start = chunks[1].start_char;
            let prev_end = chunks[0].end_char;
            assert!(
                overlap_start < prev_end,
                "Overlap should exist between consecutive chunks"
            );
        }
    }

    #[test]
    fn test_chunk_multiple_paragraphs_merge() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 50,
            chunk_overlap: 10,
            separator: "\n\n".to_string(),
        });
        let text = "Short.\n\nAlso short.\n\nAnother one.";
        let chunks = chunker.chunk_with_metadata(text);
        // All three small paragraphs should merge into one chunk (under 50 chars)
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_chunk_preserves_content() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 512,
            chunk_overlap: 64,
            separator: "\n\n".to_string(),
        });
        let text = "Hello world\n\nThis is a test";
        let chunks = chunker.chunk_with_metadata(text);
        // Reconstructed content should match the original
        let reconstructed: String = chunks
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_chunk_multibyte_utf8() {
        // Chinese characters: each char is 3 bytes in UTF-8
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 5, // 5 characters
            chunk_overlap: 1,
            separator: "\n\n".to_string(),
        });
        let text = "你好世界测试分割文本"; // 8 characters, 24 bytes
        let chunks = chunker.chunk_with_metadata(text);
        assert!(!chunks.is_empty());
        // First chunk should be exactly 5 characters
        assert_eq!(chunks[0].content.chars().count(), 5);
        assert_eq!(chunks[0].content, "你好世界测");
        // No panic — that's the main assertion
    }

    #[test]
    fn test_chunk_multibyte_with_separator() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 10,
            chunk_overlap: 2,
            separator: "\n\n".to_string(),
        });
        let text = "你好世界\n\n测试分割文本";
        let chunks = chunker.chunk_with_metadata(text);
        assert!(!chunks.is_empty());
        // All chunk contents should be valid UTF-8 strings
        for chunk in &chunks {
            assert!(chunk.content.is_char_boundary(chunk.content.len()));
        }
    }

    #[test]
    fn test_chunk_mixed_ascii_multibyte() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 7,
            chunk_overlap: 2,
            separator: "\n\n".to_string(),
        });
        // Mix of ASCII and multi-byte chars, length = 7 chars
        let text = "Hello你好"; // 7 characters (5 ASCII + 2 Chinese)
        let chunks = chunker.chunk_with_metadata(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello你好");
        assert_eq!(chunks[0].start_char, 0);
        assert_eq!(chunks[0].end_char, 7);
    }

    // ─── Content coverage tests ───────────────────────────────

    /// Verifies that every character in `text` appears in at least one chunk.
    /// This accounts for overlap: consecutive chunks share characters, so
    /// we cannot simply concatenate and compare.  Instead we check that the
    /// union of [start_char, end_char) intervals covers [0, text.chars().count()).
    fn assert_full_coverage(chunker: &Chunker, text: &str) {
        let chunks = chunker.chunk_with_metadata(text);
        let total = text.chars().count();

        // Build a boolean coverage array
        let mut covered = vec![false; total];
        for chunk in &chunks {
            for i in chunk.start_char..chunk.end_char {
                covered[i] = true;
            }
        }

        let uncovered: Vec<usize> = covered
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if !c { Some(i) } else { None })
            .collect();

        assert!(
            uncovered.is_empty(),
            "Characters at positions {:?} are not covered by any chunk (total={}, chunks={})",
            uncovered,
            total,
            chunks.len()
        );
    }

    #[test]
    fn test_chunk_chinese_long_text_coverage() {
        let paragraph = "这是一段中文测试文本，用于验证知识库分块功能是否完整地保留了所有内容。分块时前面的内容不应该丢失。";
        let paragraphs: Vec<String> = (0..20)
            .map(|i| format!("第{}段：{}", i + 1, paragraph))
            .collect();
        let text = paragraphs.join("\n\n");
        assert!(text.chars().count() > 600);

        let chunker = Chunker::default_config();
        assert_full_coverage(&chunker, &text);
    }

    #[test]
    fn test_chunk_chinese_small_chunk_size_coverage() {
        let paragraph = "这是一段中文测试文本，用于验证知识库分块功能是否完整地保留了所有内容。";
        let paragraphs: Vec<String> = (0..10)
            .map(|i| format!("第{}段：{}", i + 1, paragraph))
            .collect();
        let text = paragraphs.join("\n\n");

        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 30,
            chunk_overlap: 5,
            separator: "\n\n".to_string(),
        });
        assert_full_coverage(&chunker, &text);
    }

    #[test]
    fn test_chunk_chinese_leading_separator_coverage() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 30,
            chunk_overlap: 5,
            separator: "\n\n".to_string(),
        });
        let text = "\n\n你好世界\n\n测试分割文本";
        assert_full_coverage(&chunker, text);
    }

    #[test]
    fn test_chunk_chinese_no_separator_coverage() {
        let text = "你好世界测试分割文本内容丢失验证".repeat(5);
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 10,
            chunk_overlap: 3,
            separator: "\n\n".to_string(),
        });
        assert_full_coverage(&chunker, &text);
    }

    #[test]
    fn test_chunk_first_chunk_starts_at_zero() {
        // Regardless of text structure, the first chunk must start at char 0
        let texts = vec![
            "Hello world",
            "\n\nLeading separator",
            "你好世界",
            "\n\n你好世界\n\n测试",
        ];
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 20,
            chunk_overlap: 5,
            separator: "\n\n".to_string(),
        });
        for text in texts {
            let chunks = chunker.chunk_with_metadata(text);
            assert!(
                !chunks.is_empty(),
                "Should produce at least one chunk for {:?}",
                text
            );
            assert_eq!(
                chunks[0].start_char, 0,
                "First chunk must start at char 0 for {:?}",
                text
            );
        }
    }

    #[test]
    fn test_chunk_no_char_dropped_between_chunks() {
        // For texts without overlap, ensure continuity
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 20,
            chunk_overlap: 0, // no overlap
            separator: "\n\n".to_string(),
        });

        let paragraph = "这是一段中文测试文本。";
        let text = (0..5)
            .map(|i| format!("第{}段：{}", i + 1, paragraph))
            .collect::<Vec<_>>()
            .join("\n\n");

        let chunks = chunker.chunk_with_metadata(&text);
        let total = text.chars().count();

        // Build coverage
        let mut covered = vec![false; total];
        for chunk in &chunks {
            for i in chunk.start_char..chunk.end_char {
                covered[i] = true;
            }
        }
        let uncovered: Vec<usize> = covered
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if !c { Some(i) } else { None })
            .collect();
        assert!(uncovered.is_empty(), "Uncovered chars: {:?}", uncovered);
    }

    #[test]
    fn test_chunk_preserves_content_no_overlap() {
        // With zero overlap, concatenating chunks should reproduce the original
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 20,
            chunk_overlap: 0,
            separator: "\n\n".to_string(),
        });
        let text = "Hello world\n\nThis is a test\n\nMore text here";
        let chunks = chunker.chunk_with_metadata(&text);
        let reconstructed: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_chunk_chinese_preserves_content_no_overlap() {
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: 30,
            chunk_overlap: 0,
            separator: "\n\n".to_string(),
        });
        let paragraph = "这是一段中文测试文本，用于验证知识库分块功能是否完整地保留了所有内容。";
        let paragraphs: Vec<String> = (0..10)
            .map(|i| format!("第{}段：{}", i + 1, paragraph))
            .collect();
        let text = paragraphs.join("\n\n");

        let chunks = chunker.chunk_with_metadata(&text);
        let reconstructed: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert_eq!(reconstructed, text);
    }
}
