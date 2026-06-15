# Implementation Plan: Removing Compromises, Mocks, and Excessive Coupling

This plan details the changes to remove all mock simulations, hardcoded checks, and fake delays in the `ModelGo` engine and background daemon, ensuring a production-ready, high-fidelity integration with `cdDB`, `DualCache-FF`, and the `vec101` compute engine.

## User Review Required

> [!IMPORTANT]
> - **Real macOS Thermal Monitoring:** We will replace the simulated workload-based CPU temperature (`simulated_temp` / `update_temp`) in the background daemon with a real macOS `NSProcessInfo` thermalState check via direct `Foundation` framework FFI. This queries the actual OS-level thermal pressure (Nominal, Fair, Serious, Critical) without adding external dependencies.
> - **Dynamic File Metadata Extraction:** The background daemon's classification and tagging step will no longer hardcode "Supplier A" or "contract". We will implement a dynamic metadata parser that reads the extracted text (from PDF/Excel) to extract the vendor, document type, and dates dynamically. We will also run the actual 1.58-bit model (`vec101_compute`) on the preview text during background processing.
> - **Real cdDB Metadata Lookup:** In `query_with_page_fault`, we will replace the hardcoded `if query.contains("Supplier A")` check. Instead, we will scan the `./data` directory, query `cdDB`'s `workflows` partition for each file's metadata, and determine if any document matching the query needs its KV Cache computed.
> - **Physical 4-bit KV Compute on Page Fault:** During a Page Fault, we will run actual `vec101_compute` on the loaded 4-bit safetensors model weights to simulate/execute physical KV block computation, insert the computed blocks, and dynamically persist the updated metadata (`status: "processed"`) back to `cdDB`.

## Open Questions

None. The proposed changes resolve all remaining simulated features and replace them with real, clean integrations.

---

## Proposed Changes

### ModelGo

#### [MODIFY] [memory_mesh.rs](file:///Users/kuangtalin/Documents/ModelGo/src/memory_mesh.rs)
- Implement `get_workflow` to fetch a stored workflow string from `cdDB`'s `workflows` partition by `entity_id`.

#### [MODIFY] [daemon.rs](file:///Users/kuangtalin/Documents/ModelGo/src/daemon.rs)
- Add Objective-C FFI bindings to load `NSProcessInfo` and retrieve the real `thermalState`.
- Modify `HeuristicsScheduler::is_safe_to_run` to check the real thermal state (pause if state is Serious or Critical) instead of using `simulated_temp`.
- Implement a real heuristic parser to scan text extracted from documents for vendors, document types, and dates.
- Trigger the real 1.58-bit fallback model's parallel generation pass during background processing to execute actual neural compute.

#### [MODIFY] [engine.rs](file:///Users/kuangtalin/Documents/ModelGo/src/engine.rs)
- Update `query_with_page_fault` to:
  - Scan files in `./data/`.
  - Hash file names and query their metadata in `cdDB` using `get_workflow`.
  - If a file matches the query keywords and its status is `"unprocessed"`, trigger a Page Fault.
  - Run the actual `vec101_compute` using the loaded 4-bit safetensors weights.
  - Insert the block into `TieredKVCache` and save the updated `"processed"` metadata back to `cdDB`.

---

## Verification Plan

### Automated Tests
- Run `cargo test` to ensure all 22 tests pass.
- Run `cargo run --bin ModelGo -- query "Supplier A"` to verify that it scans `./data/`, triggers a Page Fault, computes the KV Cache, updates `cdDB`, and successfully executes a subsequent cache hit on the second run.

### Manual Verification
- Verify the background daemon successfully monitors `./data/` and parses PDF/Excel text dynamically.
- Check the terminal logs to ensure that the macOS thermal state is queried successfully (Nominal/Fair/Serious/Critical).
