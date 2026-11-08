//! Traffic jam: a two lane road seen side on, with a queue of cars, hatchbacks and vans stopped
//! bumper to bumper behind a signal stuck on red. Brake lights glow on the road behind each car,
//! cones and a "ROAD CLOSED" board stand past the signal, exhaust drifts up from the tailpipes and
//! a town skyline sits under a dusk sky (a bright day on a light theme) behind a verge of trees.
//! The last few cars roll up to the back of the queue in the first seconds; after that nothing
//! moves but the exhaust and the clouds.
//!
//! The sky, the skyline, the trees, the road and the exhaust are tints of the background held
//! below the non-text floor. The cars, the cones, the signal and the board are solid, so the
//! queue keeps its outline when colour is off. The lit lamp is the only filled one.

use super::paint::{self, Blender, Cloud, gradient, shaded, solid, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str = "Cars and vans queued bumper to bumper with brake lights on before a red signal, exhaust drifting up, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything is queued behind one problem.",
    "Stuck at the first red light. Clear that one.",
    "Nothing moves until the first error does.",
    "A jam, not a crash. Fix the front of the queue.",
    "The road is closed for now. Read the sign below.",
];

/// The large cars, facing right: `l` the lit top of the body, `b` the body, `d` its shaded
/// lower half, `w` glass, `k` tyres, `h` hubs, `r` the brake light and `e` the headlight.
const SEDAN: [&str; 10] = [
    ".......llllllll.......",
    "......lwwwwlwwwwl.....",
    ".....lwwwwwlwwwwwl....",
    "..lllllllllllllllllll.",
    ".bbbbbbbbbbbbbbbbbbbbb",
    "rbbbbbbbbbbbbbbbbbbbbe",
    "dddddddddddddddddddddd",
    ".dddkkkdddddddddkkkdd.",
    "...kkhkk.......kkhkk..",
    "....kkk.........kkk...",
];
const HATCH: [&str; 10] = [
    "...llllllllll.......",
    "..lwwwwlwwwwwl......",
    "..lwwwwlwwwwwwl.....",
    ".llllllllllllllll...",
    "rbbbbbbbbbbbbbbbbbb.",
    "rbbbbbbbbbbbbbbbbbbe",
    "dddddddddddddddddddd",
    ".ddkkkdddddddddkkkd.",
    "..kkhkk.......kkhkk.",
    "...kkk.........kkk..",
];
const VAN: [&str; 10] = [
    "llllllllllllllllll......",
    "lbbbbbbbbbbbbbbbbwwl....",
    "bbbbbbbbbbbbbbbbbwwwl...",
    "bbbbbbbbbbbbbbbbbwwwwl..",
    "bbbbbbbbbbbbbbbbbbbbbbbb",
    "rbbbbbbbbbbbbbbbbbbbbbbe",
    "dddddddddddddddddddddddd",
    ".ddkkkdddddddddddddkkkd.",
    "..kkhkk...........kkhkk.",
    "...kkk.............kkk..",
];
/// The small cars for a narrow or short screen.
const SMALL_SEDAN: [&str; 6] = [
    "...llllll...",
    "..lwwlwwwl..",
    "lbbbbbbbbbbb",
    "rbbbbbbbbbbe",
    "dkkddddddkkd",
    ".kk.....kk..",
];
const SMALL_VAN: [&str; 6] = [
    "lllllllll....",
    "lbbbbbbbwwl..",
    "bbbbbbbbbwwl.",
    "rbbbbbbbbbbbe",
    "ddkkdddddkkdd",
    "..kk.....kk..",
];
/// A large cone and a small one: orange with a white reflective band.
const CONE: [&str; 6] = [
    "...o...", "..ooo..", "..www..", ".ooooo.", ".wwwww.", "ooooooo",
];
const SMALL_CONE: [&str; 4] = ["..o..", ".www.", ".ooo.", "ooooo"];

/// Seconds between two cars joining the back of a lane while there is still room.
const ARRIVAL_PERIOD: f32 = 2.5;
/// Seconds between two puffs from one car at the standard pace.
pub const PUFF_PERIOD: f32 = 3.0;
/// How long a puff lasts.
const PUFF_LIFE: f32 = 2.8;
/// The cars already queued when the scene starts are this many short of a full lane, so the
/// first seconds show the last arrivals rolling up.
const STILL_TO_COME: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Sedan,
    Hatch,
    Van,
}

