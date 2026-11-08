//! A listing ship: a container ship on a grey sea under rain that slowly tips bow down over the
//! first minute while its waterline creeps up past the portholes, then rolls gently at that
//! angle with a lifebuoy and a crate drifting away from it.
//!
//! The ship is drawn by turning each pixel back into the ship's own frame, so the hull, the
//! stacked containers, the bridge and the mast all tip together as one solid shape. What sinks
//! below the surface is muddied by the sea, which is a tint of the background like the sky.

use std::f32::consts::TAU;

use super::paint::{self, Blender, Drop, Overcast, clouds, ease, gradient, shaded, solid};
use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

/// Seconds the ship takes to settle at its list at the standard pace.
pub const LIST_SECONDS: f32 = 50.0;
/// The angle the ship settles at, in degrees, bow down to the right.
pub const MAX_LIST: f32 = 8.0;
/// Seconds of one slow roll once it has settled. The roll is small and slow, so an edge of the
/// ship crosses a pixel at a time.
pub const ROLL_PERIOD: f32 = 11.0;
/// How far the ship rolls either way, in degrees.
const ROLL: f32 = 1.5;
/// How many pixels the ship sinks as it lists.
const SINK_PIXELS: f32 = 2.0;

pub const SUMMARY: &str = "A container ship on a grey sea that slowly lists bow down in the rain as its waterline creeps up";

pub const CAPTIONS: &[&str] = &[
    "Taking on water. Plug the first leak.",
    "A list to starboard. Trim it with one fix.",
    "Still afloat. Read the error, then right the ship.",
    "All hands to the first error.",
    "Ships list. Builds recover.",
];

/// A lifebuoy and a crate, the things that have gone over the side.
const BUOY: [&str; 4] = [".rw.", "r..r", "w..w", ".wr."];
const CRATE: [&str; 2] = ["cCc", "CcC"];

/// A part of the ship, found from a point in the ship's own frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Hull,
    Boot,
    Stripe,
    Porthole,
    Container(usize),
    Seam,
    Bridge,
    Glass,
    Funnel,
    Band,
    Mast,
    Flag,
}

/// The ship's measurements in pixels.
#[derive(Clone, Copy, Debug)]
struct Hull {
    length: f32,
    depth: f32,
}

impl Hull {
    /// What sits at a point `u` pixels along from the stern and `v` up from the keel.
    fn part(&self, u: f32, v: f32) -> Option<Part> {
        let (l, d) = (self.length, self.depth);
        if u < 0.0 || v < 0.0 {
            return None;
        }
        if v < d {
            // a rounded stern and a bow that rakes forward as it rises
            let stern = (1.5 - v).max(0.0) * 1.2;
            let bow = l - 4.0 + (v / d) * 4.0;
            if u < stern || u > bow {
                return None;
            }
            let row = v.floor();
            if row == (d * 0.75).floor() {
                return Some(Part::Stripe);
            }
            if row < (d * 0.35).max(1.0) {
                return Some(Part::Boot);
            }
            let porthole = row == (d * 0.55).floor() && (u as i32) % 5 == 2 && u > 3.0;
            return Some(if porthole && u < l - 6.0 {
                Part::Porthole
            } else {
                Part::Hull
            });
        }
        let above = v - d;
        // the bridge at the stern, its funnel behind it
        let bridge = (l * 0.05, l * 0.2);
        let bridge_tall = (d * 1.6).max(3.0);
        if u >= bridge.0 && u < bridge.1 && above < bridge_tall {
            let window = above.floor() == (bridge_tall - 2.0).floor() && (u as i32) % 2 == 0;
            return Some(if window { Part::Glass } else { Part::Bridge });
        }
        let funnel = (l * 0.08, l * 0.08 + 3.0);
        if u >= funnel.0 && u < funnel.1 && above >= bridge_tall && above < bridge_tall + 3.0 {
            return Some(if above < bridge_tall + 1.0 {
                Part::Band
            } else {
                Part::Funnel
            });
        }
        // containers stacked two high along the deck, each four pixels long
        let stack = (l * 0.24, l * 0.82);
        if u >= stack.0 && u < stack.1 && above < 4.0 {
            let column = ((u - stack.0) / 4.0) as usize;
            let tier = (above / 2.0) as usize;
            if (u - stack.0) % 4.0 < 1.0 {
                return Some(Part::Seam);
            }
            return Some(Part::Container(column * 2 + tier * 3));
        }
        // the foremast with its flag
        let mast = l * 0.9;
        let mast_tall = (d * 2.0).max(4.0);
        if (u - mast).abs() < 0.5 && above < mast_tall {
            return Some(Part::Mast);
        }
        if u > mast && u < mast + 3.0 && above >= mast_tall - 2.0 && above < mast_tall {
            return Some(Part::Flag);
        }
        None
    }

