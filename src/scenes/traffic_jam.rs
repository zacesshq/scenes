//! Traffic jam: cars roll in from the left and stop bumper to bumper behind a stop line, a red
//! signal that never changes and a row of cones. Once the queue reaches the edge of the screen
//! no more arrive and it stays stuck, with exhaust puffs drifting up and clouds passing over.

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str =
    "Cars queued bumper to bumper before a red signal and a row of cones, exhaust drifting up";

pub const CAPTIONS: &[&str] = &[
    "Everything is queued behind one problem.",
    "Stuck at the first red light. Clear that one.",
    "Nothing moves until the first error does.",
    "A jam, not a crash. Fix the front of the queue.",
    "The road is closed for now. Read the sign below.",
];

/// The two rows of a car facing right.
const CAR: [&str; 2] = [" ,-=-. ", "(o___o)"];
const CAR_WIDTH: i32 = 7;
/// Cells between two cars in the queue.
const GAP: i32 = 2;
/// Seconds between two cars joining the back of a lane.
const ARRIVAL_PERIOD: f32 = 2.5;
/// Seconds between two puffs from one car at the standard pace. A puff changes its own cell
/// once, from `o` to `.`, as it fades.
pub const PUFF_PERIOD: f32 = 3.0;
/// How long a puff lasts.
const PUFF_LIFE: f32 = 2.4;

#[derive(Clone, Debug)]
struct Car {
    x: f32,
    colour: usize,
    /// When this car's next puff is due, so the puffs are staggered.
    puff_in: f32,
}

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
}

pub struct TrafficJam {
    width: usize,
    height: usize,
    motion: Motion,
    lanes: Vec<Vec<Car>>,
    arrive_in: Vec<f32>,
    puffs: Vec<Puff>,
    clouds: Vec<(f32, i32, usize)>,
    hills: Vec<i32>,
    trees: Vec<usize>,
}

impl TrafficJam {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut jam = Self {
            width: 1,
            height: 1,
            motion,
            lanes: Vec::new(),
            arrive_in: Vec::new(),
            puffs: Vec::new(),
            clouds: Vec::new(),
            hills: Vec::new(),
            trees: Vec::new(),
        };
        jam.resize(width, height, rng);
        jam
    }

    fn lane_count(&self) -> usize {
        if self.height >= 16 { 2 } else { 1 }
    }

    /// The kerb row along the top of the road.
    fn road_top(&self) -> i32 {
        let lanes = self.lane_count() as i32;
        (self.height as i32 - 2 - lanes * 3 + 1).max(0)
    }

    /// The top row of a lane's cars.
    fn lane_row(&self, lane: usize) -> i32 {
        self.road_top() + 1 + lane as i32 * 3
    }

    /// Where the front of the first car stops.
    fn stop_x(&self) -> i32 {
        (self.width as i32 - 10).max(CAR_WIDTH)
    }

    /// Where the car at a place in the queue comes to rest.
    fn rest_x(&self, place: usize) -> f32 {
        (self.stop_x() - CAR_WIDTH - place as i32 * (CAR_WIDTH + GAP)) as f32
    }

    fn colour(theme: &ThemeData, index: usize) -> crate::colour::Rgb {
        match index % 7 {
            0 => theme.blue(),
            1 => theme.cyan(),
            2 => theme.magenta(),
            3 => theme.green(),
            4 => theme.bright_blue(),
            5 => theme.foreground,
            _ => theme.bright_magenta(),
        }
    }
}

