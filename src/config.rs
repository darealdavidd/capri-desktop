use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default = "default_volume")]
    pub volume: f64,
    #[serde(default = "default_true")]
    pub wander: bool,
    #[serde(default = "default_true")]
    pub possess: bool,
    #[serde(default = "default_rest")]
    pub rest: String,
    #[serde(default = "default_motion")]
    pub motion: String,
    #[serde(default = "default_assistant")]
    pub assistant: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub chat_model: Option<String>,
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub persona: String,
    #[serde(default)]
    pub my_lines: Vec<String>,
    #[serde(default)]
    pub memories: Vec<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
}

fn default_lang() -> String {
    "auto".to_string()
}
fn default_scale() -> f64 {
    0.6
}
fn default_volume() -> f64 {
    0.7
}
fn default_true() -> bool {
    true
}
fn default_rest() -> String {
    "corner".to_string()
}
fn default_motion() -> String {
    "auto".to_string()
}
fn default_assistant() -> String {
    "claude".to_string()
}
fn default_provider() -> String {
    "ollama".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lang: default_lang(),
            scale: default_scale(),
            muted: false,
            volume: default_volume(),
            wander: true,
            possess: true,
            rest: default_rest(),
            motion: default_motion(),
            assistant: default_assistant(),
            provider: default_provider(),
            chat_model: None,
            user_name: String::new(),
            persona: String::new(),
            my_lines: Vec::new(),
            memories: Vec::new(),
            visible: true,
            api_keys: HashMap::new(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("capri")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&data) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let data = serde_json::to_string_pretty(self)?;
        fs::write(Self::config_path(), data)?;
        Ok(())
    }

    pub fn resolved_lang(&self) -> String {
        if self.lang == "auto" {
            // Check system LANG env var (e.g. "fr_FR.UTF-8" -> "fr")
            if let Ok(sys_lang) = std::env::var("LANG") {
                let lower = sys_lang.to_lowercase();
                if lower.starts_with("fr") {
                    return "fr".to_string();
                } else if lower.starts_with("es") {
                    return "es".to_string();
                } else if lower.starts_with("pt") {
                    return "pt".to_string();
                } else if lower.starts_with("ru") {
                    return "ru".to_string();
                }
            }
            "en".to_string()
        } else {
            self.lang.clone()
        }
    }
}