    /// How far the ship reaches above its keel, for the area to search.
    fn reach(&self) -> f32 {
        self.depth * 3.2 + 4.0
    }
}

#[derive(Clone, Debug)]
struct Float {
    x: f32,
    speed: f32,
    crate_: bool,
}

/// The ship's colours in one theme.
struct Colours {
    overcast: Overcast,
    sea: Ramp,
    crest: Rgb,
    hull: (Rgb, Rgb, Rgb),
    boot: Rgb,
    stripe: Rgb,
    porthole: Rgb,
    containers: [Rgb; 6],
    seam: Rgb,
    bridge: Rgb,
    glass: Rgb,
    funnel: Rgb,
    flag: Rgb,
    buoy: (Rgb, Rgb),
    crate_: (Rgb, Rgb),
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let grey_sea = mix(theme.muted, theme.blue(), 0.4);
        let (top, deep) = if theme.dark {
            (
                shades.tint(grey_sea, 0.5, 2.3),
                shades.tint(grey_sea, 0.22, 1.35),
            )
        } else {
            (
                shades.tint(grey_sea, 0.35, 1.8),
                shades.tint(grey_sea, 0.75, 2.8),
            )
        };
        let crest = if theme.dark {
            mix(top, shades.tint(theme.foreground, 0.6, 2.9), 0.6)
        } else {
            mix(top, theme.background, 0.6)
        };
        let hull_base = solid(theme, theme.muted, shades.darkest(), 0.4);
        let (dark_crate, crate_, _) = shaded(theme, theme.yellow());
        Self {
            overcast: Overcast::new(theme),
            sea: Ramp::new(top, deep, 10),
            crest,
            hull: shaded(theme, hull_base),
            boot: theme.red(),
            stripe: theme.foreground,
            porthole: theme.yellow(),
            containers: [
                theme.blue(),
                theme.green(),
                theme.yellow(),
                theme.cyan(),
                theme.magenta(),
                theme.bright_red(),
            ]
            .map(|c| solid(theme, c, shades.darkest(), 0.15)),
            seam: hull_base,
            bridge: theme.foreground,
            glass: solid(theme, theme.info, shades.darkest(), 0.3),
            funnel: hull_base,
            flag: error_colour(theme),
            buoy: (error_colour(theme), theme.foreground),
            crate_: (crate_, dark_crate),
        }
    }

    fn of(&self, part: Part) -> Rgb {
        match part {
            Part::Hull => self.hull.1,
            Part::Boot => self.boot,
            Part::Stripe => self.stripe,
            Part::Porthole => self.porthole,
            Part::Container(i) => self.containers[i % self.containers.len()],
            Part::Seam => self.seam,
            Part::Bridge => self.bridge,
            Part::Glass => self.glass,
            Part::Funnel => self.funnel,
            Part::Band => self.boot,
            Part::Mast => self.hull.2,
            Part::Flag => self.flag,
        }
    }
}

pub struct ListingShip {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    seed: u32,
    /// Seconds of listing so far, kept across a resize so the ship never rights itself.
    listed: f32,
    floats: Vec<Float>,
    drops: Vec<Drop>,
}

