// Brain: interprets terminal and tool hook events (Claude, Codex, shell)
// and drives Capri's reactions, dialogues, and stances.

use crate::acting::Player;
use crate::bridge::HookEvent;
use crate::fx::FxLayer;
use crate::lines::Lines;
use crate::mission::MissionTracker;
use crate::panel::Panel;
use crate::util::{basename, current_time_ms, duration_str, rand_range};
use crate::voice::Voice;
use std::collections::{HashMap, HashSet};

const EDIT_TOOLS: &[&str] = &["Write", "Edit", "MultiEdit", "NotebookEdit"];
const HACK_TOOLS: &[&str] = &["Bash", "PowerShell", "WebFetch", "WebSearch", "Agent", "Task"];

#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: String,
    pub project: String,
    pub title: Option<String>,
    pub task_start: Option<f64>,
    pub count: u32,
    pub files: HashSet<String>,
    pub last_action: Option<String>,
    pub working: bool,
    pub waiting: bool,
    pub seen: f64,
}

pub struct Brain {
    pub sessions: HashMap<String, SessionState>,
    pub lines: Lines,
    pub thinking_since: Option<f64>,
    pub thinking_said: u32,
    pub next_fidget: f64,
}

impl Brain {
    pub fn new(lang: &str, assistant: &str) -> Self {
        Self {
            sessions: HashMap::new(),
            lines: Lines::new(lang, assistant),
            thinking_since: None,
            thinking_said: 0,
            next_fidget: 0.0,
        }
    }

