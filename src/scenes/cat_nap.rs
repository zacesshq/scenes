//! Cat nap: a cat asleep on a cushion, its back rising and falling slowly as it breathes and its
//! tail stirring now and then, with a z drifting up from it, a ball of wool on the rug and a
//! night window above.

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window";

pub const CAPTIONS: &[&str] = &[
    "Nothing to wake up for. The build is fine.",
    "Clean build. Back to sleep.",
    "Even the cat approves.",
    "All quiet. Do not disturb.",
    "Purring along nicely.",
];

/// Seconds one breath takes. The back changes shape twice a breath, once every two seconds at
/// the standard pace.
pub const BREATH_PERIOD: f32 = 4.0;
/// Seconds the tail takes to stir and settle again.
pub const TAIL_PERIOD: f32 = 7.0;
/// Seconds between two z's.
const Z_GAP: f32 = 3.0;
/// Seconds a z lasts.
const Z_LIFE: f32 = 5.0;

const CAT_OUT: [&str; 4] = [
    "          /\\_/\\  ",
    "  .------( -.- ) ",
    " /        `-^-' \\",
    "(________________)",
];
const CAT_IN: [&str; 4] = [
    "    .--.  /\\_/\\  ",
    "  .'    `( -.- ) ",
    " /        `-^-' \\",
    "(________________)",
];

#[derive(Clone, Debug)]
struct Zed {
    x: f32,
    y: f32,
    age: f32,
}

pub struct CatNap {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    zeds: Vec<Zed>,
    since_z: f32,
    stars: Vec<(i32, i32)>,
}

impl CatNap {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut nap = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            zeds: Vec::new(),
            since_z: 0.0,
            stars: Vec::new(),
        };
        nap.resize(width, height, rng);
        nap
    }

    fn breath_period(&self) -> f32 {
        BREATH_PERIOD / self.motion.pace()
    }

    fn tail_period(&self) -> f32 {
        TAIL_PERIOD / self.motion.pace()
    }

    /// Whether the cat is breathing in at the moment.
    fn breathing_in(&self) -> bool {
        (self.t / f64::from(self.breath_period())).rem_euclid(1.0) < 0.5
    }

    /// The cat's top left corner.
    fn cat_at(&self) -> (i32, i32) {
        let w = self.width as i32;
        let h = self.height as i32;
        // the cushion reaches one column either side of the cat, so the cat keeps a column of
        // room on the left and the cushion's right end stays on screen
        let x = (w * 11 / 20 - 9).clamp(1, (w - 19).max(1));
        (x, h - 2 - CAT_OUT.len() as i32)
    }

    /// The window's left, top, width and height, when there is room for it.
    fn window(&self) -> Option<(i32, i32, i32, i32)> {
        let (_, cat_y) = self.cat_at();
        let height = (cat_y - 2).min(10);
        (self.width >= 36 && height >= 5)
            .then(|| (3, 1, (self.width as i32 / 4).clamp(14, 30), height))
    }
}

impl CatNap {
    /// A bookcase against the right wall, on a screen wide enough to keep it clear of the cat.
    fn draw_shelf(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;
        if w < 70 || h < 16 {
            return;
        }
        let left = w - 24;
        let right = w - 5;
        let shelves = ((h - 6) / 4).min(4);
        let top = h - 1 - shelves * 4;
        let colours = [
            theme.red(),
            theme.blue(),
            theme.green(),
            theme.yellow(),
            theme.magenta(),
            theme.cyan(),
        ];
        for y in top + 1..h - 1 {
            frame.put(left, y, '\u{2502}', Some(theme.yellow()));
            frame.put(right, y, '\u{2502}', Some(theme.yellow()));
        }
        for x in left..=right {
            frame.put(x, top, '_', Some(theme.yellow()));
        }
        for shelf in 0..shelves {
            let base = top + 4 * shelf + 3;
            for x in left..=right {
                frame.put(x, base, '=', Some(theme.yellow()));
            }
            // books of a fixed pattern per shelf, so the case looks lived in but never changes
            let mut x = left + 1;
            let mut i = shelf as usize * 3;
            while x < right - 1 {
                let tall = i % 3 != 1;
                let colour = colours[(i * 7 + shelf as usize) % colours.len()];
                let spine = if i % 4 == 0 { '[' } else { '|' };
                frame.put(x, base - 1, spine, Some(colour));
                if tall {
                    frame.put(x, base - 2, spine, Some(colour));
                }
                x += if i % 5 == 4 { 3 } else { 1 };
                i += 1;
            }
        }
    }
}

