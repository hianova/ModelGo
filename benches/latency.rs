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

use model_go::chaos_state::{ChaosState, MicroTweak, RngState, step_forward_nd};

fn bench_chaos_learning_step(c: &mut Criterion) {
    let mut rng = RngState::new(0x1234);
    let mut state = ChaosState::<10, 1>::new([0.0]);
    let tweak = MicroTweak {
        s_exponent: 1.5,
        max_elements: 1000,
    };

    c.bench_function("chaos_learning_step", |b| {
        b.iter(|| {
            // Benchmark the O(N) pure math projection (no allocations)
            state = step_forward_nd(black_box(&state), black_box(&tweak), black_box(&mut rng));
        })
    });
}

// NOTE: bench_router_fast_path is intentionally excluded from the criterion group.
// HybridRouter::route() falls through to Vec101FallbackEngine which spawns a Python
// subprocess at ../vec101/. This path doesn't exist in most environments, causing
// the benchmark to hang indefinitely. To benchmark the full router, ensure the
// vec101 Python bridge is available and uncomment the function below.
//
// fn bench_router_fast_path(c: &mut Criterion) {
//     let router = HybridRouter::new();
//     c.bench_function("hybrid_router_miss", |b| {
//         b.iter(|| {
//             let _ = router.route(black_box(b"unknown"));
//         })
//     });
// }

criterion_group!(benches, bench_rejection_sampling, bench_memory_mesh_cache, bench_chaos_learning_step);
criterion_main!(benches);