impl Scene for TrafficJam {
    fn name(&self) -> &'static str {
        "traffic-jam"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let lanes = self.lane_count();
        // the cars already queued are kept in place, as many as still fit
        let mut kept: Vec<Vec<Car>> = std::mem::take(&mut self.lanes);
        kept.resize(lanes, Vec::new());
        kept.truncate(lanes);
        for lane in &mut kept {
            let places = lane.len();
            lane.truncate(places);
            for (place, car) in lane.iter_mut().enumerate() {
                car.x = car.x.min(self.rest_x(place));
            }
            while lane.last().is_some_and(|c| c.x < -(CAR_WIDTH as f32)) {
                lane.pop();
            }
        }
        self.lanes = kept;
        self.arrive_in = (0..lanes).map(|i| i as f32 * 1.2).collect();
        self.puffs.clear();
        let road = self.road_top();
        let amplitude = ((road - 3) as f32 / 3.0).clamp(0.0, 5.0);
        let phase = rng.range(0.0, 6.0);
        self.hills = (0..self.width)
            .map(|x| {
                let x = x as f32;
                let wave = ((x / 13.0 + phase).sin() + 0.5 * (x / 5.5).sin()) / 1.5;
                road - 2 - (amplitude * (wave + 1.0) / 2.0).round() as i32
            })
            .collect();
        self.trees = (0..self.width)
            .filter(|x| x % 5 == 2 && rng.chance(0.35))
            .collect();
        let clouds = self.motion.scaled((self.width / 35).max(1));
        self.clouds = (0..clouds)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.below((road as usize / 3).max(1)) as i32,
                    5 + rng.below(8),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let speed = self.motion.speed();
        let pace = self.motion.pace();
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.4 * speed * dt + 16.0, width + 32.0) - 16.0;
        }
        for lane in 0..self.lanes.len() {
            self.arrive_in[lane] -= dt;
            let place = self.lanes[lane].len();
            let room = self.rest_x(place) > -(CAR_WIDTH as f32) + 1.0;
            let entry_clear = self.lanes[lane]
                .last()
                .is_none_or(|c| c.x > (GAP + 1) as f32);
            if self.arrive_in[lane] <= 0.0 && room && entry_clear {
                self.arrive_in[lane] = ARRIVAL_PERIOD / speed;
                self.lanes[lane].push(Car {
                    x: -(CAR_WIDTH as f32),
                    colour: rng.below(7),
                    puff_in: rng.range(0.5, PUFF_PERIOD),
                });
            }
            let lane_row = self.lane_row(lane);
            let mut ahead = f32::MAX;
            for place in 0..self.lanes[lane].len() {
                // a car rolls in and eases to a stop, never closer than the gap to the one ahead
                let rest = self.rest_x(place);
                let target = rest.min(ahead - (CAR_WIDTH + GAP) as f32);
                let car = &mut self.lanes[lane][place];
                let left = (target - car.x).max(0.0);
                let pace_now = (left * 1.2).clamp(1.5, 8.0) * speed;
                car.x += (pace_now * dt).min(left);
                ahead = car.x;
                if left < 0.5 && target == rest {
                    car.x = target;
                    car.puff_in -= dt * pace;
                    if car.puff_in <= 0.0 {
                        car.puff_in = PUFF_PERIOD;
                        self.puffs.push(Puff {
                            x: car.x - 1.0,
                            y: (lane_row + 1) as f32,
                            age: 0.0,
                        });
                    }
                }
            }
        }
        for puff in &mut self.puffs {
            puff.age += dt * pace;
            puff.x -= 0.8 * speed * dt;
            puff.y -= 0.6 * speed * dt;
        }
        self.puffs.retain(|p| p.age < PUFF_LIFE);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let road = self.road_top();
        let bottom = self.height as i32 - 1;
        for &(x, y, len) in &self.clouds {
            for i in 0..len as i32 {
                frame.put(x.round() as i32 + i, y, '~', Some(theme.grey()));
            }
        }
        if road >= 4 {
            // a ridge of low hills behind the road, drawn by its slope
            for (x, &y) in self.hills.iter().enumerate() {
                let previous = if x == 0 { y } else { self.hills[x - 1] };
                let (ch, row) = match y.cmp(&previous) {
                    std::cmp::Ordering::Less => ('/', y + 1),
                    std::cmp::Ordering::Greater => ('\\', y),
                    std::cmp::Ordering::Equal => ('_', y),
                };
                frame.put(x as i32, row.max(1), ch, Some(theme.green()));
            }
            for &x in &self.trees {
                let y = self.hills[x];
                let flat = x > 0 && self.hills[x - 1] == y;
                if y >= 1 && flat {
                    frame.put(x as i32, y, '^', Some(theme.green()));
                }
            }
        }
        frame.fill_row(road, '_', Some(theme.muted));
        frame.fill_row(bottom, '_', Some(theme.muted));
        if self.lane_count() == 2 {
            let divider = self.lane_row(0) + 2;
            for x in (0..self.width as i32).filter(|x| x % 6 < 3) {
                frame.put(x, divider, '-', Some(theme.yellow()));
            }
        }

        let stop = self.stop_x();
        for lane in 0..self.lane_count() {
            let row = self.lane_row(lane);
            // the stop line and a cone in each lane past it
            frame.put(stop, row, '|', Some(theme.foreground));
            frame.put(stop, row + 1, '|', Some(theme.foreground));
            frame.put(stop + 2, row + 1, 'A', Some(warning_colour(theme)));
            frame.put(stop + 4, row + 1, 'A', Some(warning_colour(theme)));
            for car in self.lanes.get(lane).into_iter().flatten() {
                let x = car.x.round() as i32;
                frame.put_sprite(x, row, &CAR, Some(Self::colour(theme, car.colour)));
                // the brake light, steady red, at the back of a car that has stopped
                frame.put(x, row + 1, '(', Some(error_colour(theme)));
            }
        }
        for puff in &self.puffs {
            let ch = if puff.age < PUFF_LIFE / 2.0 { 'o' } else { '.' };
            let y = puff.y.round() as i32;
            if y > road || road < 3 {
                frame.put(puff.x.round() as i32, y, ch, Some(theme.grey()));
            }
        }

        // the signal, stuck on red: the lit lamp is a filled circle in the error colour, the
        // others are dots, so it reads without colour
        let sx = stop + 6;
        let signal: &[(&str, Option<crate::colour::Rgb>)] = &[
            (".-.", Some(theme.foreground)),
            ("(O)", Some(error_colour(theme))),
            ("(.)", Some(theme.muted)),
            ("(.)", Some(theme.muted)),
            ("'-'", Some(theme.foreground)),
        ];
        let box_top = road - signal.len() as i32 - 1;
        if box_top >= 0 && self.width >= 24 {
            for (i, (row, colour)) in signal.iter().enumerate() {
                frame.put_str(sx - 1, box_top + i as i32, row, *colour);
            }
            frame.put(sx, road - 1, '|', Some(theme.foreground));
            if self.width >= 40 && box_top >= 1 {
                let text = "ROAD CLOSED";
                let x = sx - 3 - text.len() as i32;
                frame.put_str(x, box_top + 1, text, Some(error_colour(theme)));
                frame.put_str(x, box_top + 2, "-----------", Some(warning_colour(theme)));
            }
        } else if road >= 1 {
            frame.put_str(sx - 1, road - 1, "(O)", Some(error_colour(theme)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("traffic-jam");
        check_determinism("traffic-jam");
    }

    #[test]
    fn the_queue_fills_to_the_edge_and_never_moves_past_the_stop_line() {
        let mut rng = Rng::new(3);
        let mut jam = TrafficJam::new(100, 30, Motion::LIVELY, &mut rng);
        for _ in 0..(60 * 60 * 20) {
            jam.step(0.05, &mut rng);
        }
        for lane in &jam.lanes {
            assert!(lane.len() >= 8, "{}", lane.len());
            for (place, car) in lane.iter().enumerate() {
                assert_eq!(car.x, jam.rest_x(place));
                assert!(car.x as i32 + CAR_WIDTH <= jam.stop_x());
            }
            // the back of the queue is at the edge of the screen
            assert!(lane.last().unwrap().x < (CAR_WIDTH + GAP) as f32);
        }
        let frame = render(&mut jam, 100, 30, 1);
        let text = frame.to_text();
        assert!(text.contains("(O)") && text.contains("ROAD CLOSED"));
        assert!(text.contains("(o___o)"));
    }

    #[test]
    fn puffs_are_slow_on_every_level() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let jam = TrafficJam::new(80, 24, motion, &mut rng);
            assert!(PUFF_PERIOD / jam.motion.pace() >= PUFF_PERIOD);
        }
        const { assert!(PUFF_LIFE / 2.0 > 1.0 / 3.0) };
    }
}