#[derive(Clone, Debug)]
struct Car {
    x: f32,
    colour: usize,
    kind: Kind,
    /// Pixels left between this car and the one ahead once it stops.
    gap: i32,
    /// When this car's next puff is due, so the puffs are staggered.
    puff_in: f32,
}

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
}

/// The road's colours in one theme.
struct Colours {
    sky: Ramp,
    cloud: (Rgb, Rgb),
    skyline: (Rgb, Rgb),
    window: Rgb,
    lit_windows: bool,
    verge: Rgb,
    trees: (Rgb, Rgb),
    pavement: Rgb,
    kerb: Rgb,
    road: Ramp,
    marking: Rgb,
    cars: [Rgb; 5],
    glass: Rgb,
    tyre: Rgb,
    hub: Rgb,
    brake: Rgb,
    glow: Rgb,
    head: Rgb,
    cone: (Rgb, Rgb),
    housing: (Rgb, Rgb, Rgb),
    pole: Rgb,
    unlit: Rgb,
    halo: Rgb,
    puff: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dark = theme.dark;
        let grey = mix(theme.muted, theme.info, 0.3);
        // a dusk on a dark theme, glowing low over the town; a clear day on a light one
        let (sky_top, sky_low) = if dark {
            (
                shades.tint(theme.blue(), 0.16, 1.28),
                shades.tint(mix(theme.warning, theme.magenta(), 0.3), 0.45, 2.15),
            )
        } else {
            (
                shades.tint(theme.info, 0.34, 1.7),
                shades.tint(theme.info, 0.07, 1.1),
            )
        };
        let cloud = if dark {
            (
                shades.tint(mix(theme.magenta(), theme.foreground, 0.4), 0.3, 1.75),
                shades.tint(theme.muted, 0.2, 1.4),
            )
        } else {
            (
                shades.tint(theme.info, 0.01, 1.03),
                shades.tint(theme.muted, 0.18, 1.35),
            )
        };
        // the skyline is a silhouette against the glow on a dark theme and a pale blue grey on
        // a light one, the far blocks fainter than the near
        let skyline = if dark {
            (
                shades.tint(theme.blue(), 0.2, 1.38),
                shades.tint(grey, 0.12, 1.2),
            )
        } else {
            (shades.tint(grey, 0.24, 1.42), shades.tint(grey, 0.38, 1.75))
        };
        let window = if dark {
            shades.tint(theme.bright_yellow(), 0.5, 2.1)
        } else {
            shades.tint(theme.info, 0.08, 1.12)
        };
        let (verge, trees) = if dark {
            (
                shades.tint(theme.green(), 0.22, 1.45),
                (
                    shades.tint(theme.green(), 0.32, 1.7),
                    shades.tint(theme.green(), 0.16, 1.3),
                ),
            )
        } else {
            (
                shades.tint(theme.green(), 0.4, 1.85),
                (
                    shades.tint(theme.green(), 0.55, 2.2),
                    shades.tint(theme.green(), 0.78, 2.75),
                ),
            )
        };
        let (pavement, kerb, road_top, road_low, marking) = if dark {
            (
                shades.tint(theme.muted, 0.42, 2.1),
                shades.tint(theme.foreground, 0.5, 2.6),
                shades.tint(theme.muted, 0.22, 1.5),
                shades.tint(theme.muted, 0.14, 1.3),
                shades.tint(theme.foreground, 0.85, 2.85),
            )
        } else {
            (
                shades.tint(theme.muted, 0.22, 1.4),
                shades.tint(theme.muted, 0.08, 1.12),
                shades.tint(theme.muted, 0.6, 2.35),
                shades.tint(theme.muted, 0.72, 2.7),
                shades.tint(theme.info, 0.02, 1.03),
            )
        };
        let glass = if dark {
            shades.tint(mix(theme.info, sky_low, 0.3), 0.42, 2.0)
        } else {
            shades.tint(theme.info, 0.16, 1.3)
        };
        let tyre = if dark {
            shades.tint(theme.muted, 0.05, 1.08)
        } else {
            solid(theme, theme.muted, shades.darkest(), 0.8)
        };
        let housing = solid(theme, theme.muted, shades.darkest(), 0.6);
        let brake = error_colour(theme);
        Self {
            sky: Ramp::new(sky_top, sky_low, 9),
            cloud,
            skyline,
            window,
            lit_windows: dark,
            verge,
            trees,
            pavement,
            kerb,
            road: Ramp::new(road_top, road_low, 5),
            marking,
            cars: [
                theme.blue(),
                theme.magenta(),
                theme.cyan(),
                theme.green(),
                theme.yellow(),
            ],
            glass,
            tyre,
            hub: if dark {
                theme.muted
            } else {
                shades.tint(theme.muted, 0.35, 1.9)
            },
            brake,
            glow: shades.tint(brake, 0.6, 2.2),
            head: theme.bright_yellow(),
            cone: (
                warning_colour(theme),
                if dark {
                    paint::strong(theme, theme.foreground)
                } else {
                    shades.tint(theme.info, 0.02, 1.04)
                },
            ),
            housing: shaded(theme, housing),
            pole: solid(theme, theme.muted, shades.darkest(), 0.3),
            unlit: shades.tint(theme.muted, 0.3, 1.8),
            halo: shades.tint(brake, 0.45, 2.0),
            puff: if dark {
                shades.tint(theme.foreground, 0.4, 2.2)
            } else {
                shades.tint(theme.muted, 0.3, 1.55)
            },
        }
    }

    /// A car's body in three tones: lit top, body and shaded lower half.
    fn body(&self, theme: &ThemeData, colour: usize) -> (Rgb, Rgb, Rgb) {
        let (dark, body, light) = shaded(theme, self.cars[colour % self.cars.len()]);
        (light, body, dark)
    }
}

