//! Corrosion: a wall of riveted steel plates where rust starts at a few rivets and creeps cell by
//! cell across the plates and their seams, darkening as it ages, until about half the wall has
//! gone. Drips of rust then run slowly down from the worst patches.

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

/// Seconds a patch of rust takes to darken by one stage. Four stages, so no cell changes in
/// place more than four times in its life and never more often than once in this time.
pub const STAGE_SECONDS: f32 = 8.0;
/// The share of the wall that rust covers before it stops spreading.
pub const MAX_COVER: f32 = 0.5;
/// Seconds the rust takes to reach most of its cover at the standard pace.
const SPREAD_SECONDS: f32 = 60.0;

pub const SUMMARY: &str =
    "Riveted steel plates where rust creeps out from the rivets cell by cell and slowly drips";

pub const CAPTIONS: &[&str] = &[
    "Something has rusted. Scrape back to the first error.",
    "A little corrosion. Nothing a fix cannot treat.",
    "Rust spreads from one spot. So do fixes.",
    "The plates are sound underneath. Start at the rivet.",
    "Slow rot in the build. Catch it early.",
];

/// What a cell of the wall is when no rust is drawn on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Plate,
    SeamAcross,
    SeamDown,
    Joint,
    Rivet,
}

#[derive(Clone, Debug)]
struct Drip {
    x: i32,
    y: f32,
    speed: f32,
    bottom: i32,
}

pub struct Corrosion {
    width: usize,
    height: usize,
    motion: Motion,
    parts: Vec<Part>,
    /// Seconds each cell has been rusty, negative where it is still clean.
    rust: Vec<f32>,
    rusty: Vec<usize>,
    drips: Vec<Drip>,
    /// Seconds until the next drip may start.
    next_drip: f32,
    /// Spread attempts carried over between frames, so a slow spread still advances.
    pending: f32,
    sign: Option<(i32, i32)>,
}

impl Corrosion {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut wall = Self {
            width: 1,
            height: 1,
            motion,
            parts: Vec::new(),
            rust: Vec::new(),
            rusty: Vec::new(),
            drips: Vec::new(),
            next_drip: 6.0,
            pending: 0.0,
            sign: None,
        };
        wall.resize(width, height, rng);
        wall
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height)
            .then(|| y as usize * self.width + x as usize)
    }

    /// How much of the wall is rusty, from 0 to 1.
    fn cover(&self) -> f32 {
        self.rusty.len() as f32 / self.rust.len().max(1) as f32
    }

    fn lay_out_plates(&mut self) {
        let (w, h) = (self.width, self.height);
        let plate_w = (w / 4).clamp(8, 24);
        let plate_h = (h / 3).clamp(4, 9);
        self.parts = vec![Part::Plate; w * h];
        for y in 0..h {
            for x in 0..w {
                let across = y % plate_h == plate_h - 1;
                // every other row of plates is offset by half a plate, like a riveted hull
                let shift = if (y / plate_h) % 2 == 1 {
                    plate_w / 2
                } else {
                    0
                };
                let down = (x + shift) % plate_w == plate_w - 1;
                let part = match (across, down) {
                    (true, true) => Part::Joint,
                    (true, false) => Part::SeamAcross,
                    (false, true) => Part::SeamDown,
                    (false, false) => {
                        // one line of rivets beside each seam, as a lapped plate has
                        let near_across = y % plate_h == 0;
                        let near_down = (x + shift) % plate_w == 0;
                        if (near_across && (x + shift) % 4 == 2)
                            || (near_down && y % plate_h % 2 == 1)
                        {
                            Part::Rivet
                        } else {
                            Part::Plate
                        }
                    }
                };
                self.parts[y * w + x] = part;
            }
        }
    }

    /// Starts a patch of rust at a random rivet.
    fn seed(&mut self, rng: &mut Rng) {
        let rivets: Vec<usize> = (0..self.parts.len())
            .filter(|&i| self.parts[i] == Part::Rivet && self.rust[i] < 0.0)
            .collect();
        if rivets.is_empty() {
            return;
        }
        let i = *rng.pick(&rivets);
        self.rust[i] = 0.0;
        self.rusty.push(i);
    }

    /// One try at spreading: a rusty cell passes rust to a clean neighbour.
    fn spread(&mut self, rng: &mut Rng) {
        if self.rusty.is_empty() || self.cover() >= MAX_COVER {
            return;
        }
        let from = *rng.pick(&self.rusty);
        let (x, y) = ((from % self.width) as i32, (from / self.width) as i32);
        // rust runs downhill and along the seams more readily than up
        let (dx, dy) = *rng.pick(&[(1, 0), (-1, 0), (0, 1), (0, 1), (0, -1), (1, 1), (-1, 1)]);
        if let Some(to) = self.index(x + dx, y + dy) {
            if self.rust[to] < 0.0 {
                self.rust[to] = 0.0;
                self.rusty.push(to);
            }
        }
    }

    fn stage(age: f32) -> usize {
        if age < 0.0 {
            0
        } else {
            1 + ((age / STAGE_SECONDS) as usize).min(3)
        }
    }
}

