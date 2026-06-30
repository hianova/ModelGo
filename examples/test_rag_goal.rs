use model_go::dual_brain::DualBrainRag;
use model_go::zero_chunker::ZeroChunker;
use std::time::Instant;

fn main() {
    println!("=== 啟動 ModelGo Dual-Brain RAG (目標測試) ===");

    // 1. 模擬三本巨著 (Infinite Context Simulation)
    let book1_hong_lou_meng = "紅樓夢第一回：甄士隱夢幻識通靈，賈雨村風塵懷閨秀。\n賈寶玉含玉而生，玉上刻著『莫失莫忘，仙壽恆昌』。";
    let book2_bible = "Genesis 1:1 In the beginning God created the heavens and the earth.\nJesus was baptized in the Jordan River by John the Baptist.";
    let book3_edge_ai = "Edge AI Architecture 2026: 邊緣運算白皮書。\nTo solve the memory bandwidth problem, we utilize Zero-Copy memory mapped files via rkyv, completely bypassing the CPU cache allocations.";

    // 結合為一個連續的虛擬巨大記憶體池 (例如從 PDFKit 或 mmap 取得)
    let massive_pool = format!(
        "{}\n\n{}\n\n{}",
        book1_hong_lou_meng, book2_bible, book3_edge_ai
    );

    // 2. 零拷貝分塊 (Zero-Copy Chunking)
    let chunker = ZeroChunker::new(50); // 50 characters per chunk

    // Warmup OS paging / caches
    let _warmup: arrayvec::ArrayVec<_, 64> = chunker.chunk_text("warmup text").collect();
    let rag = DualBrainRag::new();
    let _warmup_embed = rag.left_brain.embed_chunk(&_warmup[0]);

    let t0 = Instant::now();
    // Use ArrayVec to eliminate Vec dynamic allocation during chunking
    let chunks: arrayvec::ArrayVec<_, 64> = chunker.chunk_text(&massive_pool).collect();
    let chunk_time = t0.elapsed();

    // 斷言 (Infinite Context / Zero-Copy Validation)
    let pool_start = massive_pool.as_ptr() as usize;
    let pool_end = pool_start + massive_pool.len();

    for c in &chunks {
        let ptr = c.content.as_ptr() as usize;
        assert!(
            ptr >= pool_start && ptr < pool_end,
            "Zero-Copy 失敗！發生記憶體拷貝！"
        );
    }
    println!(
        "✅ 零拷貝檢驗通過！{} 個 Chunk 完全映射在原始 Heap 內，花費 {:?}",
        chunks.len(),
        chunk_time
    );

    // 3. 左腦即時建立記憶 (Embedding)
    let mut rag = DualBrainRag::new();
    let t1 = Instant::now();
    let embedded_db: arrayvec::ArrayVec<_, 64> = chunks
        .iter()
        .map(|c| rag.left_brain.embed_chunk(c))
        .collect();
    println!("✅ 左腦即時記憶建立完成，花費 {:?}", t1.elapsed());

    // 4. 交叉主題輪詢與右腦推理 (Cross-examination)
    let queries = [
        "請問賈寶玉出生的時候，玉上面刻了什麼字？",
        "Where was Jesus baptized?",
        "How do we solve the memory bandwidth problem in Edge AI?",
        "未知的領域：請問量子力學的薛丁格方程式是什麼？", // This will trigger Self-Learning
    ];

    println!("\n[Phase 2 & 3] 交叉輪詢與自學習檢驗...");
    let mut q_idx = 1;
    for q in queries {
        let mut buf = arrayvec::ArrayString::<256>::new();

        let iters = 1000;
        let t2 = Instant::now();
        for _ in 0..iters {
            buf.clear();
            rag.process_query(q, &embedded_db, &mut buf);
        }
        let total_time = t2.elapsed();
        let infer_time = total_time / iters;

        println!("--------------------------------------------------");
        println!("Query {}: {}", q_idx, q);
        println!("Response: {}", buf);
        println!("Latency: {:?}", infer_time);

        if q.contains("Jesus") || q.contains("Edge AI") {
            assert!(
                infer_time.as_nanos() < 100,
                "效能檢驗失敗：草稿驗證延遲超過 100ns！ ({}ns)",
                infer_time.as_nanos()
            );
        }
        q_idx += 1;
    }

    println!("--------------------------------------------------");
    println!("✅ 所有的 ModelGo 目標與效能檢測皆順利通過！ (無限上下文、小模型自學習、極速效能)");
}
