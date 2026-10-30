use crate::util::today_str;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDay {
    pub date: String,
    pub missions: u32,
    pub actions: u32,
    pub files: HashSet<String>,
    pub errors: u32,
    pub prompts: u32,
    pub approvals: u32,
    pub longest_ms: f64,
    pub longest_project: String,
    pub projects: HashSet<String>,
    pub reported: bool,
}

impl Default for MissionDay {
    fn default() -> Self {
        Self {
            date: today_str(),
            missions: 0,
            actions: 0,
            files: HashSet::new(),
            errors: 0,
            prompts: 0,
            approvals: 0,
            longest_ms: 0.0,
            longest_project: String::new(),
            projects: HashSet::new(),
            reported: false,
        }
    }
}

pub struct MissionTracker {
    pub day: MissionDay,
}

impl MissionTracker {
    fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("capri")
            .join("missions.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let today = today_str();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(day) = serde_json::from_str::<MissionDay>(&data) {
                if day.date == today {
                    return Self { day };
                }
            }
        }
        Self {
            day: MissionDay {
                date: today,
                ..Default::default()
            },
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(&self.day) {
            let _ = fs::write(path, data);
        }
    }

    pub fn prompt(&mut self, project: &str) {
        self.day.prompts += 1;
        self.day.projects.insert(project.to_string());
        self.save();
    }

    pub fn action(&mut self) {
        self.day.actions += 1;
        self.save();
    }

    pub fn file(&mut self, path: &str) {
        self.day.files.insert(path.to_string());
        self.save();
    }

    pub fn error(&mut self) {
        self.day.errors += 1;
        self.save();
    }

    pub fn approval(&mut self) {
        self.day.approvals += 1;
        self.save();
    }

    pub fn finish_mission(&mut self, duration_ms: f64, project: &str) {
        self.day.missions += 1;
        if duration_ms > self.day.longest_ms {
            self.day.longest_ms = duration_ms;
            self.day.longest_project = project.to_string();
        }
        self.save();
    }

    pub fn mark_reported(&mut self) {
        self.day.reported = true;
        self.save();
    }
}
