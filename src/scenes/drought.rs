//! Drought: cracked earth under a heavy sun. The cracks spread across the ground one plate edge
//! at a time over about a minute, the last puddle in a dry riverbed shrinks away and a withered
//! tree stands on the horizon. Once dry it stays dry, with dust drifting past and a lone bird
//! crossing the sky. The sun is still: there is no heat shimmer.

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::warning_colour;

pub const SUMMARY: &str = "Cracked earth spreading under a heavy sun, a dry riverbed with its last puddle and a bare tree";

pub const CAPTIONS: &[&str] = &[
    "Dry spell. The first error is the place to dig.",
    "Nothing grows until it builds. Start at the top.",
    "A long dry patch. One fix brings the rain.",
    "Parched, not lost. Read the first message.",
    "The ground cracked. Look for where it started.",
];

/// Seconds for the cracks to spread across the whole ground at the standard pace.
pub const CRACK_SECONDS: f32 = 60.0;
/// Seconds between the puddle losing one cell at the standard pace.
pub const PUDDLE_SHRINK: f32 = 6.0;
/// The share of the cracks already there when the scene starts.
const FIRST_CRACKS: f32 = 0.3;

/// One crack cell: where it is, its character and when it appears, from 0 to 1.
#[derive(Clone, Copy, Debug)]
struct Crack {
    x: i32,
    y: i32,
    ch: char,
    at: f32,
}

#[derive(Clone, Copy, Debug)]
struct Mote {
    x: f32,
    y: f32,
    speed: f32,
    ch: char,
}

pub struct Drought {
    width: usize,
    height: usize,
    motion: Motion,
    /// How far the cracks have spread, from 0 to 1.
    progress: f32,
    /// Seconds the puddle has been drying.
    drying: f32,
    cracks: Vec<Crack>,
    /// The top row of the riverbed in each column.
    river: Vec<i32>,
    pebbles: Vec<(i32, i32)>,
    dust: Vec<Mote>,
    bird: f32,
    t: f64,
}

/// A hash of two numbers into `[0, 1)`, so a whole plate edge shares one moment to appear.
fn hash(a: usize, b: usize) -> f32 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 29;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 32;
    (z % 10_000) as f32 / 10_000.0
}

impl Drought {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut land = Self {
            width: 1,
            height: 1,
            motion,
            progress: FIRST_CRACKS,
            drying: 0.0,
            cracks: Vec::new(),
            river: Vec::new(),
            pebbles: Vec::new(),
            dust: Vec::new(),
            bird: 0.0,
            t: 0.0,
        };
        land.resize(width, height, rng);
        land
    }

    fn horizon(&self) -> i32 {
        ((self.height as f32 * 0.42) as i32).max(2)
    }

    /// The puddle's starting length.
    fn puddle_start(&self) -> i32 {
        (self.width as i32 / 10).clamp(3, 12)
    }

    /// The puddle's length now: one cell less every `PUDDLE_SHRINK` seconds.
    fn puddle(&self) -> i32 {
        let lost = (self.drying / PUDDLE_SHRINK) as i32;
        (self.puddle_start() - lost).max(0)
    }

    fn in_river(&self, x: i32, y: i32) -> bool {
        self.river
            .get(x as usize)
            .is_some_and(|&top| (top - 1..top + 3).contains(&y))
    }
}

impl Scene for Drought {
    fn name(&self) -> &'static str {
        "drought"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let horizon = self.horizon();
        let ground_top = horizon + 1;
        let rows = (self.height as i32 - ground_top).max(0);

        // the riverbed meanders across the lower half of the ground
        let mid = ground_top + rows * 3 / 5;
        let phase = rng.range(0.0, 6.0);
        self.river = (0..self.width)
            .map(|x| {
                let bend = ((x as f32 / 11.0 + phase).sin() * (rows as f32 / 6.0).min(2.5)).round();
                (mid + bend as i32)
                    .min(self.height as i32 - 3)
                    .max(ground_top + 2)
            })
            .collect();

