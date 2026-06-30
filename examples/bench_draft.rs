use model_go::dual_brain::DualBrainRag;
use std::hint::black_box;

fn main() {
    let mut rag = DualBrainRag::new();
    let query = "How do we solve the memory bandwidth problem in Edge AI?";

    // We want to run this in a tight loop to allow samply to profile it
    let iterations = 10_000_000;

    let mut buf = arrayvec::ArrayString::<256>::new();
    for _ in 0..iterations {
        buf.clear();
        rag.process_query(black_box(query), &[], &mut buf);
        black_box(&buf);
    }
}