impl Scene for Corrosion {
    fn name(&self) -> &'static str {
        "corrosion"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        let (old_w, old_h) = (self.width, self.height);
        let old = std::mem::take(&mut self.rust);
        self.width = width.max(1);
        self.height = height.max(1);
        self.lay_out_plates();
        // the rust is stretched onto the new size, so a resize never cleans the wall
        self.rust = vec![-1.0; self.width * self.height];
        self.rusty.clear();
        if !old.is_empty() {
            for y in 0..self.height {
                for x in 0..self.width {
                    let from = (y * old_h / self.height) * old_w + x * old_w / self.width;
                    let age = old.get(from).copied().unwrap_or(-1.0);
                    if age >= 0.0 {
                        let i = y * self.width + x;
                        self.rust[i] = age;
                        self.rusty.push(i);
                    }
                }
            }
        }
        if self.rusty.is_empty() {
            let seeds = count_for(self.width, self.height, 600, 2, 6);
            for _ in 0..seeds {
                self.seed(rng);
            }
        }
        self.drips.clear();
        self.sign = (self.width >= 30 && self.height >= 10)
            .then(|| (self.width as i32 - 8, (self.height / 3) as i32 / 2 + 1));
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let speed = self.motion.speed();
        let pace = self.motion.pace();
        for &i in &self.rusty {
            self.rust[i] += dt * pace;
        }
        // tries a second are set so the rust reaches most of its cover in about a minute
        let tries = self.rust.len() as f32 * MAX_COVER * 3.0 / SPREAD_SECONDS;
        self.pending += tries * dt * speed;
        while self.pending >= 1.0 {
            self.pending -= 1.0;
            self.spread(rng);
        }
        if self.cover() < MAX_COVER && rng.chance(dt * speed / 15.0) {
            self.seed(rng);
        }