impl Scene for CatNap {
    fn name(&self) -> &'static str {
        "cat-nap"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.zeds.clear();
        self.stars.clear();
        if let Some((left, top, w, h)) = self.window() {
            let inner_w = (w - 2) as usize;
            let inner_h = (h - 2) as usize;
            let wanted = count_for(inner_w, inner_h, 12, 2, 30);
            self.stars = (0..wanted)
                .map(|_| {
                    (
                        left + 1 + rng.below(inner_w) as i32,
                        top + 1 + rng.below(inner_h) as i32,
                    )
                })
                .collect();
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.since_z += dt;
        if self.since_z >= Z_GAP / self.motion.pace() {
            self.since_z = 0.0;
            let (x, y) = self.cat_at();
            self.zeds.push(Zed {
                x: (x + 16) as f32,
                y: (y - 1) as f32,
                age: 0.0,
            });
        }
        for z in &mut self.zeds {
            z.age += dt;
            z.x += 0.45 * speed * dt;
            z.y -= 0.6 * speed * dt;
        }
        self.zeds.retain(|z| z.age < Z_LIFE && z.y >= 0.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;

        if let Some((left, top, ww, wh)) = self.window() {
            let right = left + ww - 1;
            let bottom = top + wh - 1;
            let colour = Some(theme.muted);
            for x in left..=right {
                frame.put(x, top, '\u{2500}', colour);
                frame.put(x, bottom, '\u{2500}', colour);
            }
            for y in top..=bottom {
                frame.put(left, y, '\u{2502}', colour);
                frame.put(right, y, '\u{2502}', colour);
            }
            frame.put(left, top, '\u{250c}', colour);
            frame.put(right, top, '\u{2510}', colour);
            frame.put(left, bottom, '\u{2514}', colour);
            frame.put(right, bottom, '\u{2518}', colour);
            for &(x, y) in &self.stars {
                frame.put(x, y, '.', Some(theme.foreground));
            }
            frame.put_sprite(
                right - 5,
                top + 1,
                &[" _", "( ", " `"],
                Some(theme.bright_yellow()),
            );
            // a sill under the window
            for x in left - 1..=right + 1 {
                frame.put(x, bottom + 1, '=', Some(theme.yellow()));
            }
        }

        // the rug, in two colours so its pattern reads without colour too
        let floor = h - 1;
        for x in 0..w {
            let (ch, colour) = if x % 4 < 2 {
                ('=', theme.red())
            } else {
                ('-', theme.yellow())
            };
            frame.put(x, floor, ch, Some(colour));
        }

        let (cat_x, cat_y) = self.cat_at();
        let cushion_y = cat_y + CAT_OUT.len() as i32;
        let cushion_w = CAT_OUT[3].len() as i32 + 2;
        frame.put(cat_x - 1, cushion_y, '(', Some(theme.magenta()));
        for x in cat_x..cat_x + cushion_w - 2 {
            frame.put(x, cushion_y, '#', Some(theme.magenta()));
        }
        frame.put(cat_x + cushion_w - 2, cushion_y, ')', Some(theme.magenta()));

        let sprite = if self.breathing_in() {
            &CAT_IN
        } else {
            &CAT_OUT
        };
        frame.put_sprite(cat_x, cat_y, sprite, Some(theme.foreground));
        // the nose in a warmer colour
        frame.put(cat_x + 12, cat_y + 1, '.', Some(theme.red()));
        let stir = (self.t / f64::from(self.tail_period())).rem_euclid(1.0) < 0.3;
        let tail = if stir { '~' } else { '_' };
        frame.put(
            cat_x + CAT_OUT[3].len() as i32,
            cat_y + 3,
            tail,
            Some(theme.foreground),
        );

        self.draw_shelf(frame, theme);

        if w >= 40 {
            let wool_x = cat_x + cushion_w + 4;
            frame.put_str(wool_x, floor - 1, "(@)", Some(theme.cyan()));
            frame.put_str(wool_x - 3, floor - 1, "~.~", Some(theme.cyan()));
        }

        for z in &self.zeds {
            let late = z.age > Z_LIFE * 0.75;
            let (ch, colour) = if late {
                ('z', theme.grey())
            } else if z.age > Z_LIFE * 0.35 {
                ('Z', theme.muted)
            } else {
                ('z', theme.muted)
            };
            frame.put(z.x.round() as i32, z.y.round() as i32, ch, Some(colour));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("cat-nap");
        check_determinism("cat-nap");
    }

    #[test]
    fn the_cat_breathes_slowly_and_its_tail_stirs_rarely() {
        let mut rng = Rng::new(3);
        for motion in Motion::ALL {
            let nap = CatNap::new(80, 24, motion, &mut rng);
            assert!(nap.breath_period() >= BREATH_PERIOD);
            assert!(nap.tail_period() >= TAIL_PERIOD);
        }
        let mut nap = CatNap::new(80, 24, Motion::FULL, &mut rng);
        let mut shapes = Vec::new();
        for _ in 0..(BREATH_PERIOD as usize * 20) {
            nap.step(0.05, &mut rng);
            shapes.push(nap.breathing_in());
        }
        let changes = shapes.windows(2).filter(|w| w[0] != w[1]).count();
        assert!((1..=2).contains(&changes), "{changes}");
    }

    #[test]
    fn a_z_drifts_up_from_the_cat() {
        let mut rng = Rng::new(3);
        let mut nap = CatNap::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut nap, 60, 20, 100);
        let text = frame.to_text();
        assert!(text.contains("-.-"), "{text}");
        assert!(text.contains('z') || text.contains('Z'), "{text}");
        let small = render(&mut CatNap::new(20, 8, Motion::CALM, &mut rng), 20, 8, 10);
        assert!(small.to_text().contains("-.-"), "{}", small.to_text());
        // the whole cushion fits, both ends included
        assert!(small.row_text(6).starts_with('('), "{}", small.to_text());
        assert!(small.row_text(6).ends_with(')'), "{}", small.to_text());
    }
}
