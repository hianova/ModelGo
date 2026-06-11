# ModelGo Test & Benchmark 審計報告

## 總結

> [!CAUTION]
> **整個 crate 沒有任何一個 `#[test]` 或 `#[bench]`**。`cargo test` 結果：`running 0 tests`。所有「測試」都包裝在一個手動呼叫的 `BenchmarkSuite` struct 裡，是透過 `cargo run -- benchmark` 跑的 CLI 子命令，**完全不受 Rust 測試框架管理**。

---

## 1. 當前狀態掃描

| 指標 | 結果 |
|------|------|
| `#[test]` 函數數量 | **0** |
| `#[bench]` 函數數量 | **0** |
| `#[cfg(test)]` 模組數量 | **0** |
| Doc-test 數量 | **0** |
| `cargo test` 通過的測試 | **0 passed; 0 failed** |
| 實際「測試」程式碼 | 全部在 [benchmarks.rs](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs)，手動 `println!` 判斷 |

---

## 2. 四個 Benchmark 的逐項審計

### Test 1：Cold Start Assassin（TTFT < 10ms）

**檔案**：[benchmarks.rs:29-57](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs#L29-L57)

| 項目 | 評估 |
|------|------|
| 量測方法 | ✅ `Instant::now()` → `elapsed()` 有效 |
| 測試對象 | ⚠️ 只 mmap 了一個 50MB 暫存檔，不是真正的 3GB 模型 |
| 失敗處理 | ❌ 超時只 `println!("[FAIL]")`，**不會 panic 或回傳 Err** |
| 清理 | ✅ 刪除暫存檔 |
| 可重複性 | ⚠️ 依賴 OS page cache 暖度，第二次跑一定更快 |

> [!WARNING]
> 用 50MB 檔案來代表「3GB 模型載入」不具說服力。mmap 的 `map()` 只建立虛擬記憶體對映，不觸發 I/O；真正的延遲在第一次 page fault 時才會發生，但這裡從未讀取 mmap 的內容。

---

### Test 2：Violent Introspection（Rejection Sampling < 200ms）

**檔案**：[benchmarks.rs:61-87](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs#L61-L87)

| 項目 | 評估 |
|------|------|
| 量測方法 | ❌ **嚴重問題**：先手動 `thread::sleep(60ms × 3 = 180ms)`，再呼叫 `execute_with_rejection_sampling`，所以至少 180ms 是人工灌水的 |
| 測試對象 | ⚠️ `System2Verifier` 內部用硬編碼的 mock 回應（attempt 1 = 語法錯、attempt 2 = opcode 0、attempt 3 = 正確） |
| 失敗處理 | ❌ 超時只印 `[FAIL]`，程式繼續跑 |
| 真實性 | ❌ 沒有真正呼叫 vec101 LLM，所有回應都是 `if attempt == N` 的硬編碼字串 |

> [!CAUTION]
> 這個測試同時量測了自己注入的 `thread::sleep(180ms)` 加上 verifier 的時間，然後驗證總時間 < 200ms。這意味著只有 ~20ms 的容錯空間，但這 180ms 本身就是假的「模擬延遲」，不是真正的推理延遲。實際跑出 195ms 幾乎就快超標了。

---

### Test 3：O(1) Self-Learning（DualCacheFF Hit < 1ms）

**檔案**：[benchmarks.rs:91-121](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs#L91-L121)

| 項目 | 評估 |
|------|------|
| Cache Miss 量測 | ❌ 又是 `thread::sleep(150ms)` 模擬 LLM 延遲 |
| Cache Hit 量測 | ❌ **完全是假的**：L107-110 只是把字串賦值給 `_hit`，根本沒有呼叫 `cache.get()`。量測的是一個 `let` 賦值的時間（0.000000ms），而非真正的快取查詢 |
| 失敗處理 | ❌ 同上，只印不 panic |

> [!CAUTION]
> **最嚴重的問題**：Cache Hit 的量測完全沒有觸碰到 `DualCacheFF`。程式碼註解甚至承認了：*"Since we don't expose the underlying DualCache lookup publicly in our mock, we simulate the 88ns Wait-Free lookup."* 但「模擬」的方式是直接用一個局部變數，0 開銷。**這不是測試，是自欺欺人。**

---

### Test 4：False Miracles（cdDB Prefill < 5ms）

**檔案**：[benchmarks.rs:126-146](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs#L126-L146)

| 項目 | 評估 |
|------|------|
| 量測方法 | ❌ 只有 `thread::sleep(3ms)`，完全沒有呼叫 cdDB 的任何 API |
| 測試對象 | ❌ 沒有載入 PDF，沒有 KV Cache 注入，沒有 mmap pointer swap |
| 失敗處理 | ❌ 同上 |

> [!WARNING]
> 整段程式碼只做了一件事：`sleep(3ms)` 然後檢查 `< 5ms`。**這永遠會通過**。

---

## 3. 系統性問題彙整

### 架構層

| 問題 | 嚴重程度 | 說明 |
|------|----------|------|
| 零 `#[test]` 函數 | 🔴 Critical | CI/CD 無法攔截任何回歸 |
| 零 `#[bench]` 函數 | 🔴 Critical | 無法使用 `cargo bench` 或 criterion 追蹤效能回歸 |
| 失敗不 panic | 🔴 Critical | 即使所有 benchmark 都失敗，程式仍回傳 `Ok(())`，exit code = 0 |
| `thread::sleep` 灌水 | 🔴 Critical | 4 個 benchmark 中有 3 個用 sleep 來模擬延遲，讓量測完全失去意義 |
| Cache Hit 從未呼叫快取 | 🔴 Critical | Test 3 宣稱驗證 DualCacheFF，但從未呼叫其 API |
| cdDB 從未被呼叫 | 🟠 High | Test 4 宣稱驗證 cdDB context injection，但程式碼裡沒有任何 cdDB 操作 |
| Mock 硬編碼 | 🟡 Medium | System2Verifier 的 retry 行為是用 `if attempt == N` 硬編碼，不具泛化性 |

### 程式碼品質

| 問題 | 位置 |
|------|------|
| `main.rs` 中 `MemoryMesh` 未使用 import | [main.rs:4](file:///Users/kuangtalin/Documents/ModelGo/src/main.rs#L4) |
| crate 名稱 `ModelGo` 不符合 snake_case 慣例 | [Cargo.toml:2](file:///Users/kuangtalin/Documents/ModelGo/Cargo.toml#L2) |
| `benchmarks.rs` 中 `env`、`thread` 等模組沒有被用在正確的測試框架中 | [benchmarks.rs:1-6](file:///Users/kuangtalin/Documents/ModelGo/src/benchmarks.rs#L1-L6) |

---

## 4. 真實 Benchmark 執行結果

最後一次 `cargo run -- benchmark` 的結果：

| Test | 量測值 | 門檻 | 結果 | 可信度 |
|------|--------|------|------|--------|
| Cold Start TTFT | 0.105 ms | < 10ms | PASS | ⚠️ 中（50MB ≠ 3GB） |
| Rejection Sampling | 195.416 ms | < 200ms | PASS | ❌ 極低（180ms 是 sleep） |
| DualCacheFF Hit | 0.000000 ms | < 1ms | PASS | ❌ 無效（未呼叫快取） |
| cdDB Prefill | 3.761 ms | < 5ms | PASS | ❌ 無效（只有 sleep(3ms)） |

---

## 5. 修正建議

### 5.1 立即修正（P0）

1. **為每個模組新增 `#[cfg(test)] mod tests {}`**，至少覆蓋：
   - `System2Verifier::parse_and_verify` — 正確 JSON、語法錯誤 JSON、opcode=0
   - `ZeroCopyMmapReader::new` — 正常檔案、不存在的檔案、空檔案
   - `HybridRouter::route` — L0 命中、L0 miss fallback L1、未知 intent
   - `MemoryMesh::cache_intent_success` + 真正的 cache lookup
   - `SelfEvolvingLoop::intercept_success` — 驗證 3 次後產生 macro

2. **Benchmark 改用 [criterion](https://crates.io/crates/criterion)**：
   ```toml
   [dev-dependencies]
   criterion = { version = "0.5", features = ["html_reports"] }

   [[bench]]
   name = "latency"
   harness = false
   ```

3. **所有失敗必須 panic 或回傳 Err**，確保 CI 能攔截：
   ```rust
   assert!(elapsed.as_millis() < 10, "TTFT exceeded 10ms: {:?}", elapsed);
   ```

### 5.2 短期改善（P1）

4. **移除所有 `thread::sleep` 灌水**，用真正的元件呼叫取代
5. **Test 3 必須呼叫 `cache.get()`** 來量測真實的快取查詢延遲
6. **Test 4 必須建立真正的 cdDB 資料**，量測 mmap pointer swap 的時間
7. **為 `JitCompiler` 新增安全測試**：確保不會執行惡意 shell 指令

### 5.3 長期強化（P2）

8. 新增 CI pipeline（GitHub Actions），在每次 PR 時自動跑 `cargo test` + `cargo bench`
9. 為 `mmap_reader` 新增 fuzz testing（使用 `cargo-fuzz`）
10. 為所有 public API 新增 doc-test（`///` 範例程式碼）
11. 考慮使用 `proptest` 對 `System2Verifier::parse_and_verify` 做 property-based testing

---

## 6. 建議的測試範例

以下是 `System2Verifier` 應有的最小測試模組：

```rust
// 在 system2_verifier.rs 底部新增
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_ast() {
        let json = r#"{"opcode": 32, "payload_id": 1337, "arguments": ["test"]}"#;
        let ast = System2Verifier::parse_and_verify(json).unwrap();
        assert_eq!(ast.opcode, 32);
        assert_eq!(ast.payload_id, 1337);
        assert_eq!(ast.arguments, vec!["test"]);
    }

    #[test]
    fn parse_rejects_opcode_zero() {
        let json = r#"{"opcode": 0, "payload_id": 1337, "arguments": []}"#;
        assert!(System2Verifier::parse_and_verify(json).is_err());
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let json = r#"{"opcode": 32, "payload_id": 1337, "arguments": [test]}"#;
        assert!(System2Verifier::parse_and_verify(json).is_err());
    }

    #[test]
    fn rejection_sampling_succeeds_within_retries() {
        let result = System2Verifier::execute_with_rejection_sampling("test prompt", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn rejection_sampling_fails_with_zero_retries() {
        let result = System2Verifier::execute_with_rejection_sampling("test prompt", 0);
        assert!(result.is_err());
    }
}
```
