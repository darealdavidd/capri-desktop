#![allow(dead_code)]

mod acting;
mod app;
mod brain;
mod bridge;
mod capri;
mod chip;
mod config;
mod fx;
mod lines;
mod mission;
mod panel;
mod sprite;
mod tray;
mod util;
mod voice;

use clap::{Parser, Subcommand};
use config::Config;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "capri")]
#[command(author = "mochiiwuff, david1357")]
#[command(version = "0.5.0")]
#[command(about = "Capri Desktop Mate — Anime Desktop Companion for Linux, Wayland & Tiling WMs", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Sprite scale factor (default: 0.6)")]
    scale: Option<f64>,

    #[arg(short, long, help = "Language (en, fr, es, pt, ru, auto)")]
    lang: Option<String>,

    #[arg(long, help = "Assistant to follow (claude, codex, agy)")]
    assistant: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run hook forwarder for coding assistants (reads stdin, sends to Capri)")]
    Hook {
        #[arg(default_value = "claude", help = "Assistant name (claude, codex, agy)")]
        assistant: String,
    },
    #[command(about = "Install hooks for a coding assistant")]
    InstallHooks {
        #[arg(default_value = "claude", help = "Assistant name (claude, codex, agy)")]
        assistant: String,
    },
    #[command(about = "Uninstall hooks for a coding assistant")]
    UninstallHooks {
        #[arg(default_value = "claude", help = "Assistant name (claude, codex, agy)")]
        assistant: String,
    },
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}

fn install_hooks_for(assistant: &str) -> Result<(), anyhow::Error> {
    let capri_exe = std::env::current_exe()?;
    let hook_cmd = format!("{} hook {assistant}", capri_exe.display());

    match assistant {
        "claude" => {
            let claude_dir = home_dir().join(".claude");
            fs::create_dir_all(&claude_dir)?;
            let settings_file = claude_dir.join("settings.json");

            let mut val: serde_json::Value = if settings_file.exists() {
                let data = fs::read_to_string(&settings_file)?;
                serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            let hook_entry = serde_json::json!({
                "type": "command",
                "command": hook_cmd
            });

            let events = [
                "SessionStart",
                "UserPromptSubmit",
                "PreToolUse",
                "PostToolUseFailure",
                "Notification",
                "Stop",
                "Interrupt",
                "PreCompact",
                "SubagentStart",
                "SubagentStop",
                "SessionEnd",
            ];

            let hooks_obj = val
                .as_object_mut()
                .unwrap()
                .entry("hooks")
                .or_insert_with(|| serde_json::json!({}));

            if let Some(h) = hooks_obj.as_object_mut() {
                for evt in events {
                    let arr = h.entry(evt).or_insert_with(|| serde_json::json!([]));
                    if let Some(a) = arr.as_array_mut() {
                        let already_present = a.iter().any(|item| {
                            item.get("command").and_then(|c| c.as_str()).map_or(false, |c| c.contains("capri hook"))
                        });
                        if !already_present {
                            a.push(hook_entry.clone());
                        }
                    }
                }
            }

            fs::write(settings_file, serde_json::to_string_pretty(&val)?)?;
            println!("Installed Claude Code hooks successfully!");
        }
        "codex" => {
            let codex_dir = home_dir().join(".codex");
            fs::create_dir_all(&codex_dir)?;
            let hooks_file = codex_dir.join("hooks.json");
            let hook_obj = serde_json::json!({
                "hooks": {
                    "all": [{ "command": hook_cmd }]
                }
            });
            fs::write(hooks_file, serde_json::to_string_pretty(&hook_obj)?)?;
            println!("Installed Codex hooks successfully!");
        }
        "agy" => {
            let agy_dir = home_dir().join(".gemini").join("config");
            fs::create_dir_all(&agy_dir)?;
            let hooks_file = agy_dir.join("hooks.json");
            let hook_obj = serde_json::json!({
                "hooks": {
                    "all": [{ "command": hook_cmd }]
                }
            });
            fs::write(hooks_file, serde_json::to_string_pretty(&hook_obj)?)?;
            println!("Installed Antigravity hooks successfully!");
        }
        _ => eprintln!("Unknown assistant: {assistant}. Use claude, codex, or agy."),
    }
    Ok(())
}

fn uninstall_hooks_for(assistant: &str) -> Result<(), anyhow::Error> {
    match assistant {
        "claude" => {
            let settings_file = home_dir().join(".claude").join("settings.json");
            if settings_file.exists() {
                let data = fs::read_to_string(&settings_file)?;
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&data) {
                    if let Some(hooks_obj) = val.get_mut("hooks").and_then(|h| h.as_object_mut()) {
                        for (_, arr_val) in hooks_obj.iter_mut() {
                            if let Some(arr) = arr_val.as_array_mut() {
                                arr.retain(|item| {
                                    item.get("command").and_then(|c| c.as_str()).map_or(true, |c| !c.contains("capri hook"))
                                });
                            }
                        }
                    }
                    fs::write(settings_file, serde_json::to_string_pretty(&val)?)?;
                    println!("Uninstalled Claude Code hooks successfully.");
                }
            }
        }
        "codex" => {
            let hooks_file = home_dir().join(".codex").join("hooks.json");
            if hooks_file.exists() {
                let _ = fs::remove_file(hooks_file);
                println!("Uninstalled Codex hooks.");
            }
        }
        "agy" => {
            let hooks_file = home_dir().join(".gemini").join("config").join("hooks.json");
            if hooks_file.exists() {
                let _ = fs::remove_file(hooks_file);
                println!("Uninstalled Antigravity hooks.");
            }
        }
        _ => eprintln!("Unknown assistant: {assistant}"),
    }
    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Hook { assistant } => {
                bridge::run_cli_hook(&assistant);
                return Ok(());
            }
            Commands::InstallHooks { assistant } => {
                return install_hooks_for(&assistant);
            }
            Commands::UninstallHooks { assistant } => {
                return uninstall_hooks_for(&assistant);
            }
        }
    }

    // Load config and apply CLI overrides
    let mut config = Config::load();
    if let Some(s) = cli.scale {
        config.scale = s;
    }
    if let Some(l) = cli.lang {
        config.lang = l;
    }
    if let Some(a) = cli.assistant {
        config.assistant = a;
    }

    println!("Starting Capri Desktop Mate v0.5.0 (Rust / Wayland)");
    app::run(config)?;
    Ok(())
}
