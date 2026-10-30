// Capri's cyberpunk transmission panel, dialogue bubble, faces, and action ticker.

use cairo::Context;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use std::collections::HashMap;
use std::path::Path;

pub struct PanelButton {
    pub id: String,
    pub label: String,
    pub primary: bool,
    pub rect: (f64, f64, f64, f64),
}

pub struct Panel {
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: String, // "info", "work", "done", "alert", "error"
    pub tag: String,
    pub line: String,
    pub quote: String,
    pub face: String,
    pub face_pixbufs: HashMap<String, Pixbuf>,
    pub revealed_chars: usize,
    pub char_timer_ms: f64,
    pub hold_until_ms: f64,
    pub ticker_text: Option<String>,
    pub buttons: Vec<PanelButton>,
}

impl Panel {
    pub fn new(assets_dir: &Path) -> Self {
        let mut faces = HashMap::new();
        let face_names = ["calm", "look", "laugh", "smug", "wide", "many", "aside"];

        for &name in &face_names {
            let p = assets_dir.join(format!("face-{name}.png"));
            if let Ok(pb) = Pixbuf::from_file(&p) {
                faces.insert(name.to_string(), pb);
            }
        }

        Self {
            visible: false,
            x: 200.0,
            y: 200.0,
            width: 320.0,
            height: 120.0,
            kind: "info".to_string(),
            tag: String::new(),
            line: String::new(),
            quote: String::new(),
            face: "calm".to_string(),
            face_pixbufs: faces,
            revealed_chars: 0,
            char_timer_ms: 0.0,
            hold_until_ms: 0.0,
            ticker_text: None,
            buttons: Vec::new(),
        }
    }

    pub fn say(
        &mut self,
        kind: &str,
        line: &str,
        tag: &str,
        face: Option<&str>,
        quote: Option<&str>,
        hold_ms: f64,
        now: f64,
    ) {
        self.visible = true;
        self.kind = kind.to_string();
        self.line = line.to_string();
        self.tag = tag.to_string();
        self.quote = quote.unwrap_or("").to_string();
        self.face = face.unwrap_or(match kind {
            "done" => "laugh",
            "alert" => "many",
            "error" => "wide",
            "work" => "look",
            _ => "calm",
        }).to_string();
        self.revealed_chars = 0;
        self.char_timer_ms = now;
        self.hold_until_ms = now + hold_ms;
        self.buttons.clear();
    }

    pub fn set_ticker(&mut self, text: Option<String>) {
        self.ticker_text = text;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.revealed_chars = 0;
        self.buttons.clear();
    }

