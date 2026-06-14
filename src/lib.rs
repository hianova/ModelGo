pub mod mmap_reader;
pub mod router;
pub mod jit_compiler;
pub mod spatial;
pub mod memory_mesh;
pub mod system2_verifier;
pub mod self_evolving_loop;
pub mod os_dispatch;
pub mod benchmarks;
pub mod process_intent;
pub mod chaos_state;
pub mod ffi;
pub mod biology_router;
pub mod fft_chaos;
pub mod speculative_engine;

pub mod tiered_kv;
pub use speculative_engine::SpeculativeEngine;
pub use mmap_reader::ZeroCopyMmapReader;
pub use router::{IntentRouter, UnionCodeEngine, Vec101FallbackEngine, HybridRouter};
pub use jit_compiler::JitCompiler;
pub use spatial::{BoundingBox, SpatialIntentEngine};
pub use memory_mesh::MemoryMesh;
pub use system2_verifier::{System2Verifier, UnionAst};
pub use self_evolving_loop::SelfEvolvingLoop;
pub use os_dispatch::OsDispatch;
pub use benchmarks::BenchmarkSuite;
pub use process_intent::process_intent;
pub use fft_chaos::FftChaosAnalyzer;

pub mod decision_engine;
pub use decision_engine::{DecisionEngine, DecisionResult, RiskRating, ExternalDataSource};

pub mod engine;
pub use engine::Vec101Engine;