impl ListingShip {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut ship = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            listed: 0.0,
            floats: Vec::new(),
            drops: Vec::new(),
        };
        ship.resize(width, height, rng);
        ship
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the horizon, where the sea starts.
    fn horizon(&self) -> i32 {
        ((self.tall() as f32 * 0.55) as i32).max(2)
    }

    fn hull(&self) -> Hull {
        let length = if self.width >= 70 && self.height >= 16 {
            46.0
        } else {
            (self.width as f32 * 0.6).clamp(12.0, 40.0)
        };
        Hull {
            length,
            depth: (length / 7.0).clamp(3.0, 7.0),
        }
    }

    /// The ship's angle in degrees.
    fn angle(&self) -> f32 {
        let settled = ease(self.listed / LIST_SECONDS);
        let roll = ROLL * settled * cycle(self.t, TAU / ROLL_PERIOD * self.motion.pace()).sin();
        MAX_LIST * settled + roll
    }

    /// How far the ship has sunk, in pixels.
    fn sink(&self) -> f32 {
        SINK_PIXELS * ease(self.listed / LIST_SECONDS)
    }

    /// The column of the ship's centre: left of the middle on a wide screen, so a panel in the
    /// middle of the watch mode leaves it in view.
    fn centre(&self) -> f32 {
        let half = self.hull().length / 2.0 + 2.0;
        if self.width >= 70 {
            (self.width as f32 * 0.3).max(half)
        } else {
            self.width as f32 / 2.0
        }
    }

    /// The surface of the sea at a column, rising and falling with a slow swell.
    fn surface(&self, x: i32) -> f32 {
        let phase = cycle(self.t, 0.5 * self.motion.speed());
        self.horizon() as f32 + 1.0 + 0.6 * (x as f32 * 0.16 - phase).sin()
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(5);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let horizon = self.horizon();
        gradient(canvas, &c.overcast.sky, 0, horizon);
        let drift = (self.t % 100_000.0) as f32 * 0.5 * self.motion.speed();
        clouds(
            canvas,
            &mut mixer,
            &c.overcast.cloud,
            -2,
            (horizon as f32 * 0.7) as i32,
            drift,
            self.seed,
            0.75,
        );
        // the rain blends in two steps only, which keeps the number of colours it adds small
        paint::rain(
            canvas,
            &mut Blender::new(2),
            &self.drops,
            &c.overcast,
            0.4,
            |_| horizon,
        );

        // a grey sea darkening towards the viewer, with highlights drifting across it that
        // fade with distance from the horizon
        let span = (tall - horizon).max(1) as f32;
        let phase = cycle(self.t, 0.6 * self.motion.pace());
        let threshold = if self.motion.calm { 1.25 } else { 1.05 };
        for y in horizon..tall {
            let depth = (y - horizon) as f32 / span;
            for x in 0..wide {
                let (xf, yf) = (x as f32, y as f32);
                let v = (xf * 0.27 + yf * 1.1 - phase).sin()
                    + 0.7 * (xf * 0.13 - yf * 0.7 + phase * 0.6 + 1.0).sin();
                let lift = ((v - threshold) / 0.45).clamp(0.0, 1.0) * (1.0 - depth * 0.5);
                let base = c.sea.at(depth);
                canvas.set(x, y, mixer.mix(base, c.crest, lift));
            }
        }
        if horizon < tall {
            canvas.fill_row(horizon, mixer.mix(c.sea.at(0.0), c.crest, 0.5));
        }

        // what has gone over the side drifts away along the surface
        if self.listed > 4.0 {
            for float in &self.floats {
                let x = float.x.round() as i32;
                let y = self.surface(x).round() as i32;
                if float.crate_ {
                    canvas.sprite(x, y - 1, &CRATE, &[('c', c.crate_.0), ('C', c.crate_.1)]);
                } else {
                    canvas.sprite(x, y - 2, &BUOY, &[('r', c.buoy.0), ('w', c.buoy.1)]);
                }
            }
        }

        self.paint_ship(canvas, &mut mixer, c);
    }

    /// Draws the ship tipped by its angle: every pixel near it is turned back into the ship's
    /// frame and coloured by the part found there, so the whole ship tips as one shape. The
    /// stern stays put and the bow goes down, the way a ship taking on water at the bow settles.
    fn paint_ship(&self, canvas: &mut Canvas, mixer: &mut Blender, c: &Colours) {
        let hull = self.hull();
        let theta = self.angle().to_radians();
        let (along, up) = ((theta.cos(), theta.sin()), (theta.sin(), -theta.cos()));
        // the ship turns about a point a third of the way along its keel, so the stern lifts a
        // little as the bow goes down
        let turn = hull.length * 0.35;
        let keel = self.horizon() as f32 + hull.depth * 0.3 + self.sink();
        let centre = (self.centre() - hull.length / 2.0 + turn, keel);
        let pivot = (centre.0 - turn * along.0, centre.1 - turn * along.1);
        let reach = hull.length + hull.reach();
        let span = self.tall() as f32 - self.horizon() as f32;
        for y in (pivot.1 - reach).floor() as i32..=(pivot.1 + reach).ceil() as i32 {
            for x in (pivot.0 - 4.0).floor() as i32..=(pivot.0 + reach).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - pivot.0, y as f32 + 0.5 - pivot.1);
                let u = dx * along.0 + dy * along.1;
                let v = dx * up.0 + dy * up.1;
                let Some(part) = hull.part(u, v) else {
                    continue;
                };
                let mut colour = c.of(part);
                // the side away from the light, the lowest row of the hull, is in shadow
                if part == Part::Hull && v < hull.depth * 0.5 {
                    colour = c.hull.0;
                }
                canvas.set(x, y, colour);
                // below the surface the ship is seen through grey water
                if y as f32 > self.surface(x) {
                    let depth = ((y - self.horizon()) as f32 / span.max(1.0)).min(1.0);
                    mixer.blend(canvas, x, y, c.sea.at(depth), 0.75);
                }
            }
        }
    }
}