/// Where everything sits at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    big: bool,
    /// Each sprite pixel is drawn as a square this many pixels wide.
    scale: i32,
    lanes: usize,
    /// The pixel row of the top of the far pavement.
    road_top: i32,
    /// Rows from the top of one lane to the top of the next.
    lane_rows: i32,
}

impl Layout {
    fn new(width: usize, height: usize) -> Self {
        let tall = height as i32 * 2;
        let wide = width as i32;
        let big = tall >= 44 && wide >= 60;
        let scale = if tall >= 170 && wide >= 260 { 2 } else { 1 };
        let car = if big { 10 } else { 6 } * scale;
        let lane_rows = car + 3 * scale;
        let pavement = 2 * scale;
        let two = tall - (pavement + 2 * lane_rows + 1) >= 12;
        let lanes = if two { 2 } else { 1 };
        let road_top = (tall - pavement - lanes as i32 * lane_rows - 1).max(0);
        Self {
            big,
            scale,
            lanes,
            road_top,
            lane_rows,
        }
    }

    fn car_rows(&self) -> i32 {
        (if self.big { 10 } else { 6 }) * self.scale
    }

    /// The top pixel row of a lane's band of road.
    fn lane_top(&self, lane: usize) -> i32 {
        self.road_top + 2 * self.scale + lane as i32 * self.lane_rows
    }

    /// The top pixel row of a lane's cars, their wheels just above the band's lower edge.
    fn car_top(&self, lane: usize) -> i32 {
        self.lane_top(lane) + self.lane_rows - self.car_rows() - self.scale
    }

    fn sprite(&self, kind: Kind) -> &'static [&'static str] {
        match (self.big, kind) {
            (true, Kind::Sedan) => &SEDAN,
            (true, Kind::Hatch) => &HATCH,
            (true, Kind::Van) => &VAN,
            (false, Kind::Van) => &SMALL_VAN,
            (false, _) => &SMALL_SEDAN,
        }
    }

    fn car_width(&self, kind: Kind) -> i32 {
        self.sprite(kind)[0].len() as i32 * self.scale
    }
}

