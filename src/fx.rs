// Capri's visual effects: lightning bolts, particle bursts, shockwave rings,
// speed lines, teleport beams, satellite sequence, and anime cut-ins.

use cairo::Context;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use std::collections::HashMap;
use std::path::Path;

pub struct Bolt {
    from: (f64, f64),
    to: (f64, f64),
    life_ms: f64,
    start_ms: f64,
    segments: Vec<((f64, f64), (f64, f64))>,
}

pub struct Ring {
    x: f64,
    y: f64,
    radius: f64,
    max_radius: f64,
    start_ms: f64,
    duration_ms: f64,
}

pub struct BurstParticle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    size: f64,
    alpha: f64,
}

pub struct ParticleBurst {
    particles: Vec<BurstParticle>,
    start_ms: f64,
    duration_ms: f64,
}

pub struct TeleportBeam {
    x: f64,
    y: f64,
    width: f64,
    down: bool,
    start_ms: f64,
    duration_ms: f64,
}

pub struct Satellite {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub start_ms: f64,
    pub duration_ms: f64,
    pub twinkle_intensity: f64,
}

pub struct Cutin {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub dir: f64,
    pub start_ms: f64,
    pub duration_ms: f64,
}

pub struct FxLayer {
    bolts: Vec<Bolt>,
    rings: Vec<Ring>,
    bursts: Vec<ParticleBurst>,
    beams: Vec<TeleportBeam>,
    pub satellite: Option<Satellite>,
    pub cutin: Option<Cutin>,
    cutin_pixbufs: HashMap<String, Pixbuf>,
}

impl FxLayer {
    pub fn new(assets_dir: &Path) -> Self {
        let mut cutins = HashMap::new();
        let cutin_names = ["grin", "laugh", "eyes", "many", "look"];

        for &name in &cutin_names {
            let p = assets_dir.join(format!("cutin-{name}.png"));
            if let Ok(pb) = Pixbuf::from_file(&p) {
                cutins.insert(name.to_string(), pb);
            }
        }

        Self {
            bolts: Vec::new(),
            rings: Vec::new(),
            bursts: Vec::new(),
            beams: Vec::new(),
            satellite: None,
            cutin: None,
            cutin_pixbufs: cutins,
        }
    }

    pub fn add_bolt(&mut self, from: (f64, f64), to: (f64, f64), life_ms: f64, now: f64) {
        let mut segments = Vec::new();
        let steps = 6;
        let mut curr = from;
        let dx = (to.0 - from.0) / steps as f64;
        let dy = (to.1 - from.1) / steps as f64;

        for i in 1..steps {
            let offset_x = (rand::random::<f64>() - 0.5) * 28.0;
            let offset_y = (rand::random::<f64>() - 0.5) * 28.0;
            let next = (from.0 + dx * i as f64 + offset_x, from.1 + dy * i as f64 + offset_y);
            segments.push((curr, next));
            curr = next;
        }
        segments.push((curr, to));

        self.bolts.push(Bolt {
            from,
            to,
            life_ms,
            start_ms: now,
            segments,
        });
    }

    pub fn add_ring(&mut self, x: f64, y: f64, max_radius: f64, duration_ms: f64, now: f64) {
        self.rings.push(Ring {
            x,
            y,
            radius: 5.0,
            max_radius,
            start_ms: now,
            duration_ms,
        });
    }

    pub fn add_burst(&mut self, x: f64, y: f64, count: usize, inward: bool, spread: f64, now: f64) {
        let mut particles = Vec::with_capacity(count);
        for _ in 0..count {
            let angle = rand::random::<f64>() * std::f64::consts::PI * 2.0;
            let speed = (rand::random::<f64>() * 0.7 + 0.3) * spread / 0.4;
            let (vx, vy) = if inward {
                (-angle.cos() * speed, -angle.sin() * speed)
            } else {
                (angle.cos() * speed, angle.sin() * speed)
            };
            particles.push(BurstParticle {
                x: if inward { x + angle.cos() * spread } else { x },
                y: if inward { y + angle.sin() * spread } else { y },
                vx,
                vy,
                size: rand::random::<f64>() * 3.5 + 2.0,
                alpha: 1.0,
            });
        }

        self.bursts.push(ParticleBurst {
            particles,
            start_ms: now,
            duration_ms: 450.0,
        });
    }