        // drips run down from the darkest rust once there is enough of it
        self.next_drip -= dt * speed;
        let limit = self
            .motion
            .scaled(count_for(self.width, self.height, 400, 1, 8));
        if self.next_drip <= 0.0 && self.drips.len() < limit {
            self.next_drip = if self.motion.calm {
                rng.range(5.0, 10.0)
            } else {
                rng.range(2.0, 5.0)
            };
            let dark: Vec<usize> = self
                .rusty
                .iter()
                .copied()
                .filter(|&i| Self::stage(self.rust[i]) >= 3)
                .collect();
            if !dark.is_empty() {
                let i = *rng.pick(&dark);
                self.drips.push(Drip {
                    x: (i % self.width) as i32,
                    y: (i / self.width) as f32 + 1.0,
                    speed: rng.range(1.0, 2.5),
                    bottom: self.height as i32 - 1,
                });
            }
        }
        for drip in &mut self.drips {
            drip.y += drip.speed * speed * dt;
        }
        self.drips.retain(|d| (d.y as i32) < d.bottom);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let rust_colours = [
            theme.yellow(),
            warning_colour(theme),
            theme.red(),
            error_colour(theme),
        ];
        let heavy = theme.faint_shade();
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let stage = Self::stage(self.rust[i]);
                let rusted = stage.checked_sub(1).map(|s| rust_colours[s]);
                let (ch, colour) = match self.parts[i] {
                    Part::SeamAcross => {
                        ('-', rusted.filter(|_| stage >= 3).unwrap_or(theme.grey()))
                    }
                    Part::SeamDown => ('|', rusted.filter(|_| stage >= 3).unwrap_or(theme.grey())),
                    Part::Joint => ('+', rusted.filter(|_| stage >= 3).unwrap_or(theme.grey())),
                    Part::Rivet => ('o', rusted.unwrap_or(theme.muted)),
                    Part::Plate => match stage {
                        0 => continue,
                        1 => ('.', rust_colours[0]),
                        2 => (':', rust_colours[1]),
                        3 => ('%', rust_colours[2]),
                        _ => (heavy, rust_colours[3]),
                    },
                };
                frame.put(x as i32, y as i32, ch, Some(colour));
            }
        }
        for drip in &self.drips {
            frame.put(drip.x, drip.y as i32, '\'', Some(theme.red()));
        }
        if let Some((x, y)) = self.sign {
            frame.put_str(x - 1, y - 1, "     ", None);
            frame.put_str(x - 1, y, " /!\\ ", Some(warning_colour(theme)));
            frame.put_str(x - 1, y + 1, "     ", None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("corrosion");
        check_determinism("corrosion");
    }

    #[test]
    fn rust_creeps_slowly_then_stops_at_half_the_wall() {
        let mut rng = Rng::new(12);
        let mut wall = Corrosion::new(80, 24, Motion::LIVELY, &mut rng);
        for _ in 0..(5 * 20) {
            wall.step(0.05, &mut rng);
        }
        assert!(wall.cover() < 0.1, "{}", wall.cover());
        for _ in 0..(120 * 20) {
            wall.step(0.05, &mut rng);
        }
        assert!(wall.cover() > MAX_COVER * 0.8, "{}", wall.cover());
        for _ in 0..(3600 * 20) {
            wall.step(0.05, &mut rng);
        }
        assert!(wall.cover() <= MAX_COVER + 0.01, "{}", wall.cover());
        let frame = render(&mut wall, 80, 24, 1);
        let text = frame.to_text();
        assert!(text.contains('o') && text.contains('|') && text.contains('-'));
        assert!(
            text.contains('\u{2592}') || text.contains('\u{2591}'),
            "{text}"
        );
    }

    #[test]
    fn a_cell_darkens_no_faster_than_one_stage_a_period() {
        // the stage only depends on age, which grows at most at the standard pace
        for motion in Motion::ALL {
            assert!(motion.pace() <= 1.0);
        }
        assert_eq!(Corrosion::stage(-1.0), 0);
        assert_eq!(Corrosion::stage(0.0), 1);
        assert_eq!(Corrosion::stage(STAGE_SECONDS - 0.01), 1);
        assert_eq!(Corrosion::stage(STAGE_SECONDS), 2);
        assert_eq!(Corrosion::stage(STAGE_SECONDS * 10.0), 4);
        const { assert!(STAGE_SECONDS >= 1.0 / 3.0) };
    }

    #[test]
    fn a_resize_keeps_the_rust() {
        let mut rng = Rng::new(5);
        let mut wall = Corrosion::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            wall.step(0.05, &mut rng);
        }
        let before = wall.cover();
        wall.resize(120, 40, &mut rng);
        assert!(
            (wall.cover() - before).abs() < 0.1,
            "{before} {}",
            wall.cover()
        );
    }
}
