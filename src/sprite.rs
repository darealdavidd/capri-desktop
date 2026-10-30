// Capri's sprite renderer and hit testing.
// Manages the 240x420 sprite cut-out, anime transformations,
// hit-testing, and alpha mapping.

use gdk::prelude::GdkContextExt;
use gdk_pixbuf::Pixbuf;
use std::path::Path;

pub const SPRITE_WIDTH: f64 = 240.0;
pub const SPRITE_HEIGHT: f64 = 420.0;
pub const SPRITE_PAD: f64 = 56.0;

pub const LANDMARK_HEAD: (f64, f64) = (116.0, 132.0);
pub const LANDMARK_FEET: (f64, f64) = (128.0, 418.0);
pub const LANDMARK_HIP: f64 = 232.0;
pub const LANDMARK_SHOULDER_L: (f64, f64) = (98.0, 168.0);
pub const LANDMARK_SHOULDER_R: (f64, f64) = (136.0, 168.0);
pub const LANDMARK_HAND_L: (f64, f64) = (24.0, 212.0);
pub const LANDMARK_MIRROR: f64 = 234.0;

#[derive(Debug, Clone)]
pub struct SpriteParams {
    pub seed: f64,
    pub boil: f64,
    pub hair: f64,
    pub head: f64,
    pub mane: f64,
    pub arm_l: f64,
    pub arm_r: f64,
    pub long_l: f64,
    pub long_r: f64,
    pub sit: f64,
    pub kick_l: f64,
    pub kick_r: f64,
    pub smear: (f64, f64),
    pub echo: (f64, f64),
    pub glitch: f64,
    pub glitch_seed: f64,
    pub split: f64,
    pub collapse: f64,
    pub dissolve: f64,
    pub sparkle: f64,
    pub grade: f64,
    pub sil: [f64; 3],
    pub sil_amount: f64,
    pub misreg: (f64, f64),
    pub alpha: f64,
}

impl Default for SpriteParams {
    fn default() -> Self {
        Self {
            seed: 0.0,
            boil: 0.9,
            hair: 0.0,
            head: 0.0,
            mane: 0.0,
            arm_l: 0.0,
            arm_r: 0.0,
            long_l: 1.0,
            long_r: 1.0,
            sit: 0.0,
            kick_l: 0.0,
            kick_r: 0.0,
            smear: (0.0, 0.0),
            echo: (0.0, 0.0),
            glitch: 0.0,
            glitch_seed: 0.0,
            split: 0.0,
            collapse: 0.0,
            dissolve: 0.0,
            sparkle: 1.0,
            grade: 1.0,
            sil: [0.08, 0.06, 0.11],
            sil_amount: 0.0,
            misreg: (-2.6, 2.2),
            alpha: 1.0,
        }
    }
}

pub struct SpriteRenderer {
    pub pixbuf: Pixbuf,
    alpha_map: Vec<u8>,
    width: i32,
    height: i32,
}

impl SpriteRenderer {
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        let pixbuf = Pixbuf::from_file(path)?;
        let width = pixbuf.width();
        let height = pixbuf.height();
        let n_channels = pixbuf.n_channels();
        let rowstride = pixbuf.rowstride() as usize;

        // Build alpha map
        let mut alpha_map = vec![0u8; (width * height) as usize];
        let pixels = unsafe { pixbuf.pixels() };

        for y in 0..height {
            for x in 0..width {
                let offset = (y as usize) * rowstride + (x as usize) * (n_channels as usize);
                let alpha = if n_channels >= 4 {
                    pixels[offset + 3]
                } else {
                    255
                };
                alpha_map[(y * width + x) as usize] = alpha;
            }
        }

        // Add mirrored right arm to alpha map
        for y in 0..height {
            for x in 0..width {
                if is_arm_l(x as f64, y as f64) {
                    let mx = (LANDMARK_MIRROR - x as f64) as i32;
                    if mx >= 0 && mx < width {
                        let to = (y * width + mx) as usize;
                        let from = (y * width + x) as usize;
                        alpha_map[to] = alpha_map[to].max(alpha_map[from]);
                    }
                }
            }
        }

