pub mod benchmarks;
pub mod biology_router;
pub mod chaos_state;
pub mod config;

pub mod dual_brain;
pub mod ffi;
pub mod fft_chaos;
pub mod jit_compiler;
pub mod loader;

pub mod markdown_diffusion;
pub mod memory_mesh;
pub mod mmap_reader;

pub mod os_dispatch;

pub mod pdf_kit;
pub mod process_intent;
pub mod router;
pub mod self_evolving_loop;
pub mod spatial;
pub mod speculative_engine;
pub mod system2_verifier;
pub mod temporal_filter;

pub mod video_toolbox;
pub mod visual_cache;

pub mod zero_chunker;

pub mod science;

pub mod tiered_kv;
pub use benchmarks::BenchmarkSuite;
pub use fft_chaos::FftChaosAnalyzer;
pub use jit_compiler::JitCompiler;
pub use memory_mesh::MemoryMesh;
pub use mmap_reader::ZeroCopyMmapReader;

pub use os_dispatch::OsDispatch;
pub use process_intent::process_intent;
pub use router::{HybridRouter, IntentRouter, UnionCodeEngine, Vec101FallbackEngine};
pub use self_evolving_loop::SelfEvolvingLoop;
pub use spatial::{BoundingBox, SpatialIntentEngine};
pub use speculative_engine::SpeculativeEngine;
pub use system2_verifier::{System2Verifier, UnionAst};

pub mod decision_engine;
pub use decision_engine::{DecisionEngine, DecisionResult, ExternalDataSource, RiskRating};

pub mod engine;
pub use engine::Vec101Engine;


pub mod agent_cli;

pub mod daemon;
pub mod watcher;

pub mod universal_solver;
pub mod universal_tensor;
pub use universal_solver::{ObjectiveFunction, UniversalOptimizationEngine};
pub use universal_tensor::UniversalTensor;
