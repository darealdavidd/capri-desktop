// Biochip and sleeping mask renderer.
// When stowed (Ctrl+Alt+C) or asleep for a long time, Capri retreats to her mask/biochip.

use cairo::Context;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use std::path::Path;

pub struct Mask {
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub waking: bool,
    pub pixbuf: Option<Pixbuf>,
}

impl Mask {
    pub fn new(assets_dir: &Path) -> Self {
        let p = assets_dir.join("capri-face.png");
        let pixbuf = Pixbuf::from_file(p).ok();
        Self {
            visible: false,
            x: 100.0,
            y: 100.0,
            scale: 0.6,
            waking: false,
            pixbuf,
        }
    }

    pub fn show(&mut self, x: f64, y: f64, scale: f64) {
        self.visible = true;
        self.x = x;
        self.y = y;
        self.scale = scale;
        self.waking = false;
    }

    pub fn wake(&mut self) {
        self.waking = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.waking = false;
    }

    pub fn hit_test(&self, mx: f64, my: f64) -> bool {
        if !self.visible {
            return false;
        }
        let radius = 40.0 * self.scale;
        let dx = mx - self.x;
        let dy = my - self.y;
        (dx * dx + dy * dy).sqrt() <= radius
    }

    pub fn draw(&self, cr: &Context, now: f64) {
        if !self.visible {
            return;
        }
        if let Some(pb) = &self.pixbuf {
            cr.save().unwrap();
            let bob = (now * 0.003).sin() * 4.0;
            let size = 64.0 * self.scale;
            cr.translate(self.x - size / 2.0, self.y - size / 2.0 + bob);

            let sx = size / pb.width() as f64;
            let sy = size / pb.height() as f64;
            cr.scale(sx, sy);
            cr.set_source_pixbuf(pb, 0.0, 0.0);
            let _ = cr.paint();
            cr.restore().unwrap();

            // Soft aura
            cr.save().unwrap();
            cr.set_source_rgba(0.21, 1.0, 0.48, 0.25);
            cr.arc(self.x, self.y + bob, 32.0 * self.scale, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.fill();
            cr.restore().unwrap();
        }
    }
}

pub struct Chip {
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub mode: String,
}

impl Chip {
    pub fn new() -> Self {
        Self {
            visible: false,
            x: 20.0,
            y: 20.0,
            width: 70.0,
            height: 38.0,
            mode: "idle".to_string(),
        }
    }

    pub fn place(&mut self, _screen_w: f64, screen_h: f64) {
        self.x = 24.0;
        self.y = screen_h - self.height - 24.0;
    }

    pub fn hit_test(&self, mx: f64, my: f64) -> bool {
        if !self.visible {
            return false;
        }
        mx >= self.x && mx <= self.x + self.width && my >= self.y && my <= self.y + self.height
    }

    pub fn draw(&self, cr: &Context, _now: f64) {
        if !self.visible {
            return;
        }

        cr.save().unwrap();
        // Dark futuristic biochip body
        cr.set_source_rgba(0.08, 0.06, 0.11, 0.9);
        cr.rectangle(self.x, self.y, self.width, self.height);
        let _ = cr.fill();

        // Neon border
        let (r, g, b) = match self.mode.as_str() {
            "work" => (0.48, 0.24, 1.0),
            "alert" => (1.0, 0.8, 0.2),
            _ => (0.21, 1.0, 0.48),
        };
        cr.set_source_rgba(r, g, b, 0.9);
        cr.set_line_width(2.0);
        cr.rectangle(self.x, self.y, self.width, self.height);
        let _ = cr.stroke();

        // Chip PIN contacts
        cr.set_source_rgba(1.0, 0.85, 0.3, 0.8);
        for i in 0..5 {
            cr.rectangle(self.x + 8.0 + i as f64 * 11.0, self.y + self.height - 6.0, 6.0, 4.0);
            let _ = cr.fill();
        }

        // Label
        cr.set_source_rgba(0.9, 0.95, 1.0, 1.0);
        cr.set_font_size(10.0);
        cr.move_to(self.x + 8.0, self.y + 18.0);
        let _ = cr.show_text("CAPRI.BIO");

        cr.restore().unwrap();
    }
}