    pub fn hit_test(&self, x: f64, y: f64) -> bool {
        if !self.visible {
            return false;
        }
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn update_pos(&mut self, head_x: f64, head_y: f64, screen_w: f64, lean: f64) {
        let margin = 24.0;
        let target_x = if lean < 0.0 || (lean == 0.0 && head_x > screen_w / 2.0) {
            head_x - self.width - margin
        } else {
            head_x + margin
        };

        self.x = target_x.clamp(16.0, screen_w - self.width - 16.0);
        self.y = (head_y - self.height / 2.0).max(40.0);
    }

    pub fn draw(&mut self, cr: &Context, now: f64) {
        // Draw ticker if present
        if let Some(ticker) = &self.ticker_text {
            cr.save().unwrap();
            cr.set_source_rgba(0.08, 0.06, 0.11, 0.85);
            let tw = 220.0;
            let th = 26.0;
            let tx = self.x;
            let ty = (self.y - 32.0).max(10.0);
            cr.rectangle(tx, ty, tw, th);
            let _ = cr.fill();

            // Neon line
            cr.set_source_rgba(0.21, 1.0, 0.48, 0.9);
            cr.set_line_width(1.5);
            cr.rectangle(tx, ty, tw, th);
            let _ = cr.stroke();

            // Ticker text
            cr.set_source_rgba(0.9, 1.0, 0.95, 1.0);
            cr.set_font_size(11.0);
            cr.move_to(tx + 8.0, ty + 17.0);
            let _ = cr.show_text(ticker);
            cr.restore().unwrap();
        }

        if !self.visible {
            return;
        }

        // Auto-close if hold time expired
        if self.hold_until_ms > 0.0 && now > self.hold_until_ms {
            self.hide();
            return;
        }

        // Advance typewriter
        if now - self.char_timer_ms > 25.0 && self.revealed_chars < self.line.chars().count() {
            self.revealed_chars += 1;
            self.char_timer_ms = now;
        }

        cr.save().unwrap();

        // Cyberpunk dark background
        cr.set_source_rgba(0.08, 0.06, 0.11, 0.92);
        cr.rectangle(self.x, self.y, self.width, self.height);
        let _ = cr.fill();

        // Rim border colored by kind
        let (border_r, border_g, border_b) = match self.kind.as_str() {
            "done" => (0.21, 1.0, 0.48),  // Neon Green
            "error" => (1.0, 0.13, 0.32), // Hot Red
            "alert" => (1.0, 0.8, 0.2),   // Amber
            _ => (0.48, 0.24, 1.0),       // Violet
        };

        cr.set_source_rgba(border_r, border_g, border_b, 0.85);
        cr.set_line_width(2.0);
        cr.rectangle(self.x, self.y, self.width, self.height);
        let _ = cr.stroke();

        // Draw portrait face
        let face_size = 56.0;
        let pad = 10.0;
        let face_x = self.x + pad;
        let face_y = self.y + pad;

        if let Some(pb) = self.face_pixbufs.get(&self.face) {
            cr.save().unwrap();
            let scale_x = face_size / pb.width() as f64;
            let scale_y = face_size / pb.height() as f64;
            cr.translate(face_x, face_y);
            cr.scale(scale_x, scale_y);
            cr.set_source_pixbuf(pb, 0.0, 0.0);
            let _ = cr.paint();
            cr.restore().unwrap();

            // Face box outline
            cr.set_source_rgba(border_r, border_g, border_b, 0.5);
            cr.set_line_width(1.0);
            cr.rectangle(face_x, face_y, face_size, face_size);
            let _ = cr.stroke();
        }

        // Draw Tag / Status Badge
        let text_x = face_x + face_size + 12.0;
        let mut text_y = self.y + 20.0;

        if !self.tag.is_empty() {
            cr.set_source_rgba(border_r, border_g, border_b, 0.9);
            cr.set_font_size(10.0);
            cr.move_to(text_x, text_y);
            let _ = cr.show_text(&self.tag.to_uppercase());
            text_y += 16.0;
        }

        // Draw Typed Dialogue Text
        let displayed_text: String = self.line.chars().take(self.revealed_chars).collect();
        cr.set_source_rgba(0.95, 0.93, 0.98, 1.0);
        cr.set_font_size(13.0);

        // Simple text wrap (max ~32 chars per line)
        for chunk in displayed_text.split('\n') {
            let mut line_buf = String::new();
            for word in chunk.split(' ') {
                if line_buf.len() + word.len() > 28 {
                    cr.move_to(text_x, text_y);
                    let _ = cr.show_text(&line_buf);
                    text_y += 18.0;
                    line_buf.clear();
                }
                if !line_buf.is_empty() {
                    line_buf.push(' ');
                }
                line_buf.push_str(word);
            }
            if !line_buf.is_empty() {
                cr.move_to(text_x, text_y);
                let _ = cr.show_text(&line_buf);
                text_y += 18.0;
            }
        }

        // Quote text if present
        if !self.quote.is_empty() {
            cr.set_source_rgba(0.65, 0.65, 0.75, 0.9);
            cr.set_font_size(11.0);
            cr.move_to(text_x, text_y + 4.0);
            let _ = cr.show_text(&format!("\"{}\"", self.quote));
        }

        cr.restore().unwrap();
    }
}
