// Capri's character controller: physics, behavior, stances, dragging, and teleportation.

use crate::acting::{Pose, Player};
use crate::fx::FxLayer;
use crate::sprite::{SpriteParams, SpriteRenderer, LANDMARK_FEET, LANDMARK_HEAD};
use crate::util::{clamp, current_time_ms, rand_range};
use crate::voice::Voice;

const GRAVITY: f64 = 2600.0; // px/s²
const SLEEP_AFTER_MS: f64 = 10.0 * 60.0 * 1000.0;
const DOZE_AFTER_MS: f64 = 20.0 * 1000.0;

pub struct Capri {
    pub pos: (f64, f64),
    pub facing: f64,
    pub mode: String,
    pub stance: String,
    pub scale: f64,
    pub floor: f64,
    pub screen_w: f64,
    pub screen_h: f64,
    pub dragging: bool,
    pub drag_start: (f64, f64),
    pub pos_at_drag: (f64, f64),
    pub vy: f64,
    pub p: SpriteParams,
    pub bob: f64,
    pub tilt: f64,
    pub sx: f64,
    pub sy: f64,
    pub base: Player,
    pub act: Option<Player>,
    pub wander: bool,
    pub stowed: bool,
    pub dormant: bool,
    pub last_activity: f64,
    pub slept_at: f64,
    pub next_move: f64,
    pub next_twitch: f64,
    pub next_shed: f64,
    pub next_gaze: f64,
    pub uplink_due: bool,
}

impl Capri {
    pub fn new(screen_w: f64, screen_h: f64, scale: f64) -> Self {
        let now = current_time_ms();
        let floor = screen_h - 10.0;
        let pos = (screen_w - 220.0, floor);

        Self {
            pos,
            facing: 1.0,
            mode: "idle".to_string(),
            stance: "stand".to_string(),
            scale,
            floor,
            screen_w,
            screen_h,
            dragging: false,
            drag_start: (0.0, 0.0),
            pos_at_drag: pos,
            vy: 0.0,
            p: SpriteParams::default(),
            bob: 0.0,
            tilt: 0.0,
            sx: 1.0,
            sy: 1.0,
            base: Player::new("stand", 0.0, 1.0, now),
            act: None,
            wander: true,
            stowed: false,
            dormant: false,
            last_activity: now,
            slept_at: 0.0,
            next_move: now + rand_range(25000.0, 60000.0),
            next_twitch: now + rand_range(4000.0, 9000.0),
            next_shed: now + rand_range(4000.0, 9000.0),
            next_gaze: now + rand_range(40000.0, 90000.0),
            uplink_due: false,
        }
    }

    pub fn head_pos(&self) -> (f64, f64) {
        let s = self.scale;
        let dx = (LANDMARK_HEAD.0 - LANDMARK_FEET.0) * s * self.facing;
        let dy = (LANDMARK_FEET.1 - LANDMARK_HEAD.1) * s;
        (self.pos.0 + dx, self.pos.1 - dy + self.bob)
    }

    pub fn wake(&mut self) {
        let now = current_time_ms();
        self.last_activity = now;
        if self.mode == "sleep" {
            self.mode = "idle".to_string();
            self.p.alpha = 1.0;
            self.p.sparkle = 1.0;
            self.base = Player::new("stand", 0.0, 1.0, now);
        }
        self.dormant = false;
    }

    pub fn perform(&mut self, act_name: &str, dir: f64) {
        if self.stowed || self.dormant {
            return;
        }
        let now = current_time_ms();
        let next = Player::new(act_name, dir, 1.0, now);

        if let Some(curr) = &self.act {
            if curr.priority() > next.priority() {
                return;
            }
        }
        self.act = Some(next);
    }