/// Draws a sprite with each of its pixels as a `scale` by `scale` square.
fn sprite_at(canvas: &mut Canvas, x: i32, y: i32, rows: &[&str], scale: i32, p: &[(char, Rgb)]) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if let Some(&(_, colour)) = p.iter().find(|(key, _)| *key == ch) {
                canvas.fill_rect(
                    x + dx as i32 * scale,
                    y + dy as i32 * scale,
                    scale,
                    scale,
                    colour,
                );
            }
        }
    }
}

pub struct TrafficJam {
    width: usize,
    height: usize,
    motion: Motion,
    seed: u32,
    layout: Layout,
    lanes: Vec<Vec<Car>>,
    arrive_in: Vec<f32>,
    puffs: Vec<Puff>,
    clouds: Vec<Cloud>,
}

impl TrafficJam {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut jam = Self {
            width: 1,
            height: 1,
            motion,
            seed: (rng.next_u64() >> 32) as u32,
            layout: Layout::new(1, 1),
            lanes: Vec::new(),
            arrive_in: Vec::new(),
            puffs: Vec::new(),
            clouds: Vec::new(),
        };
        jam.resize(width, height, rng);
        // the jam is already there: each lane starts a few cars short of full
        for lane in 0..jam.lanes.len() {
            let mut queue: Vec<Car> = Vec::new();
            loop {
                let mut car = jam.new_car(rng);
                if let Some(ahead) = queue.last() {
                    car.colour = (ahead.colour + 1 + rng.below(4)) % 5;
                }
                let rest = jam.rest_for(&queue, &car);
                if rest < -(jam.layout.car_width(car.kind) as f32) * 0.5 {
                    break;
                }
                queue.push(Car { x: rest, ..car });
            }
            let keep = queue.len().saturating_sub(STILL_TO_COME);
            queue.truncate(keep.max(1));
            jam.lanes[lane] = queue;
        }
        jam
    }

    fn new_car(&self, rng: &mut Rng) -> Car {
        let kind = match rng.below(5) {
            0 => Kind::Van,
            1 | 2 => Kind::Hatch,
            _ => Kind::Sedan,
        };
        Car {
            x: 0.0,
            colour: rng.below(5),
            kind,
            gap: (2 + rng.below(3) as i32) * self.layout.scale,
            puff_in: rng.range(0.3, PUFF_PERIOD),
        }
    }

    /// Where the front of the first car in a lane stops: just short of the signal.
    fn stop_x(&self) -> i32 {
        let reserve = if self.layout.big { 18 } else { 10 } * self.layout.scale;
        (self.width as i32 - reserve).max(8)
    }

    /// Where a car joining the back of `queue` comes to rest.
    fn rest_for(&self, queue: &[Car], car: &Car) -> f32 {
        let front = queue
            .last()
            .map_or(self.stop_x() as f32, |ahead| ahead.x - car.gap as f32);
        front - self.layout.car_width(car.kind) as f32
    }

    /// The rest position of each car in a lane, front first.
    fn rests(&self, lane: &[Car]) -> Vec<f32> {
        let mut out: Vec<f32> = Vec::with_capacity(lane.len());
        let mut front = self.stop_x() as f32;
        for car in lane {
            if let Some(&ahead) = out.last() {
                front = ahead - car.gap as f32;
            }
            out.push(front - self.layout.car_width(car.kind) as f32);
        }
        out
    }

    /// The column of the signal's pole.
    fn signal_x(&self) -> i32 {
        self.stop_x() + if self.layout.big { 12 } else { 7 } * self.layout.scale
    }

    fn paint_backdrop(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = canvas.width() as i32;
        let road = self.layout.road_top;
        gradient(canvas, &c.sky, 0, road);
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud.0, c.cloud.1);
        }
        if road < 10 {
            return;
        }
        // the town: a far row of blocks and a nearer one, flat roofed, with a few lit windows
        let verge_top = road - (road / 7).clamp(2, 10);
        for (row, (colour, height, salt)) in [
            (c.skyline.0, road as f32 * 0.42, 71_u32),
            (c.skyline.1, road as f32 * 0.3, 72_u32),
        ]
        .into_iter()
        .enumerate()
        {
            let mut x = -(unit(0, 0, salt) * 6.0) as i32;
            let mut i = 0;
            while x < wide {
                let w = 5 + (unit(i, 1, salt + self.seed) * 8.0) as i32;
                let h = (height * (0.45 + 0.55 * unit(i, 2, salt + self.seed))) as i32;
                let top = (verge_top - h).max(1);
                canvas.fill_rect(x, top, w, verge_top - top + 1, colour);
                // a few lit windows on the near blocks, in rows, so they read as floors
                if row == 1 && w >= 6 && c.lit_windows {
                    for wy in (top + 2..verge_top - 1).step_by(3) {
                        for wx in (x + 1..x + w - 1).step_by(2) {
                            if unit(wx, wy, salt ^ self.seed) < 0.22 {
                                canvas.set(wx, wy, c.window);
                            }
                        }
                    }
                }
                x += w + if row == 0 {
                    0
                } else {
                    (unit(i, 3, salt) * 3.0) as i32
                };
                i += 1;
            }
        }
        // the verge, with a line of round trees along it
        canvas.fill_rect(0, verge_top, wide, road - verge_top, c.verge);
        let r = ((road - verge_top) as f32 * 0.9).clamp(1.6, 4.5);
        let mut x = (unit(5, 5, self.seed) * 6.0) as i32;
        let mut i = 0;
        while x < wide + 4 {
            let size = r * (0.75 + 0.4 * unit(i, 6, self.seed));
            let cy = verge_top as f32 - size * 0.4;
            // a shaded lower half and a lit upper half, light from above
            paint::disc(canvas, x as f32, cy, size, c.trees.1);
            paint::disc(
                canvas,
                x as f32 - 0.4,
                cy - size * 0.35,
                size * 0.7,
                c.trees.0,
            );
            x += (size * 2.6) as i32 + (unit(i, 7, self.seed) * 5.0) as i32;
            i += 1;
        }
    }

    fn paint_road(&self, canvas: &mut Canvas, c: &Colours) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let l = self.layout;
        let road = l.road_top;
        canvas.fill_rect(0, road, wide, 2 * l.scale, c.pavement);
        canvas.fill_rect(0, road + 2 * l.scale - 1, wide, 1, c.kerb);
        let top = l.lane_top(0);
        let span = (tall - top).max(1) as f32;
        for y in top..tall {
            canvas.fill_row(y, c.road.at((y - top) as f32 / span));
        }
        canvas.fill_row(tall - 1, c.kerb);
        if l.lanes == 2 {
            let y = l.lane_top(1);
            let dash = 6 * l.scale;
            for x in (0..wide).filter(|x| x % (dash * 2) < dash) {
                canvas.fill_rect(x, y - l.scale, 1, l.scale, c.marking);
            }
        }
    }

    fn paint_cars(&self, canvas: &mut Canvas, c: &Colours, theme: &ThemeData) {
        let l = self.layout;
        let mut blender = Blender::new(4);
        for (lane, cars) in self.lanes.iter().enumerate() {
            let top = l.car_top(lane);
            for car in cars {
                let rows = l.sprite(car.kind);
                let x = car.x.round() as i32;
                // the brake light's glow lies on the road and the air behind the car
                let brake_row = rows.iter().position(|r| r.starts_with('r')).unwrap_or(0) as i32;
                let by = top + brake_row * l.scale;
                for dy in 0..l.scale {
                    for dx in 1..=2 * l.scale {
                        let near = 1.0 - (dx as f32 / (2.5 * l.scale as f32));
                        blender.blend(canvas, x - dx, by + dy, c.glow, 0.8 * near);
                    }
                }
                let (lit, body, shade) = c.body(theme, car.colour);
                let palette = [
                    ('l', lit),
                    ('b', body),
                    ('d', shade),
                    ('w', c.glass),
                    ('k', c.tyre),
                    ('h', c.hub),
                    ('r', c.brake),
                    ('e', c.head),
                ];
                sprite_at(canvas, x, top, rows, l.scale, &palette);
            }
        }
    }

    fn paint_puffs(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(4);
        let s = self.layout.scale as f32;
        for puff in &self.puffs {
            let life = puff.age / PUFF_LIFE;
            let r = s * (0.8 + 1.6 * life);
            let strength = 0.75 * (1.0 - life);
            let reach = r.ceil() as i32 + 1;
            let (px, py) = (puff.x.round() as i32, puff.y.round() as i32);
            for y in py - reach..=py + reach {
                for x in px - reach..=px + reach {
                    let (dx, dy) = (x as f32 - puff.x, y as f32 - puff.y);
                    if dx * dx + dy * dy <= (r + 0.3) * (r + 0.3) {
                        blender.blend(canvas, x, y, c.puff, strength);
                    }
                }
            }
        }
    }

    /// The signal and the cones past the front of the queue.
    fn paint_furniture(&self, canvas: &mut Canvas, c: &Colours, theme_dark: bool) {
        let l = self.layout;
        let s = l.scale;
        let road = l.road_top;
        let sx = self.signal_x();
        let (dark, body, light) = c.housing;
        let mut blender = Blender::new(4);
        for lane in 0..l.lanes {
            let rows: &[&str] = if l.big { &CONE } else { &SMALL_CONE };
            let foot = l.lane_top(lane) + l.lane_rows - s;
            let x = self.stop_x() + 2 * s;
            sprite_at(
                canvas,
                x,
                foot - rows.len() as i32 * s,
                rows,
                s,
                &[('o', c.cone.0), ('w', c.cone.1)],
            );
        }
        // the signal stands on the far pavement; its housing reaches up into the sky
        let lamp = if l.big { 3 * s } else { 2 * s };
        let housing_h = 3 * lamp + 2 * s;
        let housing_w = lamp + 2 * s;
        // the pole is as tall as there is room for, up to a full height
        let pole = (road - housing_h - 2).clamp(2 * s, (if l.big { 10 } else { 4 }) * s);
        let top = road + s - housing_h - pole;
        if top < 1 {
            // no room to stand it up: a red lamp on a stub at the roadside
            if road >= 2 {
                paint::disc(canvas, sx as f32, road as f32 - 1.0, 1.2, c.brake);
            }
            return;
        }
        canvas.fill_rect(sx, top + housing_h, s, road + s - top - housing_h, c.pole);
        let hx = sx - housing_w / 2;
        canvas.fill_rect(hx, top, housing_w, housing_h, body);
        canvas.fill_rect(hx, top, s, housing_h, light);
        canvas.fill_rect(hx + housing_w - s, top, s, housing_h, dark);
        canvas.fill_rect(
            hx + s,
            top,
            housing_w - 2 * s,
            s,
            if theme_dark { light } else { body },
        );
        let cx = hx as f32 + housing_w as f32 / 2.0 - 0.5;
        let r = lamp as f32 / 2.0 - 0.1;
        for i in 0..3 {
            let cy = (top + s + i * lamp) as f32 + lamp as f32 / 2.0 - 0.5;
            if i == 0 {
                // a soft halo round the lit lamp, on the sky beside the housing
                for y in (cy - r * 3.0) as i32..=(cy + r * 3.0) as i32 {
                    for x in (cx - r * 3.5) as i32..=(cx + r * 3.5) as i32 {
                        let inside = (hx..hx + housing_w).contains(&x)
                            && (top..top + housing_h).contains(&y);
                        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                        if !inside && d <= r * 3.2 {
                            blender.blend(canvas, x, y, c.halo, 0.6 * (1.0 - d / (r * 3.4)));
                        }
                    }
                }
                paint::disc(canvas, cx, cy, r.max(0.5), c.brake);
            } else {
                paint::disc(canvas, cx, cy, r.max(0.5), c.unlit);
            }
        }
    }

    /// The closed board's text row in cells and its left column, when there is room for it.
    fn board(&self) -> Option<(i32, i32)> {
        let l = self.layout;
        if !l.big || self.width < 50 {
            return None;
        }
        let row = (l.road_top - 9) / 2;
        let left = self.signal_x() - 19;
        (row >= 2).then_some((left, row))
    }

    /// The closed board: a plain panel between red and white stripes on two legs, standing on
    /// the verge left of the signal. The text is written over the panel after the blit.
    fn paint_board(&self, canvas: &mut Canvas, c: &Colours, theme: &ThemeData) {
        let Some((left, row)) = self.board() else {
            return;
        };
        let (y0, y1) = (row * 2 - 2, row * 2 + 3);
        for x in left..left + 13 {
            let stripe = if ((x - left) / 2) % 2 == 0 {
                c.brake
            } else {
                c.cone.1
            };
            canvas.set(x, y0, stripe);
            canvas.set(x, y0 + 1, stripe);
            canvas.set(x, y1 - 1, stripe);
            canvas.set(x, y1, stripe);
        }
        canvas.fill_rect(left, row * 2, 13, 2, theme.background);
        for leg in [left + 2, left + 10] {
            canvas.fill_rect(leg, y1 + 1, 1, self.layout.road_top + 1 - y1, c.pole);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours, theme: &ThemeData) {
        self.paint_backdrop(canvas, c);
        self.paint_road(canvas, c);
        self.paint_board(canvas, c, theme);
        self.paint_furniture(canvas, c, theme.dark);
        self.paint_cars(canvas, c, theme);
        self.paint_puffs(canvas, c);
    }
}

