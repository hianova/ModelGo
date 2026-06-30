use crate::self_evolving_loop::SelfEvolvingLoop;
use crate::system2_verifier::UnionAst;
use UnionTranslator::esm2::Esm2Model;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref EVOLVER: SelfEvolvingLoop = SelfEvolvingLoop::new();
    static ref MACRO_CACHE: Mutex<HashMap<String, Vec<Vec<f32>>>> = Mutex::new(HashMap::new());
    // Load Esm2Model once since it's expensive. Path adjusted for ModelGo running from workspace or GENESIS dir.
    static ref ESM2_ENGINE: Mutex<Esm2Model> = Mutex::new(Esm2Model::load_158b("../GENESIS/data/esm2_8m.158b").or_else(|_| Esm2Model::load_158b("data/esm2_8m.158b")).expect("Failed to initialize ESM-2 Engine"));
}

pub struct BiologyRouter;

impl BiologyRouter {
    /// Predicts the 3D structure of a protein sequence using O(1) Macro Bypass or O(N) ESM-2
    pub fn predict_structure(token_ids: &[usize]) -> anyhow::Result<Vec<Vec<f32>>> {
        let path_key = format!("{:?}", token_ids);

        // [O(1) Bypass]
        {
            let cache = MACRO_CACHE.lock().unwrap();
            if let Some(cached_result) = cache.get(&path_key) {
                println!(
                    "[ModelGo::BiologyRouter] ⚡ O(1) Macro Bypass Triggered! Returning cached result instantly."
                );
                return Ok(cached_result.clone());
            }
        }

        // [O(N) Compute]
        println!(
            "[ModelGo::BiologyRouter] Cache Miss. Forwarding to System 2 (UnionTranslator)..."
        );
        let result = {
            let engine = ESM2_ENGINE.lock().unwrap();
            engine.forward(token_ids)?
        };

        // [Self-Evolving]
        // Create an AST representing this biological query
        let ast = UnionAst {
            opcode: 158,
            payload_id: token_ids.iter().sum::<usize>() as u32,
            arguments: vec![path_key.clone()],
        };

        if EVOLVER.intercept_success(&ast) {
            println!(
                "[ModelGo::BiologyRouter] 📦 Macro discovered! Caching the O(1) Result forever."
            );
            let mut cache = MACRO_CACHE.lock().unwrap();
            cache.insert(path_key, result.clone());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biology_router_cache_hit() {
        let token_ids = vec![1, 2, 3];
        let path_key = format!("{:?}", token_ids);

        let expected_result = vec![vec![1.0, 2.0]];
        {
            let mut cache = MACRO_CACHE.lock().unwrap();
            cache.insert(path_key, expected_result.clone());
        }

        let result = BiologyRouter::predict_structure(&token_ids).unwrap();
        assert_eq!(result, expected_result);
    }
}
