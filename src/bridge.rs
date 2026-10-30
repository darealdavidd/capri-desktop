// Local HTTP bridge server and CLI hook forwarding.
// Listens on 127.0.0.1:47823 for Claude Code, Codex, and Antigravity hooks.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::sync::mpsc::Sender;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const DEFAULT_PORT: u16 = 47823;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub hook_event_name: String,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub session_title: Option<String>,
    pub permission_mode: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<Value>,
    pub tool_use_id: Option<String>,
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
    pub is_interrupt: Option<bool>,
    pub prompt: Option<String>,
    pub message: Option<String>,
    pub title: Option<String>,
    pub notification_type: Option<String>,
    pub last_assistant_message: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub source: Option<String>,
    pub reason: Option<String>,
    pub trigger: Option<String>,
    #[serde(default)]
    pub demo: bool,
}

pub struct BridgeServer;

impl BridgeServer {
    pub fn start(port: u16, tx: Sender<HookEvent>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let addr = format!("127.0.0.1:{port}");
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => {
                        println!("[capri] bridge listening on {addr}");
                        l
                    }
                    Err(e) => {
                        eprintln!("[capri] bridge port {port} bind error: {e}");
                        return;
                    }
                };

                loop {
                    if let Ok((mut socket, _)) = listener.accept().await {
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            let n = match socket.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };
                            let req = String::from_utf8_lossy(&buf[..n]);

                            if req.starts_with("GET /health") {
                                let body = serde_json::json!({
                                    "ok": true,
                                    "name": "capri",
                                    "pid": std::process::id()
                                }).to_string();
                                let resp = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(resp.as_bytes()).await;
                                return;
                            }

                            if req.starts_with("POST /event") {
                                if let Some(body_start) = req.find("\r\n\r\n") {
                                    let json_part = &req[body_start + 4..];
                                    if let Ok(event) = serde_json::from_str::<HookEvent>(json_part) {
                                        let _ = tx_clone.send(event);
                                        let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        return;
                                    }
                                }
                                let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                                let _ = socket.write_all(resp.as_bytes()).await;
                                return;
                            }

                            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                            let _ = socket.write_all(resp.as_bytes()).await;
                        });
                    }
                }
            });
        });
    }
}

/// Standalone CLI hook forwarder: reads stdin, slims down the payload, and sends to 127.0.0.1:47823
pub fn run_cli_hook(assistant: &str) {
    let mut stdin_data = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut stdin_data);

    if let Ok(mut val) = serde_json::from_slice::<Value>(&stdin_data) {
        // Tag session if codex or agy
        if assistant != "claude" {
            if let Some(obj) = val.as_object_mut() {
                let id = obj.get("session_id").and_then(|v| v.as_str()).unwrap_or("default");
                obj.insert("session_id".to_string(), Value::String(format!("{assistant}:{id}")));
            }
        }

        let body = serde_json::to_string(&val).unwrap_or_default();
        let port = std::env::var("CAPRI_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let req = format!(
            "POST /event HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        if let Ok(mut stream) = std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
            let _ = std::io::Write::write_all(&mut stream, req.as_bytes());
        }
    }
}
