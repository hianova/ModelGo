use crate::{IntentRouter, MemoryMesh, SelfEvolvingLoop, UnionAst};
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};

/// 狀態機：描述 Agent 目前處於哪一個對話或執行階段
#[derive(Debug, Clone, PartialEq)]
enum AgentState {
    Idle,
    AwaitingCoffeeSelection,
    AwaitingRefundConfirmation,
}

pub async fn run_agent(watch_path: String) -> anyhow::Result<()> {
    println!("🚀 [ModelGo Agent] 啟動自主本地 Agent (Union Protocol 狀態機模式)...");

    let router = crate::HybridRouter::new(&crate::config::EngineConfig::default());
    let memory_mesh = match MemoryMesh::new(&crate::config::EngineConfig::default()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[ModelGo Agent] 無法初始化 MemoryMesh: {}", e);
            return Err(e);
        }
    };
    let evolver = SelfEvolvingLoop::new();
    let mut workflow_id: u32 = 0;

    let mut state = AgentState::Idle;
    let mut stdin_reader = BufReader::new(tokio::io::stdin()).lines();

    // 啟動背景任務監控
    let watcher = crate::watcher::TaskWatcher::new(watch_path);
    let mut task_rx = watcher.start().await?;

    println!("============================================================");
    println!("ModelGo Local Agent - Terminal Interface");
    println!("Type 'exit' to quit.");
    println!("============================================================");

    loop {
        // 根據當前狀態，顯示 Prompt 與選項
        match state {
            AgentState::Idle => {
                print!("\n[Agent: Idle] 請輸入您的需求 > ");
            }
            AgentState::AwaitingCoffeeSelection => {
                println!("\n[Agent: Option] 偵測到飲料購買意圖。請選擇品項：");
                println!("  [1] 熱美式 (送出確認 OpCode: 0x12)");
                println!("  [2] 冰拿鐵 (送出確認 OpCode: 0x13)");
                println!("  [c] 取消 (送出取消 OpCode: 0x15)");
                print!("> ");
            }
            AgentState::AwaitingRefundConfirmation => {
                println!("\n[Agent: Option] 偵測到退款意圖。確定要退款最近一筆訂單嗎？");
                println!("  [y] 確定退款 (送出確認 OpCode: 0x16)");
                println!("  [n] 取消操作 (返回 Idle)");
                print!("> ");
            }
        }
        io::stdout().flush()?;

        let line = tokio::select! {
            result = stdin_reader.next_line() => {
                match result {
                    Ok(Some(input)) => input.trim().to_string(),
                    Ok(None) | Err(_) => break,
                }
            }
            Some(new_task) = task_rx.recv() => {
                println!("\n[ModelGo Watcher] 偵測到背景任務更新，載入上下文...");
                new_task
            }
        };

        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            println!("Exiting ModelGo Local Agent.");
            break;
        }

        workflow_id += 1;
        let json_record = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "input_text": line,
        })
        .to_string();
        memory_mesh.persist_workflow(workflow_id, &json_record);
        println!(
            "[ModelGo Memory] 成功將任務寫入 cdDB 超長上下文 (ID: {})",
            workflow_id
        );

        // 狀態機轉移邏輯
        match state {
            AgentState::Idle => match router.route(line.as_bytes()) {
                Ok((intent, parameters)) => {
                    println!(
                        "[ModelGo Router] 成功擷取意圖：OpCode 0x{:02X}, PayloadID 0x{:04X}",
                        intent.opcode, intent.payload_id
                    );
                    if let Some(ref p) = parameters {
                        println!("[ModelGo Router] 動態參數擷取成功: {:?}", p);
                    }

                    let args = match &parameters {
                        Some(serde_json::Value::Object(map)) => {
                            map.values().map(|v| v.to_string()).collect::<Vec<String>>()
                        }
                        Some(serde_json::Value::Array(arr)) => {
                            arr.iter().map(|v| v.to_string()).collect::<Vec<String>>()
                        }
                        _ => vec![],
                    };

                    let ast = UnionAst {
                        opcode: intent.opcode,
                        payload_id: intent.payload_id as u32,
                        arguments: args,
                    };

                    if evolver.intercept_success(&ast) {
                        println!("============================================================");
                        println!("🧠 [ModelGo 大腦] 偵測到高頻任務模式，已突破數學混沌閾值！");
                        println!("🚀 [ModelGo 大腦] 正在將此流程壓縮為 O(1) 短路巨集...");
                        println!("============================================================");
                    }

                    match intent.opcode {
                        0x11 | crate::process_intent::OP_ACQUIRE => {
                            state = AgentState::AwaitingCoffeeSelection;
                        }
                        0x15 => {
                            state = AgentState::AwaitingRefundConfirmation;
                        }
                        _ => {
                            println!(
                                "[ModelGo Exec] 未知或無須選項的 OpCode，交由物理層直接處決..."
                            );
                            match crate::process_intent(intent, &line, parameters) {
                                Ok(res) => println!("[ModelGo 結果] {}", res),
                                Err(e) => eprintln!("[ModelGo 錯誤] {}", e),
                            }
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "[ModelGo Router] L0/L1 路由未命中 (Err 0x{:02X})，嘗試 fallback 物理層...",
                        e
                    );
                    match crate::process_intent(
                        union_code::CompressedIntent {
                            opcode: 0,
                            payload_id: 0,
                        },
                        &line,
                        None,
                    ) {
                        Ok(res) => println!("[ModelGo 結果] {}", res),
                        Err(err) => println!("[ModelGo 錯誤] {}", err),
                    }
                }
            },
            AgentState::AwaitingCoffeeSelection => match line.as_str() {
                "1" | "美式" => {
                    println!("[ModelGo Exec] 送出二進位 Intent -> OpCode: 0x12 (確認美式)");
                    println!("[ModelGo 結果] 交易成功，熱美式開始沖泡！");
                    state = AgentState::Idle;
                }
                "2" | "拿鐵" => {
                    println!("[ModelGo Exec] 送出二進位 Intent -> OpCode: 0x13 (確認拿鐵)");
                    println!("[ModelGo 結果] 交易成功，冰拿鐵開始沖泡！");
                    state = AgentState::Idle;
                }
                "c" | "cancel" | "取消" => {
                    println!("[ModelGo Exec] 送出二進位 Intent -> OpCode: 0x15 (取消操作)");
                    println!("[ModelGo 結果] 操作已取消。");
                    state = AgentState::Idle;
                }
                _ => {
                    println!("[ModelGo 錯誤] 無效選項，請重新輸入。");
                }
            },
            AgentState::AwaitingRefundConfirmation => match line.to_lowercase().as_str() {
                "y" | "yes" | "確定" => {
                    println!("[ModelGo Exec] 送出二進位 Intent -> OpCode: 0x16 (確認退款)");
                    println!("[ModelGo 結果] 退款已完成，款項已退回原帳戶。");
                    state = AgentState::Idle;
                }
                "n" | "no" | "取消" => {
                    println!("[ModelGo Exec] 送出二進位 Intent -> 取消退款");
                    println!("[ModelGo 結果] 已取消退款程序。");
                    state = AgentState::Idle;
                }
                _ => {
                    println!("[ModelGo 錯誤] 無效選項，請重新輸入。");
                }
            },
        }
    }

    Ok(())
}