        Ok(Self {
            pixbuf,
            alpha_map,
            width,
            height,
        })
    }

    pub fn hit_test(
        &self,
        x: f64,
        y: f64,
        pos_x: f64,
        pos_y: f64,
        scale: f64,
        facing: f64,
        bob: f64,
        sit: f64,
    ) -> bool {
        let fy = pos_y + bob;
        let sx = LANDMARK_FEET.0 + ((x - pos_x) / scale) * facing;
        let mut sy = LANDMARK_FEET.1 - (fy - y) / scale;

        if sit > 0.0 && sy > LANDMARK_HIP {
            let c = 1.0 - 0.7 * sit;
            sy = if sy <= LANDMARK_HIP + 73.0 * c {
                LANDMARK_HIP + (sy - LANDMARK_HIP) / c
            } else {
                sy + 73.0 * (1.0 - c)
            };
        }

        let px = sx.round() as i32;
        let py = sy.round() as i32;

        if px < 0 || py < 0 || px >= self.width || py >= self.height {
            return false;
        }

        self.alpha_map[(py * self.width + px) as usize] > 40
    }

    pub fn draw(
        &self,
        cr: &cairo::Context,
        pos_x: f64,
        pos_y: f64,
        scale: f64,
        facing: f64,
        bob: f64,
        tilt: f64,
        sx: f64,
        sy: f64,
        p: &SpriteParams,
    ) {
        if p.alpha <= 0.01 {
            return;
        }

        cr.save().unwrap();

        // Pivot on feet
        let foot_x = (SPRITE_PAD + LANDMARK_FEET.0) * scale;
        let foot_y = (SPRITE_PAD + if p.sit > 0.0 { LANDMARK_HIP } else { LANDMARK_FEET.1 }) * scale;



        cr.translate(pos_x, pos_y + bob);
        cr.rotate(tilt.to_radians());
        cr.scale(facing * sx * scale, sy * scale);
        cr.translate(-foot_x / scale, -foot_y / scale);

        // Teleport collapse effect (horizontal squish into a light beam)
        if p.collapse > 0.0 {
            let col = (1.0 - p.collapse).max(0.04);
            cr.translate(SPRITE_WIDTH / 2.0, SPRITE_HEIGHT / 2.0);
            cr.scale(col, 1.0 + p.collapse * 0.4);
            cr.translate(-SPRITE_WIDTH / 2.0, -SPRITE_HEIGHT / 2.0);
        }

        // Violet misregistration ghost behind figure
        if p.misreg.0 != 0.0 || p.misreg.1 != 0.0 {
            cr.save().unwrap();
            cr.translate(p.misreg.0, p.misreg.1);
            cr.set_source_pixbuf(&self.pixbuf, SPRITE_PAD, SPRITE_PAD);
            let _ = cr.paint_with_alpha(0.35 * p.alpha);
            cr.restore().unwrap();
        }

        // Afterimages (green and red copies pulled apart)
        if p.echo.0 != 0.0 || p.echo.1 != 0.0 {
            cr.save().unwrap();
            cr.translate(-p.echo.0, -p.echo.1);
            cr.set_source_pixbuf(&self.pixbuf, SPRITE_PAD, SPRITE_PAD);
            let _ = cr.paint_with_alpha(0.3 * p.alpha);
            cr.restore().unwrap();

            cr.save().unwrap();
            cr.translate(p.echo.0, p.echo.1);
            cr.set_source_pixbuf(&self.pixbuf, SPRITE_PAD, SPRITE_PAD);
            let _ = cr.paint_with_alpha(0.3 * p.alpha);
            cr.restore().unwrap();
        }

        // Smear trailing ghost
        if p.smear.0 != 0.0 || p.smear.1 != 0.0 {
            cr.save().unwrap();
            cr.translate(p.smear.0 * 0.5, p.smear.1 * 0.5);
            cr.set_source_pixbuf(&self.pixbuf, SPRITE_PAD, SPRITE_PAD);
            let _ = cr.paint_with_alpha(0.4 * p.alpha);
            cr.restore().unwrap();
        }

        // Main sprite
        cr.set_source_pixbuf(&self.pixbuf, SPRITE_PAD, SPRITE_PAD);

        if p.sil_amount > 0.0 {
            // Flash silhouette color
            cr.set_source_rgba(p.sil[0], p.sil[1], p.sil[2], p.sil_amount * p.alpha);
            let _ = cr.mask(&cairo::SurfacePattern::create(&cairo::ImageSurface::create_for_data(
                vec![255u8; 4],
                cairo::Format::ARgb32,
                1,
                1,
                4,
            ).unwrap()));
        } else {
            let _ = cr.paint_with_alpha(p.alpha);
        }

        cr.restore().unwrap();
    }
}

pub fn is_arm_l(x: f64, y: f64) -> bool {
    if x > 101.0 || y > 234.0 {
        return false;
    }
    let top = 159.0 + (98.0 - x) * 0.5;
    let bot = if x < 50.0 {
        234.0
    } else {
        178.0 + (98.0 - x) * 0.62
    };
    y >= top && y <= bot
}
