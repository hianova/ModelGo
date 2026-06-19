pub mod config;
pub mod loader;
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

pub mod science;

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

pub mod daemon;
pub mod watcher;
pub mod agent_cli;

pub mod universal_tensor;
pub mod universal_solver;
pub use universal_tensor::UniversalTensor;
pub use universal_solver::{ObjectiveFunction, UniversalOptimizationEngine};
