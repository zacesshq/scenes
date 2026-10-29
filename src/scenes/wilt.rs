//! Wilt: potted plants on a windowsill under a hard sun. Over the first minute their stems bend
//! and droop until the flowers hang over the rims, their leaves dry and fall one by one onto the
//! sill and the soil in each pot cracks.

use std::f32::consts::TAU;

use super::cycle;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Seconds the plants take to wilt at the standard pace.
pub const WILT_SECONDS: f32 = 50.0;
/// Seconds of one slow sway of the drooping stems once they have wilted.
pub const SWAY_PERIOD: f32 = 9.0;
/// How far the furthest bend reaches from upright, in radians.
const MAX_BEND: f32 = 2.8;
/// A cell is about twice as tall as it is wide.
const ASPECT: f32 = 2.0;
const FALL_SPEED: f32 = 1.5;

pub const SUMMARY: &str =
    "Potted plants on a sunny sill whose stems droop, leaves fall and soil cracks";

pub const CAPTIONS: &[&str] = &[
    "The build needs watering. Start with the first error.",
    "Drooping, not dead. One fix perks it up.",
    "Dry soil. A little care brings it back.",
    "Every plant wilts sometimes. So does every build.",
    "Prune the first error and see what grows.",
];

const POT_WIDTH: i32 = 7;

#[derive(Clone, Debug)]
struct Leaf {
    /// How far along the stem it grows, from 0 to 1.
    at: f32,
    right: bool,
    /// How wilted the plant is when this leaf lets go.
    falls_at: f32,
    /// Where it is once it has let go. `landed` says whether it has come to rest.
    falling: Option<(f32, f32)>,
    landed: bool,
}

#[derive(Clone, Debug)]
struct Plant {
    x: i32,
    length: usize,
    /// 1 to droop to the right, -1 to the left.
    side: f32,
    /// How far this plant bends compared with the others.
    droop: f32,
    leaves: Vec<Leaf>,
    cracks: Vec<f32>,
    flower: usize,
}

pub struct Wilt {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of wilting so far, kept across a resize so the plants never revive.
    wilted: f32,
    plants: Vec<Plant>,
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Wilt {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut wilt = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            wilted: 0.0,
            plants: Vec::new(),
        };
        wilt.resize(width, height, rng);
        wilt
    }

    fn amount(&self) -> f32 {
        ease(self.wilted / WILT_SECONDS)
    }

    /// The sill the pots stand on.
    fn sill(&self) -> i32 {
        self.height as i32 - 1
    }

    /// The row of a pot's rim, which holds its soil.
    fn rim(&self) -> i32 {
        self.sill() - 2
    }

    /// Points along a stem from the soil to its tip, in quarter steps so a bent stem never
    /// skips a cell.
    fn stem(&self, plant: &Plant) -> Vec<(f32, f32, f32)> {
        let amount = self.amount();
        let sway = 0.08 * amount * cycle(self.t, TAU / SWAY_PERIOD * self.motion.pace()).sin();
        let steps = plant.length * 4;
        let mut x = plant.x as f32;
        let mut y = (self.rim() - 1) as f32;
        let mut points = vec![(x, y, 0.0)];
        for k in 1..=steps {
            let s = k as f32 / steps as f32;
            let bend = plant.side * (MAX_BEND * plant.droop * amount * s * s + sway * s);
            x += bend.sin() * ASPECT * 0.25;
            y -= bend.cos() * 0.25;
            points.push((x, y, bend));
        }
        points
    }
}

/// The cells a stem passes through, each once, with the character for the stem's direction
/// where it enters them, so a curve reads as one line rather than a staircase.
fn stem_cells(points: &[(f32, f32, f32)]) -> Vec<(i32, i32, char)> {
    let mut cells: Vec<(i32, i32, char)> = Vec::new();
    for &(x, y, bend) in points {
        let cell = (x.round() as i32, y.round() as i32);
        let ch = stem_char(bend);
        // a step along a row is only drawn for a level stretch; elsewhere the stem moves on to
        // the next row diagonally, so a slanting stem is never two characters thick
        let skip = cells
            .last()
            .is_some_and(|&(lx, ly, _)| (lx, ly) == cell || (ly == cell.1 && ch != '_'));
        if !skip {
            cells.push((cell.0, cell.1, ch));
        }
    }
    cells
}