    pub fn teleport(&mut self, to: (f64, f64), fx: &mut FxLayer, voice: &Voice) {
        let now = current_time_ms();
        voice.play("teleport-out");
        fx.add_burst(self.pos.0, self.pos.1 - 100.0, 20, false, 80.0, now);
        fx.add_beam(self.pos.0, self.pos.1, 40.0, false, 250.0, now);

        self.pos = to;
        self.facing = if to.0 > self.screen_w / 2.0 { 1.0 } else { -1.0 };
        self.wake();

        voice.play("teleport-in");
        fx.add_beam(to.0, to.1, 40.0, true, 250.0, now);
        fx.add_ring(to.0, to.1 - 60.0, 80.0, 400.0, now);
        fx.add_burst(to.0, to.1 - 60.0, 16, true, 90.0, now);
    }

    pub fn fall(&mut self) {
        if self.stance != "fall" {
            self.stance = "fall".to_string();
            self.base = Player::new("fall", 0.0, 1.0, current_time_ms());
            self.vy = 0.0;
        }
    }

    pub fn tick(&mut self, dt: f64, fx: &mut FxLayer, voice: &Voice, _renderer: &SpriteRenderer) {
        let now = current_time_ms();

        // 1. Line boil
        self.p.seed = rand_range(0.0, 100.0);

        // 2. Physics & Falling
        if self.stance == "fall" {
            self.vy = (self.vy + GRAVITY * dt).min(1900.0);
            self.pos.1 += self.vy * dt;

            if self.pos.1 >= self.floor {
                self.pos.1 = self.floor;
                self.stance = "stand".to_string();
                self.base = Player::new("stand", 0.0, 1.0, now);
                self.perform("land", 0.0);
                voice.play("land");
                fx.add_burst(self.pos.0, self.pos.1 - 20.0, 14, false, 60.0, now);
            }
        }

        // 3. Pose sampling
        let base_drawing = self.base.sample(now);
        let base_pose = base_drawing
            .as_ref()
            .map(|d| self.base.resolve(&d.pose, self.facing))
            .unwrap_or_default();

        let mut act_cue = None;
        let act_pose = if let Some(player) = &self.act {
            if let Some(drawing) = player.sample(now) {
                act_cue = drawing.cue;
                player.resolve(&drawing.pose, self.facing)
            } else {
                if player.act.flip {
                    self.facing *= -1.0;
                }
                self.act = None;
                Pose::default()
            }
        } else {
            Pose::default()
        };

        let pose = Pose::neutral().merge(&base_pose).merge(&act_pose);

        // Apply to sprite params
        self.p.head = pose.head.unwrap_or(0.0);
        self.p.mane = pose.mane.unwrap_or(0.0);
        self.p.hair = pose.hair.unwrap_or(0.0);
        self.p.arm_l = pose.arm_l.unwrap_or(0.0);
        self.p.arm_r = pose.arm_r.unwrap_or(0.0);
        self.p.long_l = pose.long_l.unwrap_or(1.0);
        self.p.long_r = pose.long_r.unwrap_or(1.0);
        self.p.kick_l = pose.kick_l.unwrap_or(0.0);
        self.p.kick_r = pose.kick_r.unwrap_or(0.0);
        self.p.smear = pose.smear.unwrap_or((0.0, 0.0));
        self.p.echo = pose.echo.unwrap_or((0.0, 0.0));

        self.bob = if self.dragging { 0.0 } else { pose.bob.unwrap_or(0.0) };
        self.tilt = if self.dragging { 0.0 } else { pose.tilt.unwrap_or(0.0) };
        self.sx = pose.sx.unwrap_or(1.0);
        self.sy = pose.sy.unwrap_or(1.0);

        // Handle animation sound/fx cues
        if let Some(cue) = act_cue {
            match cue {
                "zap" => {
                    voice.play("bolt");
                    fx.add_burst(self.pos.0, self.pos.1 - 100.0, 10, false, 30.0, now);
                }
                "laugh" => {
                    fx.show_cutin("laugh", self.pos.0 + 80.0, self.pos.1 - 140.0, 1.0, 600.0, now);
                }
                "stare" => {
                    fx.show_cutin("many", self.pos.0 - 150.0, self.pos.1 - 100.0, 1.0, 640.0, now);
                }
                _ => {}
            }
        }

        // 4. Autonomous behavior loop
        if !self.dragging && self.stance != "fall" && !self.stowed && !self.dormant {
            // Glitch twitches
            if now > self.next_twitch {
                self.next_twitch = now + rand_range(6000.0, 14000.0);
                if self.mode != "sleep" {
                    self.p.glitch = 0.22;
                    self.p.split = 3.0;
                }
            } else if self.p.glitch > 0.0 {
                self.p.glitch = (self.p.glitch - dt * 2.0).max(0.0);
                self.p.split = (self.p.split - dt * 25.0).max(0.0);
            }

            // Shed mane shards
            if now > self.next_shed {
                self.next_shed = now + rand_range(5000.0, 11000.0);
                let head = self.head_pos();
                fx.add_burst(head.0, head.1, 3, false, 30.0, now);
            }

            // Gaze at satellite
            if self.mode == "idle" && now > self.next_gaze {
                self.next_gaze = now + rand_range(60000.0, 140000.0);
                let from = (clamp(self.pos.0 - 200.0, 40.0, self.screen_w - 40.0), 50.0);
                let to = (clamp(from.0 + 300.0, 40.0, self.screen_w - 40.0), 40.0);
                fx.satellite = Some(crate::fx::Satellite {
                    from,
                    to,
                    start_ms: now,
                    duration_ms: 9500.0,
                    twinkle_intensity: 1.0,
                });
                voice.play("star");
                self.perform("look", 1.0);
            }

            // Sleep check
            if self.mode == "idle" && now - self.last_activity > SLEEP_AFTER_MS {
                self.mode = "sleep".to_string();
                self.slept_at = now;
                self.p.alpha = 0.55;
                self.p.sparkle = 0.35;
                self.base = Player::new("sleep", 0.0, 1.0, now);
            }

            // Doze check
            if self.mode == "sleep" && now - self.slept_at > DOZE_AFTER_MS {
                self.dormant = true;
                self.p.alpha = 0.0;
            }

            // Wandering
            if self.wander && self.mode == "idle" && now > self.next_move {
                self.next_move = now + rand_range(30000.0, 75000.0);
                let r = rand_range(0.0, 1.0);
                if r < 0.4 {
                    let target_x = rand_range(120.0, self.screen_w - 180.0);
                    self.teleport((target_x, self.floor), fx, voice);
                } else if r < 0.7 {
                    let acts = ["spin", "hop", "look", "curious", "split"];
                    let chosen = acts[rand::random::<usize>() % acts.len()];
                    self.perform(chosen, if self.pos.0 > self.screen_w / 2.0 { -1.0 } else { 1.0 });
                }
            }
        }
    }

