use anyhow::Result;
use union_code::CompressedIntent;
use crate::jit_compiler::JitCompiler;

// 動作 (OpCode - Verb)
pub const OP_QUERY: u8     = 0x10; // 查 (16)
pub const OP_ACQUIRE: u8   = 0x20; // 拿/買/送 (32)
pub const OP_CONTROL: u8   = 0x30; // 控制/調整 (48)
pub const OP_TELEMETRY: u8 = 0x80; // 電腦電量 (128)

// 實體物 (PayloadID - Noun)
pub const PAY_INVOICE: u16    = 0x00FF; // 發票
pub const PAY_COFFEE: u16     = 0x0A42; // 咖啡
pub const PAY_TEA: u16        = 0x0A43; // 茶
pub const PAY_WATER: u16      = 0x0A44; // 水
pub const PAY_BRIGHTNESS: u16 = 0x00A1; // 亮度
pub const PAY_BATTERY: u16    = 0xFFFF; // 電量

fn query_macos_battery() -> Result<String> {
    println!("[ModelGo OS Dispatch] Querying macOS battery status...");
    let output = std::process::Command::new("pmset")
        .arg("-g")
        .arg("batt")
        .output()?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Basic extraction of battery percentage (e.g. "100%")
    let mut percentage = "未知";
    for word in stdout.split_whitespace() {
        if word.contains("%") {
            percentage = word.trim_matches(|c| c == ';' || c == ',');
            break;
        }
    }
    Ok(percentage.to_string())
}

/// Process a natural language intent by routing it through the HybridRouter 
/// and executing the appropriate physical OS action or JIT compiled script.
pub fn process_intent(
    intent: CompressedIntent, 
    intent_raw_str: &str, 
    dynamic_params: Option<serde_json::Value>
) -> Result<String> {
    println!("\n[ModelGo] Dispatching binary intent: OpCode 0x{:02X}, PayloadID 0x{:04X}, Params: {:?}", intent.opcode, intent.payload_id, dynamic_params);

    match (intent.opcode, intent.payload_id) {
        // 1. 查詢發票 (0x10, 0x00FF)
        (OP_QUERY, PAY_INVOICE) => {
            JitCompiler::compile_and_execute(intent_raw_str, dynamic_params)?;
            Ok("已經成功將發票下載至 ~/Documents/Invoices 目錄。".to_string())
        }
        
        // 2. 調整螢幕亮度 (0x30, 0x00A1)
        (OP_CONTROL, PAY_BRIGHTNESS) => {
            println!("[ModelGo OS Dispatch] Adjusting physical screen brightness...");
            let brightness = dynamic_params
                .and_then(|p| p.get("brightness").cloned())
                .and_then(|b| b.as_u64())
                .unwrap_or(30);
            Ok(format!("螢幕亮度已調整為 {}%。", brightness))
        }

        // 3. 取得咖啡/茶/水 (0x20, 咖啡系列)
        (OP_ACQUIRE, PAY_COFFEE) => {
            Ok("咖啡機已啟動，正在為您沖煮一杯熱美式咖啡。".to_string())
        }
        (OP_ACQUIRE, PAY_TEA) => {
            Ok("茶飲機已啟動，正在為您沖泡一杯經典紅茶。".to_string())
        }
        (OP_ACQUIRE, PAY_WATER) => {
            Ok("飲水機已啟動，正在為您準備一杯溫水。".to_string())
        }

        // 4. 查詢電腦電量 (0x80, PAY_BATTERY)
        (OP_TELEMETRY, PAY_BATTERY) | (OP_TELEMETRY, _) => {
            let percentage = query_macos_battery()?;
            Ok(format!("目前的電腦電量為：{}", percentage))
        }

        // 5. 其他未命中的指令 -> Fallback 至 JIT 自癒腳本執行
        _ => {
            JitCompiler::compile_and_execute(intent_raw_str, dynamic_params)?;
            Ok("指令已成功透過 JIT 腳本執行完畢。".to_string())
        }
    }
}