impl Scene for ListingShip {
    fn name(&self) -> &'static str {
        "listing-ship"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 900, 1, 4));
        self.floats = (0..wanted)
            .map(|i| Float {
                x: rng.range(0.0, self.width as f32),
                speed: rng.range(0.4, 0.9),
                crate_: i % 2 == 1,
            })
            .collect();
        let drops = self
            .motion
            .scaled(count_for(self.width, self.height, 40, 2, 160));
        let drops = if self.motion.calm { drops / 2 } else { drops };
        let horizon = self.horizon() as f32;
        self.drops = (0..drops)
            .map(|_| Drop {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(0.0, horizon),
                speed: rng.range(12.0, 18.0),
                near: rng.chance(0.35),
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.listed = (self.listed + dt * speed).min(LIST_SECONDS * 2.0);
        let width = self.width as f32;
        for float in &mut self.floats {
            float.x = wrap(float.x + float.speed * speed * dt + 6.0, width + 12.0) - 6.0;
        }
        let horizon = self.horizon() as f32;
        for drop in &mut self.drops {
            drop.y += drop.speed * speed * dt;
            drop.x = wrap(drop.x + 2.4 * speed * dt, width);
            if drop.y >= horizon {
                drop.y = -rng.range(0.0, 4.0);
                drop.x = rng.range(0.0, width);
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
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
        check_scene("listing-ship");
        check_determinism("listing-ship");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("listing-ship", 40.0);
    }

    #[test]
    fn the_ship_lists_slowly_then_rolls_gently_at_its_angle() {
        let mut rng = Rng::new(4);
        let mut ship = ListingShip::new(80, 24, Motion::FULL, &mut rng);
        assert!(ship.angle().abs() < 0.01);
        let mut last = ship.angle();
        let mut fastest = 0.0_f32;
        for _ in 0..(600 * 20) {
            ship.step(0.05, &mut rng);
            fastest = fastest.max((ship.angle() - last).abs() * 20.0);
            last = ship.angle();
        }
        assert!(fastest < 2.0, "{fastest} degrees a second");
        for _ in 0..(60 * 20) {
            ship.step(0.05, &mut rng);
            let angle = ship.angle();
            assert!((MAX_LIST - ROLL - 0.01..=MAX_LIST + ROLL + 0.01).contains(&angle));
        }
        assert!((ship.sink() - SINK_PIXELS).abs() < 0.01);
    }

    #[test]
    fn the_ship_is_drawn_with_its_flag_hull_and_containers() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(5);
        for (w, h) in [(80, 24), (40, 12)] {
            let mut ship = ListingShip::new(w, h, Motion::FULL, &mut rng);
            let frame = render(&mut ship, w, h, 60 * 20);
            let all = pixels(&frame);
            assert!(all.contains(&c.flag), "{w}x{h}");
            assert!(
                all.contains(&c.hull.1) && all.contains(&c.bridge),
                "{w}x{h}"
            );
            assert!(c.containers.iter().any(|k| all.contains(k)), "{w}x{h}");
        }
        // every part of the hull's outline is found from its own frame
        let hull = Hull {
            length: 40.0,
            depth: 4.0,
        };
        assert_eq!(hull.part(20.0, 0.5), Some(Part::Boot));
        assert_eq!(hull.part(-1.0, 2.0), None);
        assert!(matches!(
            hull.part(20.0, 4.5),
            Some(Part::Container(_) | Part::Seam)
        ));
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.sea.at(0.0),
                c.sea.at(1.0),
                0.03,
                "near and far sea",
            );
            apart(
                theme,
                c.crest,
                c.sea.at(0.3),
                0.03,
                "the crests and the sea",
            );
            apart(
                theme,
                c.sea.at(0.0),
                c.overcast.sky.at(1.0),
                0.02,
                "the sea and the sky",
            );
            apart(theme, c.hull.1, c.sea.at(0.0), 0.1, "the hull and the sea");
            apart(
                theme,
                c.flag,
                c.overcast.sky.at(0.5),
                0.1,
                "the flag and the sky",
            );
            for colour in c.sea.steps().iter().chain([&c.crest]) {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.hull.1, c.bridge, c.flag, c.boot] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