    pub fn handle(
        &mut self,
        evt: &HookEvent,
        panel: &mut Panel,
        voice: &Voice,
        fx: &mut FxLayer,
        mission: &mut MissionTracker,
        capri_pos: (f64, f64),
        act_slot: &mut Option<Player>,
        mode: &mut String,
        uplink_due: &mut bool,
    ) {
        let now = current_time_ms();
        let sid = evt.session_id.clone().unwrap_or_else(|| "default".to_string());
        let s = self.sessions.entry(sid.clone()).or_insert_with(|| SessionState {
            id: sid,
            project: evt.cwd.as_deref().map(basename).unwrap_or("project").to_string(),
            title: evt.session_title.clone(),
            task_start: None,
            count: 0,
            files: HashSet::new(),
            last_action: None,
            working: false,
            waiting: false,
            seen: now,
        });

        s.seen = now;
        if let Some(cwd) = &evt.cwd {
            s.project = basename(cwd).to_string();
        }
        if let Some(title) = &evt.session_title {
            s.title = Some(title.clone());
        }

        // If waiting was cleared by any non-notification event
        if s.waiting && evt.hook_event_name != "Notification" {
            s.waiting = false;
            *mode = if s.working { "work".to_string() } else { "idle".to_string() };
        }

        let mut vars = HashMap::new();
        vars.insert("project", s.project.as_str());

        match evt.hook_event_name.as_str() {
            "SessionStart" => {
                voice.play("hello");
                *act_slot = Some(Player::new("curious", 1.0, 1.0, now));
                let line = self.lines.pick_random(&["hello"], &vars);
                panel.say("info", &line, &s.project, Some("look"), None, 4500.0, now);
            }
            "UserPromptSubmit" => {
                s.task_start = Some(now);
                s.count = 0;
                s.files.clear();
                s.working = true;
                *mode = "work".to_string();
                self.thinking_since = Some(now);
                self.thinking_said = 0;
                self.next_fidget = now + 12000.0;

                mission.prompt(&s.project);

                let prompt_preview = evt.prompt.as_deref().unwrap_or("").trim();
                let quote = if prompt_preview.len() > 80 {
                    format!("{}...", &prompt_preview[..80])
                } else {
                    prompt_preview.to_string()
                };

                let line = self.lines.pick_random(&["prompt"], &vars);
                panel.say("work", &line, &s.project, Some("look"), Some(&quote), 4000.0, now);
            }
            "PreToolUse" => {
                s.working = true;
                s.count += 1;
                mission.action();
                self.thinking_since = Some(now); // reset thinking timer

                let tool_name = evt.tool_name.as_deref().unwrap_or("Tool");
                let ticker_verb = match tool_name {
                    "Bash" | "PowerShell" => "RUNS",
                    "Write" | "Edit" | "MultiEdit" => "EDITS",
                    "Grep" => "GREPS",
                    "Glob" => "FINDS",
                    "Read" => "READS",
                    "WebFetch" | "WebSearch" => "NET",
                    _ => "USES",
                };

                let input_str = evt.tool_input.as_ref().map(|v| {
                    if let Some(cmd) = v.get("command").and_then(|c| c.as_str()) {
                        cmd.to_string()
                    } else if let Some(path) = v.get("file_path").and_then(|p| p.as_str()) {
                        basename(path).to_string()
                    } else {
                        tool_name.to_string()
                    }
                }).unwrap_or_else(|| tool_name.to_string());

                let ticker_msg = format!("{ticker_verb} {input_str}");
                s.last_action = Some(ticker_msg.clone());
                panel.set_ticker(Some(ticker_msg));

                // Finger sparks if hacking / shell tool
                if HACK_TOOLS.contains(&tool_name) {
                    fx.add_bolt(capri_pos, (capri_pos.0 + rand_range(-80.0, 80.0), capri_pos.1 - 40.0), 200.0, now);
                    voice.play("bolt");
                }

                if EDIT_TOOLS.contains(&tool_name) {
                    if let Some(path) = evt.tool_input.as_ref().and_then(|v| v.get("file_path")).and_then(|p| p.as_str()) {
                        s.files.insert(path.to_string());
                        mission.file(path);
                    }
                }
            }
            "PostToolUseFailure" => {
                mission.error();
                voice.play("error");
                *act_slot = Some(Player::new("flinch", 0.0, 1.0, now));
                fx.add_burst(capri_pos.0, capri_pos.1 - 50.0, 12, false, 80.0, now);

                let tool = evt.tool_name.as_deref().unwrap_or("Tool");
                let mut err_vars = vars.clone();
                err_vars.insert("tool", tool);
                let line = self.lines.pick_random(&["error"], &err_vars);
                panel.say("error", &line, &s.project, Some("wide"), evt.error.as_deref(), 5000.0, now);
            }
            "Notification" => {
                s.waiting = true;
                mission.approval();
                *mode = "alert".to_string();
                voice.play("alert");
                *act_slot = Some(Player::new("alarm", 1.0, 1.0, now));
                fx.show_cutin("many", capri_pos.0 - 150.0, capri_pos.1 - 100.0, 1.0, 700.0, now);

                let line = self.lines.pick_random(&["permission"], &vars);
                panel.say("alert", &line, &s.project, Some("many"), None, 6000.0, now);
            }
            "Stop" => {
                s.working = false;
                *mode = "idle".to_string();
                self.thinking_since = None;
                panel.set_ticker(None);

                let duration_ms = s.task_start.map(|st| now - st).unwrap_or(0.0);
                mission.finish_mission(duration_ms, &s.project);

                voice.play("done");
                *act_slot = Some(Player::new("victory", 1.0, 1.0, now));
                fx.add_ring(capri_pos.0, capri_pos.1 - 60.0, 120.0, 500.0, now);
                fx.add_burst(capri_pos.0, capri_pos.1 - 60.0, 24, false, 120.0, now);
                fx.show_cutin("laugh", capri_pos.0 + 80.0, capri_pos.1 - 120.0, 1.0, 800.0, now);

                // Long job earns uplink sequence
                if duration_ms > 8.0 * 60.0 * 1000.0 || s.count >= 50 {
                    *uplink_due = true;
                }

                let line = self.lines.pick_random(&["done"], &vars);
                let dur = duration_str(duration_ms, "en");
                let summary = format!("{} actions · {} files · {}", s.count, s.files.len(), dur);
                panel.say("done", &line, &s.project, Some("laugh"), Some(&summary), 6000.0, now);
            }
            "Interrupt" => {
                s.working = false;
                *mode = "idle".to_string();
                self.thinking_since = None;
                panel.set_ticker(None);

                let line = self.lines.pick_random(&["interrupted"], &vars);
                panel.say("info", &line, &s.project, Some("aside"), None, 4000.0, now);
            }
            _ => {}
        }
    }

    pub fn tick_thinking(
        &mut self,
        panel: &mut Panel,
        _capri_pos: (f64, f64),
        act_slot: &mut Option<Player>,
        now: f64,
    ) {
        if let Some(since) = self.thinking_since {
            let quiet = now - since;
            if quiet > 15000.0 {
                let s_dur = duration_str(quiet, "en");
                panel.set_ticker(Some(format!("THINKING… {s_dur}")));

                if now > self.next_fidget {
                    self.next_fidget = now + rand_range(10000.0, 18000.0);
                    if act_slot.is_none() {
                        *act_slot = Some(Player::new("spin", 0.0, 1.0, now));
                    }
                }

                if self.thinking_said == 0 && quiet > 40000.0 {
                    self.thinking_said = 1;
                    panel.say("work", "Toujours en train de réfléchir...", "CLAUDE", Some("aside"), None, 4000.0, now);
                }
            }
        }
    }
}
