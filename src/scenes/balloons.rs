//! Balloons: hot air balloons at three distances lifting slowly from behind rolling hills and
//! drifting on a light wind, under clouds that move slower still. Each balloon rises from behind
//! the ridge it belongs to, so the hills give the sky its depth.

use super::{count_for, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Hot air balloons lifting slowly from behind rolling hills under drifting clouds";

pub const CAPTIONS: &[&str] = &[
    "Lift off. Everything passed.",
    "Rising gently on a clean build.",
    "Nothing is holding this build down.",
    "Clear skies and light winds.",
    "All ropes cast off, all checks green.",
];

const LARGE: [&str; 7] = [
    " .---. ", "/ | | \\", "| | | |", "\\ | | /", " \\_|_/ ", "  \\ /  ", "  [_]  ",
];
const MEDIUM: [&str; 5] = [" .-. ", "/ | \\", "\\ | /", " \\ / ", "  =  "];
const SMALL: [&str; 2] = ["(|)", " ' "];
const CLOUD: [&str; 2] = ["  .--.   ", ".(    ).-."];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    fn rows(self) -> &'static [&'static str] {
        match self {
            Size::Small => &SMALL,
            Size::Medium => &MEDIUM,
            Size::Large => &LARGE,
        }
    }

    /// The hill a balloon of this size rises from behind: the far, middle or near ridge.
    fn layer(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
struct Balloon {
    x: f32,
    y: f32,
    drift: f32,
    rise: f32,
    size: Size,
    colour: usize,
}

pub struct Balloons {
    width: usize,
    height: usize,
    motion: Motion,
    balloons: Vec<Balloon>,
    clouds: Vec<(f32, i32)>,
    /// The top row of each hill per column: far, middle and near.
    ridges: [Vec<i32>; 3],
    /// Tufts and hedges on the nearer hills, as column, row and character.
    texture: Vec<(i32, i32, char, usize)>,
}

impl Balloons {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sky = Self {
            width: 1,
            height: 1,
            motion,
            balloons: Vec::new(),
            clouds: Vec::new(),
            ridges: [Vec::new(), Vec::new(), Vec::new()],
            texture: Vec::new(),
        };
        sky.resize(width, height, rng);
        sky
    }

    fn sizes(&self) -> &'static [Size] {
        if self.height >= 24 && self.width >= 40 {
            &[
                Size::Small,
                Size::Small,
                Size::Medium,
                Size::Medium,
                Size::Large,
            ]
        } else if self.height >= 12 {
            &[Size::Small, Size::Small, Size::Medium]
        } else {
            &[Size::Small]
        }
    }

    /// The lowest row of a hill, so a balloon can be launched from behind it.
    fn ridge_low(&self, layer: usize) -> i32 {
        self.ridges[layer].iter().copied().max().unwrap_or(0)
    }

    fn launch(&self, rng: &mut Rng, anywhere: bool) -> Balloon {
        let size = *rng.pick(self.sizes());
        let rows = size.rows();
        let wide = rows[0].len() as f32;
        let rise = match size {
            Size::Small => rng.range(0.15, 0.25),
            Size::Medium => rng.range(0.25, 0.4),
            Size::Large => rng.range(0.35, 0.5),
        } * self.motion.speed();
        let low = self.ridge_low(size.layer()) as f32;
        // a few tries for a column clear of the other balloons of the same size, so two never
        // sit on top of each other
        let mut x = rng.range(0.0, (self.width as f32 - wide).max(1.0));
        for _ in 0..4 {
            let clear = self
                .balloons
                .iter()
                .all(|b| b.size != size || (b.x - x).abs() > wide + 1.0);
            if clear {
                break;
            }
            x = rng.range(0.0, (self.width as f32 - wide).max(1.0));
        }
        Balloon {
            x,
            y: if anywhere {
                rng.range(-(rows.len() as f32), low)
            } else {
                low + rng.range(0.0, 3.0)
            },
            drift: rng.range(-0.35, 0.35) * self.motion.speed(),
            rise,
            size,
            colour: rng.below(5),
        }
    }

    fn envelope(theme: &ThemeData, colour: usize) -> (Rgb, Rgb) {
        match colour {
            0 => (theme.bright_red(), theme.bright_yellow()),
            1 => (theme.bright_yellow(), theme.red()),
            2 => (theme.bright_cyan(), theme.foreground),
            3 => (theme.bright_magenta(), theme.bright_yellow()),
            _ => (theme.bright_green(), theme.foreground),
        }
    }

    fn ridge_char(row: &[i32], x: usize) -> char {
        let here = row[x];
        let left = if x > 0 { row[x - 1] } else { here };
        let right = row.get(x + 1).copied().unwrap_or(here);
        if right < here && left >= here {
            '/'
        } else if left < here && right >= here {
            '\\'
        } else {
            '_'
        }
    }
}

