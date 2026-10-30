// Pose-to-pose acting, timed the way cel animation is: an act is an exposure sheet,
// a list of drawings, each held for a number of frames. Nothing is tweened.

pub const SHEET_FPS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub head: Option<f64>,
    pub mane: Option<f64>,
    pub hair: Option<f64>,
    pub tilt: Option<f64>,
    pub bob: Option<f64>,
    pub sx: Option<f64>,
    pub sy: Option<f64>,
    pub arms: Option<f64>,
    pub arm_l: Option<f64>,
    pub arm_r: Option<f64>,
    pub reach: Option<f64>,
    pub long: Option<f64>,
    pub long_l: Option<f64>,
    pub long_r: Option<f64>,
    pub kick_l: Option<f64>,
    pub kick_r: Option<f64>,
    pub smear: Option<(f64, f64)>,
    pub echo: Option<(f64, f64)>,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            head: None,
            mane: None,
            hair: None,
            tilt: None,
            bob: None,
            sx: None,
            sy: None,
            arms: None,
            arm_l: None,
            arm_r: None,
            reach: None,
            long: None,
            long_l: None,
            long_r: None,
            kick_l: None,
            kick_r: None,
            smear: None,
            echo: None,
        }
    }
}

impl Pose {
    pub fn neutral() -> Self {
        Self {
            head: Some(0.0),
            mane: Some(0.0),
            hair: Some(0.0),
            tilt: Some(0.0),
            bob: Some(0.0),
            sx: Some(1.0),
            sy: Some(1.0),
            arms: None,
            arm_l: Some(0.0),
            arm_r: Some(0.0),
            reach: None,
            long: None,
            long_l: Some(1.0),
            long_r: Some(1.0),
            kick_l: Some(0.0),
            kick_r: Some(0.0),
            smear: Some((0.0, 0.0)),
            echo: Some((0.0, 0.0)),
        }
    }