impl Scene for TrafficJam {
    fn name(&self) -> &'static str {
        "traffic-jam"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let old = self.layout;
        self.layout = Layout::new(self.width, self.height);
        let lanes = self.layout.lanes;
        let mut kept: Vec<Vec<Car>> = std::mem::take(&mut self.lanes);
        if old.scale != self.layout.scale {
            kept.clear();
        }
        kept.resize(lanes, Vec::new());
        kept.truncate(lanes);
        // the cars already queued keep their places, as many as still fit
        for lane in &mut kept {
            let rests = self.rests(lane);
            for (car, rest) in lane.iter_mut().zip(&rests) {
                car.x = car.x.min(*rest);
            }
            while lane
                .last()
                .is_some_and(|c| c.x < -(self.layout.car_width(c.kind) as f32))
            {
                lane.pop();
            }
        }
        self.lanes = kept;
        self.arrive_in = (0..lanes).map(|i| 0.6 + i as f32 * 1.1).collect();
        self.puffs.clear();
        let road = self.layout.road_top as f32;
        let count = (self.width / 40).clamp(1, 6);
        let speed = 0.8 * self.motion.speed();
        // the clouds are laid out from the scene's own seed, so a resize keeps the same sky
        let mut layout = Rng::new(u64::from(self.seed) ^ 0x7a1);
        self.clouds = (0..count)
            .map(|_| {
                Cloud::random(
                    &mut layout,
                    self.width as f32,
                    (road * 0.12 + 3.0, road * 0.35 + 3.0),
                    (10.0, 22.0),
                    speed,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let speed = self.motion.speed();
        let pace = self.motion.pace();
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, width);
        }
        let scale = self.layout.scale;
        for lane in 0..self.lanes.len() {
            self.arrive_in[lane] -= dt * speed;
            let mut candidate = self.new_car(rng);
            if let Some(ahead) = self.lanes[lane].last() {
                candidate.colour = (ahead.colour + 1 + rng.below(4)) % 5;
            }
            let rest = self.rest_for(&self.lanes[lane], &candidate);
            let w = self.layout.car_width(candidate.kind) as f32;
            let room = rest > -w * 0.5;
            let entry_clear = self.lanes[lane]
                .last()
                .is_none_or(|c| c.x > (2 * scale) as f32);
            if self.arrive_in[lane] <= 0.0 && room && entry_clear {
                self.arrive_in[lane] = ARRIVAL_PERIOD;
                self.lanes[lane].push(Car { x: -w, ..candidate });
            }
            let rests = self.rests(&self.lanes[lane]);
            let widths: Vec<f32> = self.lanes[lane]
                .iter()
                .map(|car| self.layout.car_width(car.kind) as f32)
                .collect();
            let car_top = self.layout.car_top(lane);
            let car_rows = self.layout.car_rows();
            let mut ahead: Option<f32> = None;
            for (place, rest) in rests.iter().enumerate() {
                let car = &mut self.lanes[lane][place];
                // a car rolls in and eases to a stop, never closer than its gap to the one ahead
                let target = match ahead {
                    Some(front) => rest.min(front - car.gap as f32 - widths[place]),
                    None => *rest,
                };
                let left = (target - car.x).max(0.0);
                let pace_now = (left * 1.2).clamp(3.0, 18.0) * speed;
                car.x += (pace_now * dt).min(left);
                if left < 0.5 && target >= *rest - 0.01 {
                    car.x = *rest;
                    car.puff_in -= dt * pace;
                    if car.puff_in <= 0.0 {
                        car.puff_in = PUFF_PERIOD;
                        self.puffs.push(Puff {
                            x: car.x - scale as f32,
                            y: (car_top + car_rows - 3 * scale) as f32,
                            age: 0.0,
                        });
                    }
                }
                ahead = Some(car.x);
            }
        }
        for puff in &mut self.puffs {
            puff.age += dt * pace;
            puff.x -= 1.4 * speed * dt;
            puff.y -= 1.3 * speed * dt;
        }
        self.puffs.retain(|p| p.age < PUFF_LIFE);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme);
        canvas.blit(frame);
        if let Some((left, row)) = self.board() {
            paint::label(frame, left + 1, row, "ROAD CLOSED", colours.brake);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("traffic-jam");
        check_determinism("traffic-jam");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("traffic-jam", 40.0);
    }