impl Scene for Balloons {
    fn name(&self) -> &'static str {
        "balloons"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.height as f32;
        let bottom = self.height as i32 - 1;
        let bases = [0.55, 0.72, 0.88];
        let amps = [0.12, 0.08, 0.05];
        for layer in 0..3 {
            let p1 = rng.range(0.0, std::f32::consts::TAU);
            let p2 = rng.range(0.0, std::f32::consts::TAU);
            let f = rng.range(0.05, 0.09) * (1.0 + layer as f32 * 0.3);
            self.ridges[layer] = (0..self.width)
                .map(|x| {
                    let x = x as f32;
                    let wave = (x * f + p1).sin() + 0.5 * (x * f * 2.3 + p2).sin();
                    // at least a row or so of rise and fall, so a short screen still has hills
                    let amp = (amps[layer] * h).max(1.2);
                    (bases[layer] * h + amp * wave).round() as i32
                })
                .map(|y| y.clamp(1.min(bottom), bottom))
                .collect();
        }
        self.texture.clear();
        for layer in 1..3 {
            for x in 0..self.width {
                let top = self.ridges[layer][x];
                for y in top + 1..=bottom {
                    if rng.chance(if layer == 2 { 0.12 } else { 0.06 }) {
                        let ch = *rng.pick(&[',', '\'', '"', '.']);
                        self.texture.push((x as i32, y, ch, layer));
                    }
                }
                // a hedge or a tree here and there along the middle ridge
                if layer == 1 && rng.chance(0.06) && top >= 1 {
                    self.texture.push((x as i32, top - 1, '^', 0));
                }
            }
        }
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 200, 2, 16));
        self.balloons.clear();
        for _ in 0..wanted {
            let balloon = self.launch(rng, true);
            self.balloons.push(balloon);
        }
        let clouds = count_for(self.width, self.height, 500, 1, 8);
        let sky = (self.ridges[0].iter().copied().min().unwrap_or(2) - 2).max(1);
        self.clouds = (0..clouds)
            .map(|_| {
                (
                    rng.range(-10.0, self.width as f32),
                    rng.below(sky as usize) as i32,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let width = self.width as f32;
        for i in 0..self.balloons.len() {
            let b = &mut self.balloons[i];
            b.y -= b.rise * dt;
            b.x += b.drift * dt;
            let wide = b.size.rows()[0].len() as f32;
            if b.y < -(b.size.rows().len() as f32) - 1.0 || b.x < -wide - 1.0 || b.x > width + 1.0 {
                self.balloons[i] = self.launch(rng, false);
            }
        }
        let wind = 0.2 * self.motion.speed();
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + wind * dt + 12.0, width + 24.0) - 12.0;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let bottom = self.height as i32 - 1;
        for &(x, y) in &self.clouds {
            frame.put_sprite(x.round() as i32, y, &CLOUD, Some(theme.muted));
        }
        let ridge_colours = [theme.blue(), theme.green(), theme.bright_green()];
        for (layer, &ridge_colour) in ridge_colours.iter().enumerate() {
            // balloons rising from behind this hill are drawn before it, so it hides them
            for b in self.balloons.iter().filter(|b| b.size.layer() == layer) {
                let (body, gore) = Self::envelope(theme, b.colour);
                let (x, y) = (b.x.round() as i32, b.y.round() as i32);
                let rows = b.size.rows();
                // the envelope is solid, so nothing behind it shows through
                for (dy, row) in rows.iter().enumerate() {
                    let first = row.find(|c| c != ' ').unwrap_or(0);
                    let last = row.rfind(|c| c != ' ').unwrap_or(0);
                    for dx in first..=last {
                        frame.put(x + dx as i32, y + dy as i32, ' ', None);
                    }
                }
                frame.put_sprite(x, y, rows, Some(body));
                match b.size {
                    Size::Large => {
                        for dy in 1..4 {
                            frame.put(x + 2, y + dy, '|', Some(gore));
                            frame.put(x + 4, y + dy, '|', Some(gore));
                        }
                        frame.put_str(x + 2, y + 6, "[_]", Some(theme.yellow()));
                        frame.put_str(x + 2, y + 5, "\\ /", Some(theme.muted));
                    }
                    Size::Medium => {
                        frame.put(x + 2, y + 1, '|', Some(gore));
                        frame.put(x + 2, y + 2, '|', Some(gore));
                        frame.put(x + 2, y + 4, '=', Some(theme.yellow()));
                    }
                    Size::Small => {
                        frame.put(x + 1, y, '|', Some(gore));
                        frame.put(x + 1, y + 1, '\'', Some(theme.yellow()));
                    }
                }
            }
            let ridge = &self.ridges[layer];
            for x in 0..self.width {
                let top = ridge[x];
                for y in top + 1..=bottom {
                    frame.put(x as i32, y, ' ', None);
                }
                frame.put(
                    x as i32,
                    top,
                    Self::ridge_char(ridge, x),
                    Some(ridge_colour),
                );
            }
            // tufts belong to their own hill; the trees on the middle ridge are stored with
            // layer 0 and drawn along with the middle hill
            for &(x, y, ch, l) in &self.texture {
                if (l == layer && layer > 0) || (l == 0 && layer == 1) {
                    frame.put(x, y, ch, Some(theme.green()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("balloons");
        check_determinism("balloons");
    }

    #[test]
    fn balloons_rise_and_the_hills_stay_put() {
        let mut rng = Rng::new(5);
        let mut sky = Balloons::new(80, 24, Motion::LIVELY, &mut rng);
        let ridges = sky.ridges.clone();
        let before: Vec<f32> = sky.balloons.iter().map(|b| b.y).collect();
        sky.step(1.0, &mut rng);
        for (b, y) in sky.balloons.iter().zip(before) {
            assert!(b.y < y || b.y > 10.0);
        }
        // the fastest balloon climbs well under a row a second, so nothing darts about
        assert!(
            sky.balloons
                .iter()
                .all(|b| b.rise <= 0.5 * Motion::LIVELY.speed())
        );
        assert_eq!(sky.ridges, ridges);
    }

    #[test]
    fn every_size_fits_its_screen() {
        let mut rng = Rng::new(9);
        let mut big = Balloons::new(100, 30, Motion::FULL, &mut rng);
        assert!(big.sizes().contains(&Size::Large));
        let frame = render(&mut big, 100, 30, 300);
        assert!(frame.to_text().contains('|'));
        let small = Balloons::new(20, 8, Motion::FULL, &mut rng);
        assert_eq!(small.sizes(), &[Size::Small]);
    }
}