    pub fn start_drag(&mut self, mx: f64, my: f64) {
        self.wake();
        self.dragging = true;
        self.drag_start = (mx, my);
        self.pos_at_drag = self.pos;
        self.stance = "stand".to_string();
        self.act = None;
    }

    pub fn drag_move(&mut self, mx: f64, my: f64) {
        if self.dragging {
            let dx = mx - self.drag_start.0;
            let dy = my - self.drag_start.1;
            self.pos.0 = clamp(self.pos_at_drag.0 + dx, 60.0, self.screen_w - 60.0);
            self.pos.1 = clamp(self.pos_at_drag.1 + dy, 180.0, self.floor);
        }
    }

    pub fn end_drag(&mut self, voice: &Voice) {
        if self.dragging {
            self.dragging = false;
            if self.pos.1 < self.floor - 4.0 {
                self.fall();
            } else {
                self.perform("land", 0.0);
                voice.play("land");
            }
        }
    }

    pub fn handle_mouse_hover(&mut self, mx: f64, my: f64, _renderer: &SpriteRenderer) {
        let (hx, hy) = self.head_pos();
        let dist = ((mx - hx).powi(2) + (my - hy).powi(2)).sqrt();

        if dist < 180.0 {
            if self.mode == "sleep" {
                self.wake();
            }
            // Turn toward cursor if facing away
            let wanted = if mx < self.pos.0 { 1.0 } else { -1.0 };
            if wanted != self.facing && self.act.is_none() {
                self.facing = wanted;
                self.perform("curious", 1.0);
            }
        }
    }
}