    #[test]
    fn the_queue_fills_to_the_edge_and_never_moves_past_the_signal() {
        let mut rng = Rng::new(3);
        let mut jam = TrafficJam::new(100, 30, Motion::LIVELY, &mut rng);
        for _ in 0..(5 * 60 * 20) {
            jam.step(0.05, &mut rng);
        }
        assert_eq!(jam.lanes.len(), 2);
        for lane in &jam.lanes {
            assert!(lane.len() >= 3, "{}", lane.len());
            let rests = jam.rests(lane);
            for (car, rest) in lane.iter().zip(&rests) {
                assert_eq!(car.x, *rest);
                assert!(car.x as i32 + jam.layout.car_width(car.kind) <= jam.stop_x());
            }
            // the back of the queue is at the edge of the screen
            let last = lane.last().unwrap();
            assert!(last.x < 30.0, "{}", last.x);
        }
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let frame = render(&mut jam, 100, 30, 1);
        assert!(frame.to_text().contains("ROAD CLOSED"));
        let all = pixels(&frame);
        assert!(all.contains(&c.brake) && all.contains(&c.unlit) && all.contains(&c.cone.0));
        assert!(all.contains(&c.glass) && all.contains(&c.hub));
    }

    #[test]
    fn the_jam_is_there_from_the_start() {
        let mut rng = Rng::new(5);
        let jam = TrafficJam::new(120, 34, Motion::FULL, &mut rng);
        for lane in &jam.lanes {
            assert!(lane.len() >= 2, "{}", lane.len());
        }
    }

