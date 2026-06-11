use criterion::{criterion_group, criterion_main, Criterion, black_box};
use model_go::{MemoryMesh, System2Verifier, HybridRouter, IntentRouter};

fn bench_rejection_sampling(c: &mut Criterion) {
    c.bench_function("rejection_sampling", |b| {
        b.iter(|| {
            // Note: Since we removed the sleep, this will just bench the raw parser + loop overhead
            let _ = System2Verifier::execute_with_rejection_sampling(black_box("Generate JSON"), 3);
        })
    });
}

fn bench_memory_mesh_cache(c: &mut Criterion) {
    let mesh = MemoryMesh::new().unwrap();
    let hash = 0x12345;
    mesh.cache_intent_success(hash, "test state".to_string());
    
    c.bench_function("dualcache_ff_hit", |b| {
        b.iter(|| {
            // Wait-free static cache lookup benchmark
            let _ = mesh.get_cached_intent(black_box(hash));
        })
    });
}

fn bench_router_fast_path(c: &mut Criterion) {
    let router = HybridRouter::new();
    // This will hit the fallback path since static cache is empty
    c.bench_function("hybrid_router_miss", |b| {
        b.iter(|| {
            let _ = router.route(black_box(b"unknown"));
        })
    });
}

criterion_group!(benches, bench_rejection_sampling, bench_memory_mesh_cache, bench_router_fast_path);
criterion_main!(benches);
