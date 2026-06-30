use rkyv::{Archive, Deserialize, Serialize};

/// A completely zero-copy text chunk that directly references
/// the underlying buffer extracted from PDFKit or mmap.
/// By deriving rkyv traits, this struct can be serialized and deserialized
/// to/from disk or memory mapped storage with ZERO CPU allocations.
#[derive(Archive, Deserialize, Serialize, Debug)]
pub struct ZeroCopyChunk<'a> {
    pub page_number: u32,
    pub chunk_index: u32,
    pub content: &'a str,
}

pub struct ZeroChunker {
    pub max_chars: usize,
}

impl ZeroChunker {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }

    /// Returns a zero-allocation iterator over zero-copy slices `&str`.
    /// Does not allocate any new `String` or `Vec` for the chunks.
    pub fn chunk_text<'a>(&self, text_buffer: &'a str) -> ZeroChunkerIter<'a> {
        ZeroChunkerIter {
            text: text_buffer,
            max_chars: self.max_chars,
            current_idx: 0,
            chunk_count: 0,
        }
    }
}

pub struct ZeroChunkerIter<'a> {
    text: &'a str,
    max_chars: usize,
    current_idx: usize,
    chunk_count: u32,
}

impl<'a> Iterator for ZeroChunkerIter<'a> {
    type Item = ZeroCopyChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.text.len();
        if self.current_idx >= len {
            return None;
        }

        let end_idx = std::cmp::min(self.current_idx + self.max_chars, len);

        // Align to character boundaries
        let mut safe_end = end_idx;
        while safe_end < len && !self.text.is_char_boundary(safe_end) {
            safe_end -= 1;
        }
        if safe_end <= self.current_idx {
            // Failsafe to prevent infinite loops on weird unicode
            safe_end = self.current_idx
                + self.text[self.current_idx..]
                    .chars()
                    .next()
                    .unwrap()
                    .len_utf8();
        }

        let slice = &self.text[self.current_idx..safe_end];
        let chunk = ZeroCopyChunk {
            page_number: 1, // simplified
            chunk_index: self.chunk_count,
            content: slice,
        };

        self.current_idx = safe_end;
        self.chunk_count += 1;

        Some(chunk)
    }
}