    pub fn add_beam(&mut self, x: f64, y: f64, width: f64, down: bool, duration_ms: f64, now: f64) {
        self.beams.push(TeleportBeam {
            x,
            y,
            width,
            down,
            start_ms: now,
            duration_ms,
        });
    }

    pub fn show_cutin(&mut self, name: &str, x: f64, y: f64, dir: f64, duration_ms: f64, now: f64) {
        self.cutin = Some(Cutin {
            name: name.to_string(),
            x,
            y,
            dir,
            start_ms: now,
            duration_ms,
        });
    }

    pub fn update_and_draw(&mut self, cr: &Context, now: f64) {
        // Draw Teleport Beams
        self.beams.retain(|b| now - b.start_ms < b.duration_ms);
        for b in &self.beams {
            let progress = (now - b.start_ms) / b.duration_ms;
            let alpha = (1.0 - progress).powi(2);
            cr.set_source_rgba(1.0, 0.13, 0.32, alpha * 0.85);
            let w = b.width * (1.0 - progress * 0.5);
            cr.rectangle(b.x - w / 2.0, 0.0, w, b.y);
            let _ = cr.fill();
        }

        // Draw Rings (Shockwaves)
        self.rings.retain(|r| now - r.start_ms < r.duration_ms);
        for r in &self.rings {
            let progress = (now - r.start_ms) / r.duration_ms;
            let radius = r.radius + (r.max_radius - r.radius) * progress;
            let alpha = 1.0 - progress;
            cr.set_source_rgba(0.21, 1.0, 0.48, alpha * 0.7);
            cr.set_line_width(2.5 * (1.0 - progress));
            cr.arc(r.x, r.y, radius, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.stroke();
        }

        // Draw Lightning Bolts
        self.bolts.retain(|b| now - b.start_ms < b.life_ms);
        for b in &self.bolts {
            let progress = (now - b.start_ms) / b.life_ms;
            let alpha = 1.0 - progress;
            cr.set_source_rgba(1.0, 0.13, 0.32, alpha);
            cr.set_line_width(2.5);
            for &(p1, p2) in &b.segments {
                cr.move_to(p1.0, p1.1);
                cr.line_to(p2.0, p2.1);
            }
            let _ = cr.stroke();
        }

        // Draw Bursts
        self.bursts.retain(|b| now - b.start_ms < b.duration_ms);
        for b in &mut self.bursts {
            let progress = (now - b.start_ms) / b.duration_ms;
            let alpha = (1.0 - progress).max(0.0);
            let dt = 0.016; // approximate delta

            for p in &mut b.particles {
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                cr.set_source_rgba(1.0, 0.2, 0.35, alpha * p.alpha);
                cr.arc(p.x, p.y, p.size, 0.0, std::f64::consts::PI * 2.0);
                let _ = cr.fill();
            }
        }

        // Draw Satellite
        if let Some(sat) = &self.satellite {
            let progress = ((now - sat.start_ms) / sat.duration_ms).clamp(0.0, 1.0);
            let sx = sat.from.0 + (sat.to.0 - sat.from.0) * progress;
            let sy = sat.from.1 + (sat.to.1 - sat.from.1) * progress;

            // Twinkling white point
            let pulse = (now * 0.01).sin() * 0.3 + 0.7;
            cr.set_source_rgba(1.0, 1.0, 1.0, pulse);
            cr.arc(sx, sy, 3.5 * sat.twinkle_intensity, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.fill();

            // Soft cyan aura
            cr.set_source_rgba(0.2, 0.9, 1.0, 0.3 * pulse);
            cr.arc(sx, sy, 8.0, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.fill();
        }

        // Draw Anime Cutin
        if let Some(cutin) = &self.cutin {
            if now - cutin.start_ms < cutin.duration_ms {
                if let Some(pb) = self.cutin_pixbufs.get(&cutin.name) {
                    let progress = (now - cutin.start_ms) / cutin.duration_ms;
                    let alpha = if progress < 0.15 {
                        progress / 0.15
                    } else if progress > 0.8 {
                        (1.0 - progress) / 0.2
                    } else {
                        1.0
                    };

                    cr.save().unwrap();
                    cr.translate(cutin.x, cutin.y);
                    if cutin.dir < 0.0 {
                        cr.scale(-1.0, 1.0);
                    }
                    cr.set_source_pixbuf(pb, 0.0, 0.0);
                    let _ = cr.paint_with_alpha(alpha);
                    cr.restore().unwrap();
                }
            } else {
                self.cutin = None;
            }
        }
    }
}
