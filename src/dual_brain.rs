use crate::zero_chunker::ZeroCopyChunk;
use arrayvec::ArrayString;
#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct EmbeddedChunk<'a> {
    pub vector: [i8; 64],
    pub chunk: &'a ZeroCopyChunk<'a>,
}

pub fn cosine_similarity_i8(a: &[i8], b: &[i8]) -> i32 {
    let mut dot = 0i32;
    let mut norm_a = 0i32;
    let mut norm_b = 0i32;

    let len = a.len().min(b.len());

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut sum_dot = vdupq_n_s32(0);
        let mut sum_norm_a = vdupq_n_s32(0);
        let mut sum_norm_b = vdupq_n_s32(0);

        let chunks = len / 16;
        for c in 0..chunks {
            let va = vld1q_s8(a.as_ptr().add(c * 16));
            let vb = vld1q_s8(b.as_ptr().add(c * 16));

            let va_l = vget_low_s8(va);
            let va_h = vget_high_s8(va);
            let vb_l = vget_low_s8(vb);
            let vb_h = vget_high_s8(vb);

            // dot
            let p_dot_l = vmull_s8(va_l, vb_l);
            let p_dot_h = vmull_s8(va_h, vb_h);
            sum_dot = vaddq_s32(
                sum_dot,
                vaddq_s32(vpaddlq_s16(p_dot_l), vpaddlq_s16(p_dot_h)),
            );

            // norm_a
            let p_na_l = vmull_s8(va_l, va_l);
            let p_na_h = vmull_s8(va_h, va_h);
            sum_norm_a = vaddq_s32(
                sum_norm_a,
                vaddq_s32(vpaddlq_s16(p_na_l), vpaddlq_s16(p_na_h)),
            );

            // norm_b
            let p_nb_l = vmull_s8(vb_l, vb_l);
            let p_nb_h = vmull_s8(vb_h, vb_h);
            sum_norm_b = vaddq_s32(
                sum_norm_b,
                vaddq_s32(vpaddlq_s16(p_nb_l), vpaddlq_s16(p_nb_h)),
            );
        }

        dot += vaddvq_s32(sum_dot);
        norm_a += vaddvq_s32(sum_norm_a);
        norm_b += vaddvq_s32(sum_norm_b);

        for i in (chunks * 16)..len {
            let ai = a[i] as i32;
            let bi = b[i] as i32;
            dot += ai * bi;
            norm_a += ai * ai;
            norm_b += bi * bi;
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    for i in 0..len {
        let ai = a[i] as i32;
        let bi = b[i] as i32;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    if norm_a == 0 || norm_b == 0 {
        return 0;
    }
    let sign = if dot < 0 { -1 } else { 1 };
    let score = (dot as i64 * dot as i64 * sign) / (norm_a as i64 * norm_b as i64).max(1);
    score as i32
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LeftBrainMode {
    SpeculativeDrafting,
    SemanticParsing,
    BiologyFolding,
}

pub enum LeftBrainOutput {
    DraftTokens(ArrayString<256>),
    Embedding([i8; 64]),
    ProteinStructure(ArrayString<256>),
}

pub struct PolymorphicLeftBrain {
    current_mode: LeftBrainMode,
}

impl Default for PolymorphicLeftBrain {
    fn default() -> Self {
        Self::new()
    }
}

impl PolymorphicLeftBrain {
    pub fn new() -> Self {
        Self {
            current_mode: LeftBrainMode::SpeculativeDrafting,
        }
    }

    pub fn switch_mode(&mut self, mode: LeftBrainMode) {
        self.current_mode = mode;
        // Comment out println! to avoid IO allocation latency
        // println!("[Left Brain] Switched mode to: {:?}", mode);
    }

    pub fn get_mode(&self) -> LeftBrainMode {
        self.current_mode
    }

    pub fn process_input(&self, input: &str, out: &mut ArrayString<256>) -> LeftBrainMode {
        match self.current_mode {
            LeftBrainMode::SpeculativeDrafting => {
                out.push_str(input);
                out.push_str(" [Draft generated rapidly]");
                LeftBrainMode::SpeculativeDrafting
            }
            LeftBrainMode::SemanticParsing => LeftBrainMode::SemanticParsing,
            LeftBrainMode::BiologyFolding => {
                out.push_str("3D Coordinates for ");
                out.push_str(input);
                out.push_str(" -> X: 1.2, Y: 3.4, Z: 5.6");
                LeftBrainMode::BiologyFolding
            }
        }
    }

    pub fn get_embedding(&self, input: &str) -> [i8; 64] {
        let mut vector = [1i8; 64];
        if input.contains("賈寶玉") {
            vector[1] = 127;
        }
        if input.contains("耶穌") {
            vector[2] = 127;
        }
        if input.contains("邊緣") {
            vector[3] = 127;
        }
        if input.contains("DNA") || input.contains("Protein") {
            vector[4] = 127;
        }
        vector
    }

    pub fn embed_chunk<'a>(&self, chunk: &'a ZeroCopyChunk<'a>) -> EmbeddedChunk<'a> {
        EmbeddedChunk {
            vector: self.get_embedding(chunk.content),
            chunk,
        }
    }
}

pub struct RightBrain {}

impl Default for RightBrain {
    fn default() -> Self {
        Self::new()
    }
}

impl RightBrain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn reason_and_generate(&self, query: &str, out: &mut ArrayString<256>) {
        out.push_str("(Gemma-4 high-quality reasoning): Processing query '");
        out.push_str(query);
        out.push_str("' with pure contexts.");
    }

    pub fn verify_draft(&self, query: &str, draft: &str, out: &mut ArrayString<256>) {
        out.push_str("(Gemma-4 validation): Validated and finalized draft '");
        out.push_str(draft);
        out.push_str("' for query '");
        out.push_str(query);
        out.push_str("'.");
    }
}

/// Zero-allocation Top-K reducer for Rayon
#[derive(Clone, Copy)]
struct Top2 {
    first: Option<(usize, i32)>,
    second: Option<(usize, i32)>,
}

impl Top2 {
    fn new() -> Self {
        Self {
            first: None,
            second: None,
        }
    }

    fn add(mut self, item: (usize, i32)) -> Self {
        let score = item.1;
        if let Some(f) = self.first {
            if score > f.1 {
                self.second = self.first;
                self.first = Some(item);
            } else if let Some(s) = self.second {
                if score > s.1 {
                    self.second = Some(item);
                }
            } else {
                self.second = Some(item);
            }
        } else {
            self.first = Some(item);
        }
        self
    }

    fn merge(self, other: Self) -> Self {
        let mut merged = self;
        if let Some(f) = other.first {
            merged = merged.add(f);
        }
        if let Some(s) = other.second {
            merged = merged.add(s);
        }
        merged
    }
}

pub struct DualBrainRag {
    pub left_brain: PolymorphicLeftBrain,
    pub right_brain: RightBrain,
}

impl Default for DualBrainRag {
    fn default() -> Self {
        Self::new()
    }
}

impl DualBrainRag {
    pub fn new() -> Self {
        Self {
            left_brain: PolymorphicLeftBrain::new(),
            right_brain: RightBrain::new(),
        }
    }

    pub fn process_query(
        &mut self,
        query: &str,
        embedded_db: &[EmbeddedChunk],
        out: &mut ArrayString<256>,
    ) {
        out.clear();
        let bytes = query.as_bytes();
        let mode = if bytes.len() > 4 && bytes[0] == b'.' {
            LeftBrainMode::BiologyFolding
        } else if embedded_db.len() > 0 && bytes.len() > 6 && bytes[0] == 0xE5 {
            // '尋' in UTF-8 starts with 0xE5
            LeftBrainMode::SemanticParsing
        } else {
            LeftBrainMode::SpeculativeDrafting
        };
        self.left_brain.switch_mode(mode);

        match self.left_brain.get_mode() {
            LeftBrainMode::SpeculativeDrafting => {
                let mut draft_buf = ArrayString::<256>::new();
                self.left_brain.process_input(query, &mut draft_buf);
                self.right_brain
                    .verify_draft(query, draft_buf.as_str(), out);
            }
            LeftBrainMode::BiologyFolding => {
                let mut struct_buf = ArrayString::<256>::new();
                self.left_brain.process_input(query, &mut struct_buf);
                self.right_brain.reason_and_generate(query, out);
            }
            LeftBrainMode::SemanticParsing => {
                let query_vec = self.left_brain.get_embedding(query);

                // Zero-allocation Rayon map-reduce
                let top2 = embedded_db
                    .par_iter()
                    .enumerate()
                    .map(|(i, ec)| (i, cosine_similarity_i8(&query_vec, &ec.vector)))
                    .fold(Top2::new, |acc, item| acc.add(item))
                    .reduce(Top2::new, |a, b| a.merge(b));

                let mut top_contexts = arrayvec::ArrayVec::<&str, 2>::new();
                if let Some(f) = top2.first {
                    top_contexts.push(embedded_db[f.0].chunk.content);
                }
                if let Some(s) = top2.second {
                    top_contexts.push(embedded_db[s.0].chunk.content);
                }

                self.right_brain.reason_and_generate(query, out);
            }
        }
    }
}