    pub fn merge(&self, other: &Pose) -> Self {
        Self {
            head: other.head.or(self.head),
            mane: other.mane.or(self.mane),
            hair: other.hair.or(self.hair),
            tilt: other.tilt.or(self.tilt),
            bob: other.bob.or(self.bob),
            sx: other.sx.or(self.sx),
            sy: other.sy.or(self.sy),
            arms: other.arms.or(self.arms),
            arm_l: other.arm_l.or(self.arm_l),
            arm_r: other.arm_r.or(self.arm_r),
            reach: other.reach.or(self.reach),
            long: other.long.or(self.long),
            long_l: other.long_l.or(self.long_l),
            long_r: other.long_r.or(self.long_r),
            kick_l: other.kick_l.or(self.kick_l),
            kick_r: other.kick_r.or(self.kick_r),
            smear: other.smear.or(self.smear),
            echo: other.echo.or(self.echo),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub hold: u32,
    pub pose: Pose,
    pub cue: Option<&'static str>,
}

impl Key {
    pub fn new(hold: u32, pose: Pose, cue: Option<&'static str>) -> Self {
        Self { hold, pose, cue }
    }
}

#[derive(Debug, Clone)]
pub struct Act {
    pub name: &'static str,
    pub loop_act: bool,
    pub directional: bool,
    pub flip: bool,
    pub fps: u32,
    pub priority: u32,
    pub keys: Vec<Key>,
}

pub struct Drawing {
    pub pose: Pose,
    pub index: usize,
    pub cue: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub act: Act,
    pub dir: f64,
    pub reach: f64,
    pub start: f64,
    pub total: u32,
}

impl Player {
    pub fn new(name: &str, dir: f64, reach: f64, at: f64) -> Self {
        let act = get_act(name);
        let total = act.keys.iter().map(|k| k.hold).sum();
        Self {
            name: name.to_string(),
            act,
            dir,
            reach,
            start: at,
            total,
        }
    }

    pub fn priority(&self) -> u32 {
        self.act.priority
    }

    pub fn sample(&self, now: f64) -> Option<Drawing> {
        let fps = self.act.fps;
        let frame_ms = 1000.0 / (fps as f64);
        let elapsed = (now - self.start).max(0.0);
        let mut f = (elapsed / frame_ms) as u32;

        if f >= self.total {
            if !self.act.loop_act {
                return None;
            }
            if self.total > 0 {
                f %= self.total;
            }
        }

        for (i, key) in self.act.keys.iter().enumerate() {
            if f < key.hold {
                return Some(Drawing {
                    pose: key.pose,
                    index: i,
                    cue: key.cue,
                });
            }
            f -= key.hold;
        }
        None
    }

    pub fn resolve(&self, pose: &Pose, facing: f64) -> Pose {
        let mut out = *pose;
        let lean = if self.act.directional && self.dir != 0.0 {
            self.dir * facing
        } else {
            0.0
        };

        if lean != 0.0 {
            if let Some(h) = out.head {
                out.head = Some(h * lean);
            }
            if let Some(t) = out.tilt {
                out.tilt = Some(t * self.dir);
            }
            if let Some(s) = out.smear {
                out.smear = Some((s.0 * lean, s.1));
            }
            if let Some(e) = out.echo {
                out.echo = Some((e.0 * lean, e.1));
            }
        }

        if let Some(a) = out.arms {
            out.arm_l = Some(a);
            out.arm_r = Some(a);
            out.arms = None;
        }

        if let Some(r) = out.reach {
            if lean < 0.0 {
                out.arm_l = Some(r * self.reach);
            } else {
                out.arm_r = Some(r * self.reach);
            }
            out.reach = None;
        }

        if let Some(l) = out.long {
            if lean < 0.0 {
                out.long_l = Some(l);
            } else {
                out.long_r = Some(l);
            }
            out.long = None;
        }

        out
    }
}

pub fn get_act(name: &str) -> Act {
    match name {
        "stand" => Act {
            name: "stand",
            loop_act: true,
            directional: false,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(6, Pose { bob: Some(0.0), hair: Some(-2.5), arms: Some(0.04), ..Default::default() }, None),
                Key::new(4, Pose { bob: Some(-2.0), hair: Some(0.0), arms: Some(0.0), ..Default::default() }, None),
                Key::new(6, Pose { bob: Some(-4.0), hair: Some(2.5), arms: Some(-0.05), ..Default::default() }, None),
                Key::new(4, Pose { bob: Some(-2.0), hair: Some(0.0), arms: Some(0.0), ..Default::default() }, None),
            ],
        },
        "sit" => Act {
            name: "sit",
            loop_act: true,
            directional: false,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(5, Pose { kick_l: Some(0.24), kick_r: Some(-0.03), hair: Some(-1.5), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(2, Pose { kick_l: Some(0.08), kick_r: Some(0.08), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(5, Pose { kick_l: Some(-0.03), kick_r: Some(0.24), hair: Some(1.5), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(2, Pose { kick_l: Some(0.08), kick_r: Some(0.08), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(5, Pose { kick_l: Some(0.24), kick_r: Some(-0.03), hair: Some(-1.5), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(2, Pose { kick_l: Some(0.08), kick_r: Some(0.08), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(5, Pose { kick_l: Some(-0.03), kick_r: Some(0.24), hair: Some(1.5), arms: Some(-0.42), ..Default::default() }, None),
                Key::new(10, Pose { kick_l: Some(0.04), kick_r: Some(0.04), head: Some(0.08), arms: Some(-0.4), ..Default::default() }, None),
            ],
        },
        "peek" => Act {
            name: "peek",
            loop_act: true,
            directional: true,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(18, Pose { tilt: Some(7.0), head: Some(0.14), reach: Some(0.1), ..Default::default() }, None),
                Key::new(1, Pose { tilt: Some(7.0), head: Some(0.2), smear: Some((-6.0, 0.0)), reach: Some(0.16), ..Default::default() }, None),
                Key::new(14, Pose { tilt: Some(7.5), head: Some(0.24), mane: Some(0.08), reach: Some(0.18), ..Default::default() }, None),
                Key::new(1, Pose { tilt: Some(6.0), head: Some(0.14), reach: Some(0.12), ..Default::default() }, None),
                Key::new(12, Pose { tilt: Some(6.5), head: Some(0.12), bob: Some(-2.0), reach: Some(0.1), ..Default::default() }, None),
            ],
        },
        "sleep" => Act {
            name: "sleep",
            loop_act: true,
            directional: false,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(18, Pose { head: Some(0.1), mane: Some(-0.12), bob: Some(2.0), arms: Some(-0.3), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.16), mane: Some(-0.13), bob: Some(4.0), arms: Some(-0.34), ..Default::default() }, None),
                Key::new(10, Pose { head: Some(0.21), mane: Some(-0.15), bob: Some(5.0), arms: Some(-0.36), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.06), mane: Some(-0.08), bob: Some(0.0), smear: Some((0.0, 8.0)), arms: Some(-0.18), ..Default::default() }, None),
                Key::new(4, Pose { head: Some(0.08), mane: Some(-0.1), bob: Some(1.0), arms: Some(-0.26), ..Default::default() }, None),
            ],
        },
        "talk" => Act {
            name: "talk",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(3, Pose { head: Some(0.04), mane: Some(0.08), ..Default::default() }, None),
                Key::new(3, Pose { head: Some(0.0), mane: Some(0.03), ..Default::default() }, None),
                Key::new(3, Pose { head: Some(0.04), mane: Some(0.08), ..Default::default() }, None),
                Key::new(3, Pose { head: Some(0.0), mane: Some(0.03), ..Default::default() }, None),
            ],
        },
        "curious" => Act {
            name: "curious",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 12,
            priority: 1,
            keys: vec![
                Key::new(1, Pose { head: Some(-0.05), mane: Some(0.04), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.12), mane: Some(0.15), tilt: Some(1.0), smear: Some((-12.0, 4.0)), ..Default::default() }, None),
                Key::new(10, Pose { head: Some(0.19), mane: Some(0.14), tilt: Some(2.0), ..Default::default() }, None),
                Key::new(6, Pose { head: Some(0.16), mane: Some(0.1), tilt: Some(1.6), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.07), tilt: Some(0.6), ..Default::default() }, None),
            ],
        },
        "look" => Act {
            name: "look",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 12,
            priority: 1,
            keys: vec![
                Key::new(1, Pose { head: Some(0.08), smear: Some((-8.0, 0.0)), ..Default::default() }, None),
                Key::new(8, Pose { head: Some(0.16), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.0), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(-0.08), smear: Some((8.0, 0.0)), ..Default::default() }, None),
                Key::new(8, Pose { head: Some(-0.16), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(-0.05), ..Default::default() }, None),
                Key::new(3, Pose { head: Some(0.0), ..Default::default() }, None),
            ],
        },
        "spin" => Act {
            name: "spin",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(2, Pose { sy: Some(0.92), sx: Some(1.05), bob: Some(3.0), arms: Some(-0.2), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(0.7), echo: Some((8.0, 0.0)), mane: Some(0.25), arms: Some(0.15), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(0.2), echo: Some((14.0, 0.0)), mane: Some(0.35), bob: Some(-4.0), arms: Some(0.28), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.3), echo: Some((14.0, 0.0)), mane: Some(0.35), bob: Some(-6.0), arms: Some(0.3), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.75), echo: Some((10.0, 0.0)), mane: Some(0.3), bob: Some(-6.0), arms: Some(0.3), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-1.0), echo: Some((6.0, 0.0)), bob: Some(-6.0), arms: Some(0.28), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.75), echo: Some((10.0, 0.0)), mane: Some(0.3), bob: Some(-5.0), arms: Some(0.3), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.3), echo: Some((14.0, 0.0)), mane: Some(0.35), bob: Some(-4.0), arms: Some(0.3), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(0.2), echo: Some((14.0, 0.0)), mane: Some(0.35), bob: Some(-2.0), arms: Some(0.25), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(0.7), echo: Some((8.0, 0.0)), mane: Some(0.25), arms: Some(0.15), ..Default::default() }, None),
                Key::new(2, Pose { sx: Some(1.06), sy: Some(0.95), mane: Some(0.15), arms: Some(-0.06), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(1.0), ..Default::default() }, None),
            ],
        },
        "turn" => Act {
            name: "turn",
            loop_act: false,
            directional: false,
            flip: true,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(1, Pose { sx: Some(0.7), sy: Some(1.02), echo: Some((6.0, 0.0)), arms: Some(-0.1), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(0.25), sy: Some(1.04), echo: Some((10.0, 0.0)), mane: Some(0.2), arms: Some(-0.2), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.25), sy: Some(1.04), echo: Some((10.0, 0.0)), mane: Some(0.2), arms: Some(-0.2), ..Default::default() }, None),
                Key::new(1, Pose { sx: Some(-0.7), sy: Some(1.02), arms: Some(-0.1), ..Default::default() }, None),
                Key::new(2, Pose { sx: Some(-1.0), arms: Some(0.04), ..Default::default() }, None),
            ],
        },
        "hop" => Act {
            name: "hop",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(3, Pose { sy: Some(0.86), sx: Some(1.08), bob: Some(6.0), mane: Some(-0.08), arms: Some(-0.3), ..Default::default() }, None),
                Key::new(1, Pose { sy: Some(1.18), sx: Some(0.9), bob: Some(-10.0), smear: Some((0.0, 16.0)), arms: Some(0.4), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(1.08), sx: Some(0.95), bob: Some(-32.0), mane: Some(0.15), arms: Some(0.55), ..Default::default() }, None),
                Key::new(4, Pose { bob: Some(-42.0), mane: Some(0.3), arms: Some(0.6), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(1.06), bob: Some(-26.0), mane: Some(0.2), arms: Some(0.35), ..Default::default() }, None),
                Key::new(1, Pose { sy: Some(1.12), sx: Some(0.92), bob: Some(-8.0), smear: Some((0.0, -14.0)), arms: Some(0.15), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(0.82), sx: Some(1.12), bob: Some(5.0), mane: Some(-0.1), arms: Some(-0.28), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(0.94), sx: Some(1.04), bob: Some(2.0), arms: Some(0.06), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "land" => Act {
            name: "land",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(2, Pose { sy: Some(0.72), sx: Some(1.22), bob: Some(6.0), mane: Some(-0.22), arms: Some(-0.4), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(0.88), sx: Some(1.08), bob: Some(3.0), mane: Some(-0.1), arms: Some(-0.15), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(1.05), sx: Some(0.97), mane: Some(0.1), arms: Some(0.1), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "plop" => Act {
            name: "plop",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(2, Pose { sy: Some(0.85), sx: Some(1.1), mane: Some(-0.12), kick_l: Some(0.3), kick_r: Some(0.3), arms: Some(-0.55), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(1.05), sx: Some(0.97), kick_l: Some(0.12), kick_r: Some(0.12), arms: Some(-0.38), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "fall" => Act {
            name: "fall",
            loop_act: true,
            directional: false,
            flip: false,
            fps: 24,
            priority: 0,
            keys: vec![
                Key::new(2, Pose { sy: Some(1.12), sx: Some(0.92), mane: Some(0.36), head: Some(0.05), smear: Some((0.0, -16.0)), arm_l: Some(0.8), arm_r: Some(0.55), ..Default::default() }, None),
                Key::new(2, Pose { sy: Some(1.1), sx: Some(0.93), mane: Some(0.26), head: Some(-0.04), smear: Some((0.0, -12.0)), arm_l: Some(0.55), arm_r: Some(0.82), ..Default::default() }, None),
            ],
        },
        "zap" => Act {
            name: "zap",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 24,
            priority: 2,
            keys: vec![
                Key::new(3, Pose { reach: Some(0.2), long: Some(0.9), head: Some(-0.06), mane: Some(0.1), bob: Some(1.0), ..Default::default() }, None),
                Key::new(1, Pose { reach: Some(1.0), long: Some(1.6), head: Some(0.12), mane: Some(0.32), smear: Some((-10.0, 0.0)), echo: Some((6.0, 0.0)), ..Default::default() }, Some("zap")),
                Key::new(6, Pose { reach: Some(1.0), long: Some(1.52), head: Some(0.14), mane: Some(0.26), ..Default::default() }, None),
                Key::new(4, Pose { reach: Some(0.96), long: Some(1.42), head: Some(0.18), mane: Some(0.2), ..Default::default() }, Some("grin")),
                Key::new(10, Pose { reach: Some(0.9), long: Some(1.36), head: Some(0.2), mane: Some(0.16), ..Default::default() }, None),
                Key::new(3, Pose { reach: Some(0.5), long: Some(1.15), head: Some(0.14), mane: Some(0.08), ..Default::default() }, None),
                Key::new(3, Pose { reach: Some(0.15), head: Some(0.06), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "victory" => Act {
            name: "victory",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 24,
            priority: 4,
            keys: vec![
                Key::new(3, Pose { sy: Some(0.9), sx: Some(1.05), bob: Some(6.0), arms: Some(-0.3), mane: Some(-0.15), ..Default::default() }, None),
                Key::new(3, Pose { sy: Some(0.82), sx: Some(1.1), bob: Some(10.0), arms: Some(-0.5), mane: Some(-0.3), head: Some(-0.04), ..Default::default() }, Some("gather")),
                Key::new(1, Pose { sy: Some(1.24), sx: Some(0.85), bob: Some(-16.0), arms: Some(0.55), mane: Some(0.45), smear: Some((0.0, 22.0)), echo: Some((12.0, 0.0)), ..Default::default() }, Some("launch")),
                Key::new(2, Pose { sy: Some(1.1), sx: Some(0.94), bob: Some(-30.0), arms: Some(1.45), mane: Some(0.85), echo: Some((20.0, -4.0)), ..Default::default() }, Some("peak")),
                Key::new(10, Pose { bob: Some(-36.0), arms: Some(1.32), mane: Some(0.78), head: Some(0.05), ..Default::default() }, None),
                Key::new(2, Pose { bob: Some(-34.0), arms: Some(1.18), mane: Some(0.7), head: Some(-0.08), ..Default::default() }, Some("laugh")),
                Key::new(2, Pose { bob: Some(-38.0), arms: Some(1.36), mane: Some(0.8), head: Some(0.08), ..Default::default() }, None),
                Key::new(2, Pose { bob: Some(-34.0), arms: Some(1.18), mane: Some(0.7), head: Some(-0.08), ..Default::default() }, None),
                Key::new(2, Pose { bob: Some(-38.0), arms: Some(1.36), mane: Some(0.8), head: Some(0.08), ..Default::default() }, None),
                Key::new(2, Pose { bob: Some(-24.0), arms: Some(0.7), mane: Some(0.45), sy: Some(1.04), ..Default::default() }, Some("speak")),
                Key::new(2, Pose { bob: Some(-8.0), arms: Some(0.15), mane: Some(0.2), sy: Some(1.06), sx: Some(0.96), smear: Some((0.0, -12.0)), ..Default::default() }, None),
                Key::new(2, Pose { bob: Some(3.0), arms: Some(-0.15), mane: Some(-0.06), sy: Some(0.88), sx: Some(1.08), ..Default::default() }, None),
                Key::new(3, Pose { bob: Some(1.0), arms: Some(0.05), sy: Some(0.97), sx: Some(1.02), ..Default::default() }, None),
                Key::new(2, Pose { arms: Some(0.0), ..Default::default() }, None),
            ],
        },
        "flinch" => Act {
            name: "flinch",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 12,
            priority: 2,
            keys: vec![
                Key::new(2, Pose { head: Some(0.22), tilt: Some(4.0), sx: Some(0.92), sy: Some(1.06), mane: Some(-0.12), bob: Some(5.0), smear: Some((-10.0, 0.0)), arms: Some(-0.32), ..Default::default() }, None),
                Key::new(5, Pose { head: Some(0.18), tilt: Some(3.4), mane: Some(-0.1), bob: Some(4.0), arms: Some(-0.22), ..Default::default() }, None),
                Key::new(1, Pose { head: Some(0.09), tilt: Some(1.5), bob: Some(2.0), arms: Some(-0.08), ..Default::default() }, None),
            ],
        },
        "split" => Act {
            name: "split",
            loop_act: false,
            directional: false,
            flip: false,
            fps: 24,
            priority: 3,
            keys: vec![
                Key::new(2, Pose { echo: Some((3.0, 0.0)), mane: Some(0.1), ..Default::default() }, None),
                Key::new(2, Pose { echo: Some((10.0, -2.0)), mane: Some(0.18), head: Some(0.05), ..Default::default() }, None),
                Key::new(2, Pose { echo: Some((22.0, -4.0)), mane: Some(0.22), head: Some(-0.06), ..Default::default() }, Some("minds")),
                Key::new(6, Pose { echo: Some((34.0, -5.0)), mane: Some(0.24), head: Some(0.06), bob: Some(-3.0), ..Default::default() }, None),
                Key::new(2, Pose { echo: Some((41.0, -6.0)), mane: Some(0.26), head: Some(-0.05), bob: Some(-3.0), ..Default::default() }, None),
                Key::new(6, Pose { echo: Some((36.0, -5.0)), mane: Some(0.24), head: Some(0.07), bob: Some(-3.0), ..Default::default() }, None),
                Key::new(1, Pose { echo: Some((12.0, 0.0)), sx: Some(1.08), sy: Some(0.94), bob: Some(-1.0), ..Default::default() }, Some("snap")),
                Key::new(1, Pose { sx: Some(0.96), sy: Some(1.04), mane: Some(0.3), ..Default::default() }, None),
                Key::new(3, Pose { mane: Some(0.1), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "point" => Act {
            name: "point",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 24,
            priority: 2,
            keys: vec![
                Key::new(2, Pose { head: Some(0.16), mane: Some(0.15), tilt: Some(1.0), smear: Some((-10.0, 2.0)), ..Default::default() }, Some("spot")),
                Key::new(3, Pose { head: Some(0.2), mane: Some(0.12), tilt: Some(1.5), ..Default::default() }, None),
                Key::new(3, Pose { reach: Some(-0.45), long: Some(0.8), head: Some(0.12), tilt: Some(-5.0), sy: Some(0.9), sx: Some(1.06), bob: Some(5.0), mane: Some(-0.2), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(-0.55), long: Some(0.75), head: Some(0.1), tilt: Some(-6.0), sy: Some(0.86), sx: Some(1.08), bob: Some(7.0), mane: Some(-0.28), ..Default::default() }, Some("coil")),
                Key::new(1, Pose { reach: Some(0.9), long: Some(2.3), head: Some(0.24), tilt: Some(8.0), sx: Some(0.94), sy: Some(1.1), mane: Some(0.6), smear: Some((-18.0, 0.0)), echo: Some((14.0, 0.0)), ..Default::default() }, Some("lock")),
                Key::new(1, Pose { reach: Some(1.08), long: Some(2.0), head: Some(0.22), tilt: Some(7.0), mane: Some(0.55), echo: Some((8.0, 0.0)), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(1.02), long: Some(1.42), head: Some(0.19), tilt: Some(4.0), mane: Some(0.4), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(0.98), long: Some(1.58), head: Some(0.18), tilt: Some(3.0), mane: Some(0.3), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.17), tilt: Some(3.0), mane: Some(0.22), ..Default::default() }, None),
            ],
        },
        "pointing" => Act {
            name: "pointing",
            loop_act: true,
            directional: true,
            flip: false,
            fps: 12,
            priority: 0,
            keys: vec![
                Key::new(12, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.17), mane: Some(0.16), tilt: Some(3.0), ..Default::default() }, None),
                Key::new(1, Pose { reach: Some(1.05), long: Some(1.8), head: Some(0.2), mane: Some(0.24), tilt: Some(4.0), ..Default::default() }, Some("tap")),
                Key::new(2, Pose { reach: Some(1.0), long: Some(1.52), head: Some(0.18), mane: Some(0.18), tilt: Some(3.0), ..Default::default() }, None),
                Key::new(1, Pose { reach: Some(1.05), long: Some(1.8), head: Some(0.2), mane: Some(0.24), tilt: Some(4.0), ..Default::default() }, Some("tap")),
                Key::new(10, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.17), mane: Some(0.16), tilt: Some(3.0), bob: Some(-2.0), ..Default::default() }, None),
                Key::new(1, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.02), mane: Some(0.12), tilt: Some(2.0), smear: Some((10.0, 0.0)), ..Default::default() }, None),
                Key::new(8, Pose { reach: Some(1.0), long: Some(1.5), head: Some(-0.1), mane: Some(0.1), tilt: Some(2.0), ..Default::default() }, None),
                Key::new(1, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.12), mane: Some(0.16), tilt: Some(3.0), smear: Some((-10.0, 0.0)), ..Default::default() }, None),
                Key::new(8, Pose { reach: Some(1.0), long: Some(1.5), head: Some(0.17), mane: Some(0.16), tilt: Some(3.0), bob: Some(-1.0), ..Default::default() }, None),
            ],
        },
        "unpoint" => Act {
            name: "unpoint",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 24,
            priority: 2,
            keys: vec![
                Key::new(1, Pose { reach: Some(0.6), long: Some(0.9), head: Some(0.08), tilt: Some(-2.0), smear: Some((12.0, 0.0)), mane: Some(0.1), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(0.1), long: Some(0.85), head: Some(0.04), tilt: Some(-1.0), ..Default::default() }, None),
                Key::new(2, Pose { reach: Some(-0.06), long: Some(1.0), tilt: Some(0.0), ..Default::default() }, None),
                Key::new(1, Pose::default(), None),
            ],
        },
        "alarm" => Act {
            name: "alarm",
            loop_act: false,
            directional: true,
            flip: false,
            fps: 12,
            priority: 2,
            keys: vec![
                Key::new(1, Pose { mane: Some(0.34), head: Some(0.1), tilt: Some(3.0), arms: Some(0.36), ..Default::default() }, Some("stare")),
                Key::new(1, Pose { mane: Some(0.12), head: Some(0.1), tilt: Some(3.0), arms: Some(0.24), ..Default::default() }, None),
                Key::new(1, Pose { mane: Some(0.34), head: Some(0.1), tilt: Some(3.0), arms: Some(0.36), ..Default::default() }, None),
                Key::new(1, Pose { mane: Some(0.12), head: Some(0.1), tilt: Some(3.0), arms: Some(0.24), ..Default::default() }, None),
                Key::new(1, Pose { mane: Some(0.34), head: Some(0.1), tilt: Some(3.0), arms: Some(0.36), ..Default::default() }, None),
                Key::new(1, Pose { mane: Some(0.12), head: Some(0.1), tilt: Some(3.0), arms: Some(0.24), ..Default::default() }, None),
            ],
        },
        _ => get_act("stand"),
    }
}