    #[test]
    fn puffs_are_slow_on_every_level_and_sprites_are_even() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let jam = TrafficJam::new(80, 24, motion, &mut rng);
            assert!(PUFF_PERIOD / jam.motion.pace() >= PUFF_PERIOD);
        }
        for rows in [
            &SEDAN[..],
            &HATCH[..],
            &VAN[..],
            &SMALL_SEDAN[..],
            &SMALL_VAN[..],
            &CONE[..],
            &SMALL_CONE[..],
        ] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()));
        }
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.sky.at(1.0),
                c.skyline.1,
                0.03,
                "the sky and the town",
            );
            apart(
                theme,
                c.verge,
                c.pavement,
                0.03,
                "the verge and the pavement",
            );
            apart(
                theme,
                c.brake,
                c.unlit,
                0.1,
                "the red lamp and the dark ones",
            );
            apart(
                theme,
                c.glass,
                c.road.at(0.5),
                0.03,
                "the glass and the road",
            );
            for car in c.cars {
                apart(theme, car, c.road.at(0.5), 0.1, "a car and the road");
                apart(theme, car, c.glass, 0.08, "a car and its windows");
            }
            for colour in c
                .sky
                .steps()
                .iter()
                .chain(c.road.steps())
                .chain([&c.unlit, &c.puff, &c.glow, &c.halo, &c.glass])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.brake, c.cone.0, c.housing.1, c.head] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
