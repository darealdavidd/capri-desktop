// Capri's dialogue lines across languages and coding assistants.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

static LINES_DATA: OnceLock<Value> = OnceLock::new();
static OTHERS_DATA: OnceLock<Value> = OnceLock::new();

fn get_lines_raw() -> &'static Value {
    LINES_DATA.get_or_init(|| {
        let raw = include_str!("../assets/lines.json");
        serde_json::from_str(raw).unwrap_or(Value::Null)
    })
}

fn get_others_raw() -> &'static Value {
    OTHERS_DATA.get_or_init(|| {
        let raw = include_str!("../assets/others.json");
        serde_json::from_str(raw).unwrap_or(Value::Null)
    })
}

pub struct Lines {
    lang: String,
    assistant: String,
}

impl Lines {
    pub fn new(lang: &str, assistant: &str) -> Self {
        Self {
            lang: lang.to_string(),
            assistant: assistant.to_string(),
        }
    }

    pub fn assistant_display_name(&self) -> &str {
        match self.assistant.as_str() {
            "codex" => "Codex",
            "agy" => "Antigravity",
            _ => "Claude",
        }
    }

    pub fn get_value(&self, path: &[&str]) -> Option<&'static Value> {
        let root = get_lines_raw();
        let lang_obj = root.get(&self.lang).or_else(|| root.get("en"))?;
        let mut curr = lang_obj;
        for &seg in path {
            curr = curr.get(seg)?;
        }
        Some(curr)
    }

    pub fn get_str(&self, path: &[&str], vars: &HashMap<&str, &str>) -> String {
        if let Some(val) = self.get_value(path) {
            if let Some(s) = val.as_str() {
                return self.interpolate(s, vars);
            }
        }
        String::new()
    }

    pub fn get_list(&self, path: &[&str]) -> Vec<String> {
        if let Some(val) = self.get_value(path) {
            if let Some(arr) = val.as_array() {
                return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
        }
        Vec::new()
    }

    pub fn pick_random(&self, path: &[&str], vars: &HashMap<&str, &str>) -> String {
        let list = self.get_list(path);
        if list.is_empty() {
            return String::new();
        }
        let item = &list[rand::random::<usize>() % list.len()];
        self.interpolate(item, vars)
    }

    fn interpolate(&self, text: &str, vars: &HashMap<&str, &str>) -> String {
        let mut out = text.to_string();
        for (&k, &v) in vars {
            out = out.replace(&format!("{{{k}}}"), v);
        }

        // Adapt assistant name if watching Codex or Antigravity
        if self.assistant != "claude" {
            let others = get_others_raw();
            if let Some(other) = others.get(&self.assistant) {
                if let Some(name) = other.get("name").and_then(|v| v.as_str()) {
                    out = out.replace("Claude Code", name);
                    out = out.replace("Claude", name);
                    out = out.replace("CLAUDE", &name.to_uppercase());
                }
                if let Some(file) = other.get("file").and_then(|v| v.as_str()) {
                    out = out.replace("settings.json", file);
                }
                if let Some(dir) = other.get("dir").and_then(|v| v.as_str()) {
                    out = out.replace(".claude", dir);
                }
            }
        }
        out
    }
}
