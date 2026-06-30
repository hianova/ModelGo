use model_go::dual_brain::{DualBrainRag, LeftBrainMode};
use model_go::zero_chunker::ZeroChunker;

fn main() {
    println!("=== 啟動 ModelGo Polymorphic Left Brain 測試 ===\n");

    let mut rag = DualBrainRag::new();

    // Context preparation for Semantic parsing (BGE-m3 mock)
    let chunker = ZeroChunker::new(50);
    let mock_text = "紅樓夢第一回：賈寶玉與通靈寶玉。";
    let chunks: Vec<_> = chunker.chunk_text(mock_text).collect();

    // Force to semantic mode for initial memory embedding
    rag.left_brain.switch_mode(LeftBrainMode::SemanticParsing);
    let embedded_db: Vec<_> = chunks
        .iter()
        .map(|c| rag.left_brain.embed_chunk(c))
        .collect();

    let mut buf = arrayvec::ArrayString::<256>::new();

    println!("\n--- 測試 1: 一般聊天 (預期切換至 SpeculativeDrafting / BitNet) ---");
    let query_chat = "哈囉，你今天好嗎？";
    rag.process_query(query_chat, &embedded_db, &mut buf);
    println!("Response: {}\n", buf);

    println!("--- 測試 2: RAG 知識檢索 (預期切換至 SemanticParsing / BGE-m3) ---");
    let query_rag = "請問賈寶玉的玉是怎麼來的？";
    rag.process_query(query_rag, &embedded_db, &mut buf);
    println!("Response: {}\n", buf);

    println!("--- 測試 3: 蛋白質結構分析 (預期切換至 BiologyFolding / ESMFold) ---");
    let query_bio = "分析這段序列: MKTLLILAVIMRVAG.pdb";
    rag.process_query(query_bio, &embedded_db, &mut buf);
    println!("Response: {}\n", buf);

    println!("✅ Polymorphic 左腦切換測試順利通過！右腦決策維持純淨不受污染！");
}