        // plates of dry earth: every cell belongs to its nearest seed, with rows counted double
        // because a cell is about twice as tall as it is wide
        let seeds = super::count_for(self.width, rows.max(1) as usize, 80, 4, 300);
        let points: Vec<(f32, f32)> = (0..seeds)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.range(ground_top as f32, self.height as f32),
                )
            })
            .collect();
        let owner = |x: i32, y: i32| -> usize {
            let mut best = (f32::MAX, 0);
            for (i, &(px, py)) in points.iter().enumerate() {
                let dx = x as f32 - px;
                let dy = (y as f32 - py) * 2.0;
                let d = dx * dx + dy * dy;
                if d < best.0 {
                    best = (d, i);
                }
            }
            best.1
        };
        let w = self.width as i32;
        let h = self.height as i32;
        let grid: Vec<Vec<usize>> = (ground_top..h)
            .map(|y| (0..w).map(|x| owner(x, y)).collect())
            .collect();
        let at = |x: i32, y: i32| grid[(y - ground_top) as usize][x as usize];
        self.cracks.clear();
        for y in ground_top..h {
            for x in 0..w {
                if self.in_river(x, y) {
                    continue;
                }
                let me = at(x, y);
                let right = (x + 1 < w).then(|| at(x + 1, y)).filter(|&o| o != me);
                let down = (y + 1 < h).then(|| at(x, y + 1)).filter(|&o| o != me);
                let ch = match (right, down) {
                    (Some(_), Some(_)) => '/',
                    (Some(_), None) => '|',
                    (None, Some(_)) => '_',
                    (None, None) => continue,
                };
                let other = right.or(down).unwrap_or(me);
                let at = hash(me.min(other), me.max(other));
                self.cracks.push(Crack { x, y, ch, at });
            }
        }
        let mut pebbles = Vec::new();
        for x in 0..self.width {
            if rng.chance(0.18) {
                pebbles.push((x as i32, self.river[x] + rng.below(2) as i32));
            }
        }
        self.pebbles = pebbles;
        let motes = self
            .motion
            .scaled(super::count_for(self.width, self.height, 90, 3, 40));
        self.dust = (0..motes)
            .map(|_| Mote {
                x: rng.range(0.0, self.width as f32),
                y: rng.range((horizon - 3).max(0) as f32, (h - 1) as f32),
                speed: rng.range(0.8, 2.2),
                ch: *rng.pick(&['.', ',', '\'']),
            })
            .collect();
        self.bird = rng.range(0.0, self.width as f32);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let pace = self.motion.pace();
        let speed = self.motion.speed();
        self.progress = (self.progress + dt * pace / CRACK_SECONDS).min(1.0);
        self.drying += dt * pace;
        let width = self.width as f32;
        for mote in &mut self.dust {
            mote.x = wrap(mote.x + mote.speed * speed * dt, width);
        }
        self.bird = wrap(self.bird + 1.2 * speed * dt + 4.0, width + 8.0) - 4.0;
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let width = self.width as i32;
        let horizon = self.horizon();

        // a single bird gliding across, its wings held still
        if horizon >= 5 {
            frame.put_str(
                self.bird.round() as i32,
                (horizon / 2).max(1),
                "~v~",
                Some(theme.foreground),
            );
        }

        // the sun, heavy and still, with fixed rays. It is drawn after the bird, which passes
        // behind it
        let (sun, rays): (&[&str], &[&str]) = if self.height >= 20 && self.width >= 40 {
            (
                &[
                    "  .---.  ",
                    " /     \\ ",
                    "|       |",
                    " \\     / ",
                    "  `---'  ",
                ],
                &["\\    |    /", "", "-           -", "", "/    |    \\"],
            )
        } else {
            (&[" .-. ", "(   )", " `-' "], &[])
        };
        let sun_w = sun[0].len() as i32;
        let sun_x = width - sun_w - 4;
        let sun_y = if rays.is_empty() { 0 } else { 1 };
        if self.width >= 16 && horizon > sun.len() as i32 + sun_y {
            for (dy, row) in sun.iter().enumerate() {
                for (dx, ch) in row.chars().enumerate() {
                    let (x, y) = (sun_x + dx as i32, sun_y + dy as i32);
                    // the inside of the disc is cleared, so nothing shows through it
                    let inside = dx > 0
                        && (dx as i32) < sun_w - 1
                        && ch == ' '
                        && dy > 0
                        && dy + 1 < sun.len();
                    if ch != ' ' {
                        frame.put(x, y, ch, Some(theme.bright_yellow()));
                    } else if inside {
                        frame.put(x, y, ' ', None);
                    }
                }
            }
            let ray_x = sun_x + sun_w / 2 - 6;
            for (dy, row) in rays.iter().enumerate() {
                let y = sun_y - 1 + dy as i32 * 3 / 2;
                for (dx, ch) in row.chars().enumerate() {
                    if ch != ' ' {
                        frame.put(ray_x + dx as i32, y, ch, Some(warning_colour(theme)));
                    }
                }
            }
        }

        frame.fill_row(horizon, '_', Some(theme.yellow()));

        // the withered tree on the horizon
        if self.width >= 30 && horizon >= 6 {
            let tx = width / 6;
            let tree: [&str; 5] = ["\\ | /", " \\|/_", "  |/ ", "  |  ", "  |  "];
            frame.put_sprite(tx - 2, horizon - 5, &tree, Some(theme.muted));
        }

        // the cracks spread edge by edge
        let shown = self.progress;
        for crack in &self.cracks {
            if crack.at < shown {
                frame.put(crack.x, crack.y, crack.ch, Some(theme.yellow()));
            }
        }

        // the dry riverbed: a channel of sand between two banks, with pebbles and the last of
        // the water
        for (x, &top) in self.river.iter().enumerate() {
            let x = x as i32;
            for y in top..top + 2 {
                let ch = if (x + y) % 3 == 0 { '.' } else { ' ' };
                frame.put(x, y, ch, Some(theme.muted));
            }
            // each bank is a line drawn by its slope, so a bend reads as one channel
            let previous = if x == 0 {
                top
            } else {
                self.river[x as usize - 1]
            };
            for bank in [top - 1, top + 2] {
                let (ch, row) = match top.cmp(&previous) {
                    std::cmp::Ordering::Less => ('/', bank + 1),
                    std::cmp::Ordering::Greater => ('\\', bank),
                    std::cmp::Ordering::Equal => ('_', bank),
                };
                frame.put(x, row, ch, Some(theme.yellow()));
            }
        }
        for &(x, y) in &self.pebbles {
            frame.put(x, y, 'o', Some(theme.muted));
        }
        let puddle = self.puddle();
        if puddle > 0 {
            let start = width / 2 - self.puddle_start() / 2;
            let offset = (self.puddle_start() - puddle) / 2;
            for i in 0..puddle {
                let x = start + offset + i;
                if let Some(&top) = self.river.get(x as usize) {
                    frame.put(x, top + 1, '~', Some(theme.bright_blue()));
                }
            }
        }

        for mote in &self.dust {
            frame.put(
                mote.x.round() as i32,
                mote.y.round() as i32,
                mote.ch,
                Some(theme.grey()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("drought");
        check_determinism("drought");
    }

    #[test]
    fn the_cracks_spread_and_the_puddle_dries_then_it_holds() {
        let mut rng = Rng::new(8);
        let mut land = Drought::new(100, 30, Motion::FULL, &mut rng);
        let count = |frame: &Frame| {
            frame
                .to_text()
                .chars()
                .filter(|c| "/|_".contains(*c))
                .count()
        };
        let early = render(&mut land, 100, 30, 1);
        assert!(early.to_text().contains('~'));
        for _ in 0..(60 * 60 * 20) {
            land.step(0.05, &mut rng);
        }
        let late = render(&mut land, 100, 30, 1);
        assert!(count(&late) > count(&early));
        assert_eq!(land.progress, 1.0);
        assert_eq!(land.puddle(), 0);
        let ground: String = (land.horizon() as usize + 1..30)
            .map(|y| late.row_text(y))
            .collect();
        assert!(!ground.contains("~~"), "the puddle should be gone");
    }

    #[test]
    fn the_puddle_shrinks_one_cell_at_a_time_at_most_every_few_seconds() {
        let mut rng = Rng::new(2);
        for motion in Motion::ALL {
            let mut land = Drought::new(80, 24, motion, &mut rng);
            let mut last = land.puddle();
            let mut t = 0.0;
            let mut changed_at = 0.0;
            for _ in 0..(120 * 20) {
                land.step(0.05, &mut rng);
                t += 0.05;
                let now = land.puddle();
                if now != last {
                    assert_eq!(last - now, 1);
                    if changed_at > 0.0 {
                        assert!(t - changed_at >= PUDDLE_SHRINK - 0.06);
                    }
                    changed_at = t;
                    last = now;
                }
            }
        }
    }
}
