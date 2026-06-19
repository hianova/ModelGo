use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use std::fs;

/// TaskWatcher 負責監控指定的 Markdown 檔案，當發生變更時，
/// 比對前後內容，並將新增的文字透過 Channel 發送出來。
pub struct TaskWatcher {
    target_file: PathBuf,
    last_content: String,
}

impl TaskWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let initial_content = fs::read_to_string(&path_buf).unwrap_or_default();
        Self {
            target_file: path_buf,
            last_content: initial_content,
        }
    }

    /// 開始監聽，並回傳一個 Receiver，可從中讀取到新增的任務文本
    pub async fn start(mut self) -> anyhow::Result<mpsc::Receiver<String>> {
        let (tx, rx) = mpsc::channel(100);
        let (notify_tx, mut notify_rx) = mpsc::channel(100);

        // notify v6 的 event handler
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = notify_tx.blocking_send(event);
            }
        })?;

        watcher.watch(&self.target_file, RecursiveMode::NonRecursive)?;

        // 在背景執行比對邏輯
        tokio::spawn(async move {
            // 需要保持 watcher 存活，否則離開 scope 會停止監聽
            let _kept_watcher = watcher;
            println!("[TaskWatcher] 開始監控檔案: {:?}", self.target_file);

            while let Some(event) = notify_rx.recv().await {
                // 我們主要關心 Data 寫入或 Metadata 變更事件
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Access(_) => {
                        // 稍微等待一點時間，讓編輯器寫入完成，避免讀到空檔案
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        
                        if let Ok(current_content) = fs::read_to_string(&self.target_file) {
                            if current_content != self.last_content {
                                let new_text = Self::extract_new_content(&self.last_content, &current_content);
                                if !new_text.is_empty() {
                                    println!("[TaskWatcher] 偵測到新任務內容: {}", new_text);
                                    if tx.send(new_text).await.is_err() {
                                        eprintln!("[TaskWatcher] 無法發送新任務內容 (Receiver 已關閉)");
                                        break;
                                    }
                                }
                                self.last_content = current_content;
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    /// 比較舊內容與新內容，簡單的回傳新增加在結尾的字串
    fn extract_new_content(old: &str, new: &str) -> String {
        if new.starts_with(old) && new.len() > old.len() {
            new[old.len()..].trim().to_string()
        } else {
            // 發生了較大範圍的修改，直接回傳整段
            new.trim().to_string()
        }
    }
}