/// The character for a stem heading in a direction, measured from straight up in square units.
/// The angle is turned into cells, which are twice as tall as they are wide, before it is
/// matched to a character.
fn stem_char(bend: f32) -> char {
    let a = (bend.sin().abs() * ASPECT).atan2(bend.cos());
    let right = bend > 0.0;
    if a < 0.46 || a > std::f32::consts::PI - 0.46 {
        '|'
    } else if a < 0.98 {
        if right { '/' } else { '\\' }
    } else if a < std::f32::consts::PI - 0.98 {
        '_'
    } else if right {
        '\\'
    } else {
        '/'
    }
}

impl Scene for Wilt {
    fn name(&self) -> &'static str {
        "wilt"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let gap = if self.width < 40 { 9 } else { 16 };
        let count = ((self.width as i32 - 2) / gap).max(1) as usize;
        let room = (self.rim() - 1).max(1) as usize;
        let tallest = ((room as f32 * 0.8) as usize).clamp(2, 16);
        let spare = self.width as i32 - gap * count as i32;
        self.plants = (0..count)
            .map(|i| {
                let length = tallest - rng.below((tallest / 2).max(1));
                let first = rng.below(2);
                let leaves = (1..length)
                    .skip(first)
                    .step_by(2)
                    .map(|k| Leaf {
                        at: k as f32 / length as f32,
                        right: (k + first) % 4 < 2,
                        falls_at: rng.range(0.35, 0.95),
                        falling: None,
                        landed: false,
                    })
                    .collect();
                Plant {
                    x: spare / 2 + gap * i as i32 + gap / 2,
                    length,
                    side: if rng.chance(0.5) { 1.0 } else { -1.0 },
                    droop: rng.range(0.8, 1.05),
                    leaves,
                    cracks: (0..POT_WIDTH - 2).map(|_| rng.range(0.25, 1.0)).collect(),
                    flower: i,
                }
            })
            .collect();
        // leaves that have already fallen land again on the new sill at once
        let amount = self.amount();
        let floor = (self.sill() - 1) as f32;
        for plant in &mut self.plants {
            for leaf in &mut plant.leaves {
                if amount >= leaf.falls_at {
                    leaf.landed = true;
                    leaf.falling = Some((plant.x as f32 + rng.range(-5.0, 5.0), floor));
                }
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.wilted = (self.wilted + dt * speed).min(WILT_SECONDS * 2.0);
        let amount = self.amount();
        let floor = (self.sill() - 1) as f32;
        let stems: Vec<Vec<(f32, f32, f32)>> = self.plants.iter().map(|p| self.stem(p)).collect();
        for (plant, stem) in self.plants.iter_mut().zip(stems) {
            for leaf in &mut plant.leaves {
                if leaf.landed {
                    continue;
                }
                match &mut leaf.falling {
                    None if amount >= leaf.falls_at => {
                        let i = ((stem.len() - 1) as f32 * leaf.at) as usize;
                        let (x, y, _) = stem[i];
                        leaf.falling = Some((x + if leaf.right { 1.0 } else { -1.0 }, y));
                    }
                    Some((x, y)) => {
                        *y += FALL_SPEED * speed * dt;
                        *x += rng.range(-0.6, 0.6) * speed * dt;
                        if *y >= floor {
                            *y = floor;
                            leaf.landed = true;
                        }
                    }
                    None => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let amount = self.amount();
        let sill = self.sill();
        let rim = self.rim();

        // a hard sun in the corner when there is room for it
        if self.width >= 30 && self.height >= 14 {
            let sun = Some(theme.bright_yellow());
            frame.put_str(2, 0, "\\ | /", sun);
            frame.put_str(1, 1, "-- O --", sun);
            frame.put_str(2, 2, "/ | \\", sun);
        }
        frame.fill_row(sill, '=', Some(theme.muted));

        let flowers = [theme.magenta(), theme.yellow(), theme.red(), theme.cyan()];
        for plant in &self.plants {
            let stem = self.stem(plant);
            let green = if amount < 0.5 {
                theme.green()
            } else {
                theme.yellow()
            };
            for &(x, y, ch) in &stem_cells(&stem) {
                if y < rim {
                    frame.put(x, y, ch, Some(green));
                }
            }
            for leaf in &plant.leaves {
                if leaf.falling.is_some() || leaf.landed {
                    continue;
                }
                let i = ((stem.len() - 1) as f32 * leaf.at) as usize;
                let (x, y, _) = stem[i];
                let dry = amount > leaf.falls_at * 0.7;
                let (ch, colour) = match (dry, leaf.right) {
                    (false, true) => ('>', theme.green()),
                    (false, false) => ('<', theme.green()),
                    (true, _) => (',', theme.yellow()),
                };
                let dx = if leaf.right { 1 } else { -1 };
                if (y.round() as i32) < rim {
                    frame.put(x.round() as i32 + dx, y.round() as i32, ch, Some(colour));
                }
            }
            // the flower at the tip fades from a bloom to a dry head as it droops
            let (tx, ty, _) = *stem.last().expect("a stem has points");
            let (head, colour) = if amount < 0.4 {
                ('@', flowers[plant.flower % flowers.len()])
            } else if amount < 0.8 {
                ('o', flowers[plant.flower % flowers.len()])
            } else {
                ('.', theme.muted)
            };
            if (ty.round() as i32) < rim {
                frame.put(tx.round() as i32, ty.round() as i32, head, Some(colour));
            }

            for leaf in &plant.leaves {
                if let Some((x, y)) = leaf.falling {
                    let ch = if leaf.landed { '.' } else { ',' };
                    frame.put(x.round() as i32, y.round() as i32, ch, Some(theme.yellow()));
                }
            }

            // the pot, its soil cracking as it dries, drawn over any leaf that fell against it
            let left = plant.x - POT_WIDTH / 2;
            frame.put(left, rim, '[', Some(theme.red()));
            frame.put(left + POT_WIDTH - 1, rim, ']', Some(theme.red()));
            for (i, crack) in plant.cracks.iter().enumerate() {
                let ch = if amount < *crack {
                    '.'
                } else {
                    ['v', '/', '\\', 'v', '/'][i % 5]
                };
                frame.put(left + 1 + i as i32, rim, ch, Some(theme.yellow()));
            }
            frame.put_str(left + 1, rim + 1, "\\___/", Some(theme.red()));
            frame.put(plant.x, rim, '|', Some(green));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("wilt");
        check_determinism("wilt");
    }

    #[test]
    fn stems_droop_leaves_fall_and_land_on_the_sill() {
        let mut rng = Rng::new(10);
        let mut wilt = Wilt::new(80, 24, Motion::FULL, &mut rng);
        let tips: Vec<f32> = wilt
            .plants
            .iter()
            .map(|p| wilt.stem(p).last().unwrap().1)
            .collect();
        let fresh = render(&mut wilt, 80, 24, 1);
        assert!(fresh.to_text().contains('@'), "{}", fresh.to_text());
        for _ in 0..(3600 * 20) {
            wilt.step(0.05, &mut rng);
        }
        for (plant, tip) in wilt.plants.iter().zip(tips) {
            let now = wilt.stem(plant).last().unwrap().1;
            assert!(now > tip + 2.0, "the tip only moved from {tip} to {now}");
            for leaf in &plant.leaves {
                assert!(leaf.landed);
                let (x, y) = leaf
                    .falling
                    .unwrap_or((plant.x as f32, wilt.sill() as f32 - 1.0));
                assert_eq!(y, (wilt.sill() - 1) as f32);
                assert!(x > -2.0 && x < 82.0);
            }
        }
        let frame = render(&mut wilt, 80, 24, 1);
        assert!(frame.to_text().contains('v') && frame.to_text().contains("\\___/"));
    }

    #[test]
    fn stem_characters_follow_the_bend() {
        assert_eq!(stem_char(0.0), '|');
        assert_eq!(stem_char(0.4), '/');
        assert_eq!(stem_char(-0.4), '\\');
        assert_eq!(stem_char(1.5), '_');
        assert_eq!(stem_char(2.6), '\\');
        assert_eq!(stem_char(3.1), '|');
        let cells = stem_cells(&[(0.0, 5.0, 0.0), (0.1, 4.9, 0.0), (1.0, 4.0, 1.5)]);
        assert_eq!(cells, vec![(0, 5, '|'), (1, 4, '_')]);
        const { assert!(SWAY_PERIOD / 4.0 > 1.0 / 3.0) };
    }
}
