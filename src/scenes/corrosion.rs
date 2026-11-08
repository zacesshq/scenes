//! Corrosion: a close view of an old riveted steel wall, its plates lapped over one another and
//! fixed with rows of domed rivets, lit from the upper left. Rust has taken hold at a few of the
//! rivets and creeps outwards over the plates, faster along the seams where water lies. It
//! starts as a faint orange film, thickens into crusted scale and blisters into dark flaking
//! cores, while rusty water runs down from each patch in long stains, with a drop sliding slowly
//! down each stain now and then.
//!
//! The steel, its seams, its rivets, the thin rust film and the stains are tints of the
//! background held below the non-text floor. The crusted rust, its flakes and the running drops
//! are solid, so with colour off the rust shows as spreading shapes on a blank wall.

use super::paint::{self, Blender, noise, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Riveted steel plates where rust creeps out from the rivets, flakes and runs down in streaks";

pub const CAPTIONS: &[&str] = &[
    "Something has rusted. Scrape back to the first error.",
    "A little corrosion. Nothing a fix cannot treat.",
    "Rust spreads from one spot. So do fixes.",
    "The plates are sound underneath. Start at the rivet.",
    "Slow rot in the build. Catch it early.",
];

/// Seconds of the scene's own clock a patch takes to grow to its full size at the standard
/// pace. Its edge moves well under a pixel a second, so the rust creeps rather than jumps.
pub const GROW_SECONDS: f32 = 150.0;
/// Seconds of growth every patch already has when the scene starts, so the rust shows at once.
const HEAD_START: f32 = 40.0;
/// Pixels a second a drop slides down its stain.
pub const DROP_SPEED: f32 = 1.6;
/// Seconds a drop waits at the top of its stain before it sets off again.
const DROP_REST: f32 = 9.0;

/// What a pixel of the wall is, before any rust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Plate,
    /// The shadow a lapped plate casts along its edge.
    Seam,
    /// The lit lip of a plate beside a seam.
    Lip,
    RivetLight,
    Rivet,
    RivetShade,
    /// The small shadow a rivet head casts on the plate.
    RivetCast,
}

/// A rivet where rust has started, how far it will spread and when it started.
#[derive(Clone, Debug)]
struct Source {
    x: f32,
    y: f32,
    reach: f32,
    start: f32,
    salt: u32,
}

/// A stain running down from a patch: its column, the row it leaves the patch, how long it grows
/// and how fast, so each stain is a little different.
#[derive(Clone, Debug)]
struct Streak {
    x: i32,
    top: f32,
    length: f32,
    pace: f32,
    /// Seconds offset of its drop, so the drops never move together.
    phase: f32,
    wide: bool,
}

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight.
/// The lit side moves towards the theme's light end on a dark theme and towards the background
/// on a light one, never falling below the solid contrast, so the shape keeps its outline.
fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.6
    });
    let background = theme.background;
    let far = if theme.dark {
        Shades::new(theme).lightest()
    } else {
        Shades::new(theme).darkest()
    };
    // the path runs from the background through the hue to the theme's far end, so a tone
    // keeps as much of its hue as its contrast allows
    let (from, to) = if theme.contrast_of(hue) >= target {
        (background, hue)
    } else {
        (hue, far)
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..22 {
        let mid = (low + high) / 2.0;
        if mix(from, to, mid).contrast(background) >= target {
            high = mid;
        } else {
            low = mid;
        }
    }
    mix(from, to, high)
}

/// The wall's colours in one theme.
struct Colours {
    /// The steel from the lit corner of a plate to its shaded one.
    steel: Ramp,
    seam: Rgb,
    lip: Rgb,
    rivet: (Rgb, Rgb, Rgb),
    cast: Rgb,
    /// The thin orange film at the edge of a patch, still imagery.
    film: Rgb,
    stain: Rgb,
    /// Crusted rust: its shaded grain, its body and the lit flakes.
    scale: (Rgb, Rgb, Rgb),
    /// The dark blistered core of an old patch.
    core: Rgb,
    drop: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let steel = mix(theme.muted, theme.info, 0.3);
        // on a dark theme the steel sits high in the imagery range, so the seams, the rivets'
        // shadows and the darker stains have room below it to read as dark marks on the plate
        let (lit, shaded, seam, lip) = if theme.dark {
            (
                shades.tint(steel, 0.9, 2.7),
                shades.tint(steel, 0.6, 2.05),
                shades.tint(steel, 0.05, 1.08),
                shades.tint(mix(steel, theme.bright(), 0.5), 1.0, 2.97),
            )
        } else {
            (
                shades.tint(steel, 0.12, 1.2),
                shades.tint(steel, 0.42, 1.95),
                shades.tint(steel, 0.85, 2.85),
                shades.tint(steel, 0.02, 1.03),
            )
        };
        let orange = mix(theme.yellow(), theme.red(), 0.5);
        let brown = mix(orange, theme.muted, 0.3);
        let (film, stain) = if theme.dark {
            (shades.tint(orange, 0.85, 2.5), shades.tint(brown, 0.6, 1.6))
        } else {
            (shades.tint(orange, 0.42, 1.9), shades.tint(brown, 0.7, 2.5))
        };
        Self {
            steel: Ramp::new(lit, shaded, 7),
            seam,
            lip,
            rivet: if theme.dark {
                (lip, mix(lit, lip, 0.4), mix(shaded, seam, 0.45))
            } else {
                (lip, mix(lit, lip, 0.6), mix(lit, shaded, 0.55))
            },
            cast: if theme.dark {
                mix(shaded, seam, 0.65)
            } else {
                mix(shaded, seam, 0.2)
            },
            film,
            stain,
            scale: (
                tone(theme, brown, 0.0),
                tone(theme, orange, if theme.dark { 0.25 } else { 0.85 }),
                tone(
                    theme,
                    mix(orange, theme.yellow(), 0.3),
                    if theme.dark { 0.7 } else { 1.0 },
                ),
            ),
            core: tone(theme, mix(brown, theme.red(), 0.3), 0.0),
            drop: tone(theme, orange, if theme.dark { 0.5 } else { 0.4 }),
        }
    }
}

pub struct Corrosion {
    width: usize,
    height: usize,
    motion: Motion,
    /// The scene's own clock, in seconds of growth at the standard pace.
    t: f32,
    parts: Vec<Part>,
    sources: Vec<Source>,
    streaks: Vec<Streak>,
    seed: u32,
}

impl Corrosion {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut wall = Self {
            width: 1,
            height: 1,
            motion,
            t: HEAD_START,
            parts: Vec::new(),
            sources: Vec::new(),
            streaks: Vec::new(),
            seed: rng.next_u64() as u32,
        };
        wall.resize(width, height, rng);
        wall
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// A plate's width and height in pixels.
    fn plate(&self) -> (i32, i32) {
        (
            (self.wide() * 2 / 5).clamp(16, 96),
            (self.tall() * 9 / 20).clamp(10, 48),
        )
    }

    /// How far a rivet sits from its seam, how far apart the rivets are and whether the heads
    /// are large enough to be domed.
    fn rivet_spacing(&self) -> (i32, i32, bool) {
        if self.tall() >= 30 {
            (3, (self.tall() / 9).clamp(6, 11), true)
        } else {
            (2, 5, false)
        }
    }

    /// Where a pixel lies on its plate: the plate's column and row and the pixel's place on it.
    fn on_plate(&self, x: i32, y: i32) -> (i32, i32, i32, i32) {
        let (pw, ph) = self.plate();
        // the wall starts part of the way into a plate, so the seams never line up with the
        // edges of the screen
        let y = y + ph / 3;
        let row = y.div_euclid(ph);
        let shift = if row.rem_euclid(2) == 1 {
            pw / 2
        } else {
            pw / 5
        };
        let x = x + shift;
        (x.div_euclid(pw), row, x.rem_euclid(pw), y.rem_euclid(ph))
    }

    fn lay_out(&mut self) {
        let (w, h) = (self.wide(), self.tall());
        let (pw, ph) = self.plate();
        let (inset, gap, big) = self.rivet_spacing();
        self.parts = vec![Part::Plate; (w * h) as usize];
        let mut rivets = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let (_, _, lx, ly) = self.on_plate(x, y);
                let part = if ly == 0 || lx == 0 {
                    Part::Seam
                } else if ly == 1 || lx == 1 {
                    Part::Lip
                } else {
                    Part::Plate
                };
                self.parts[(y * w + x) as usize] = part;
                // a rivet's centre: a row under the top seam and a column beside the left one
                let along_top = ly == inset + 1 && lx > inset + 2 && (lx - gap / 2) % gap == 0;
                let along_side = lx == inset + 1 && ly > inset + 2 && (ly - gap / 2) % gap == 0;
                let clear = lx < pw - 3 && ly < ph - 3;
                if (along_top || along_side) && clear {
                    rivets.push((x, y));
                }
            }
        }
        // a domed head four pixels across with its corners rounded off, lit on its upper left
        let domed: &[(i32, i32, Part)] = &[
            (0, -1, Part::RivetLight),
            (1, -1, Part::Rivet),
            (-1, 0, Part::RivetLight),
            (0, 0, Part::RivetLight),
            (1, 0, Part::Rivet),
            (2, 0, Part::RivetShade),
            (-1, 1, Part::Rivet),
            (0, 1, Part::Rivet),
            (1, 1, Part::RivetShade),
            (2, 1, Part::RivetShade),
            (0, 2, Part::RivetShade),
            (1, 2, Part::RivetShade),
            (2, 2, Part::RivetCast),
        ];
        let small: &[(i32, i32, Part)] = &[
            (0, 0, Part::RivetLight),
            (1, 0, Part::Rivet),
            (0, 1, Part::Rivet),
            (1, 1, Part::RivetShade),
            (2, 1, Part::RivetCast),
            (1, 2, Part::RivetCast),
        ];
        let head = if big { domed } else { small };
        for &(x, y) in &rivets {
            for &(dx, dy, part) in head {
                let (px, py) = (x + dx, y + dy);
                if px >= 0 && py >= 0 && px < w && py < h {
                    self.parts[(py * w + px) as usize] = part;
                }
            }
        }
        self.rivets_to_sources(&rivets);
    }

    /// Picks the rivets where rust has started, spread across the wall, with the stains that run
    /// down from each.
    fn rivets_to_sources(&mut self, rivets: &[(i32, i32)]) {
        self.sources.clear();
        self.streaks.clear();
        if rivets.is_empty() {
            return;
        }
        let (w, h) = (self.wide(), self.tall());
        let (_, ph) = self.plate();
        let wanted = ((w * h) as f32 / 2600.0).round().clamp(2.0, 6.0) as usize;
        // the wall is split into bands across, one patch to a band, so the rust is spread out
        // rather than bunched in one corner; the seed picks a rivet in each band
        for i in 0..wanted {
            let left = w * i as i32 / wanted as i32;
            let right = w * (i as i32 + 1) / wanted as i32;
            let band: Vec<&(i32, i32)> = rivets
                .iter()
                .filter(|(x, y)| *x >= left && *x < right && *y > 2 && *y < h * 3 / 4)
                .collect();
            if band.is_empty() {
                continue;
            }
            let salt = self.seed.wrapping_add(i as u32 * 7919);
            // each band aims at a different height, so the patches are spread up and down too
            let aim = h as f32 * (0.15 + 0.55 * (i as f32 * 0.618 + unit(0, 1, self.seed)).fract());
            let &&(x, y) = band
                .iter()
                .min_by_key(|(bx, by)| {
                    ((*by as f32 - aim).abs() * 4.0 + unit(*bx, *by, salt) * 12.0) as i32
                })
                .expect("a rivet in the band");
            let reach = ph as f32 * (0.28 + 0.2 * unit(i as i32, 5, salt));
            self.sources.push(Source {
                x: x as f32,
                y: y as f32,
                reach,
                start: -unit(i as i32, 7, salt) * 25.0,
                salt,
            });
            // the stains leave the bottom of the patch at its full size
            let count = 3 + (unit(i as i32, 9, salt) * 3.0) as i32;
            for k in 0..count {
                let off = (unit(k, 11, salt) - 0.5) * reach * 1.1;
                let sx = (x as f32 + off).round() as i32;
                // each stain leaves from inside the lower edge of the patch, so it runs out of
                // the rust rather than hanging under it
                let below = reach * 0.9 * (1.0 - (off / reach).abs() * 0.8);
                let long = unit(k, 13, salt);
                self.streaks.push(Streak {
                    x: sx,
                    top: y as f32 + below * 0.6,
                    length: (h as f32 * (0.12 + 0.35 * long * long)).max(3.0),
                    pace: 0.3 + unit(k, 17, salt) * 0.3,
                    phase: unit(k, 19, salt) * 30.0,
                    wide: long > 0.6 && w >= 60,
                });
            }
        }
    }

    /// How far rust has eaten into a pixel: below 0 clean, then the film, the crust and the core.
    fn rust_at(&self, x: i32, y: i32) -> f32 {
        let mut most = f32::MIN;
        let seam = matches!(
            self.parts[(y * self.wide() + x) as usize],
            Part::Seam | Part::Lip
        );
        for s in &self.sources {
            let grown = paint::ease(((self.t - s.start) / GROW_SECONDS).max(0.0));
            let r = s.reach * (0.15 + 0.85 * grown);
            // water runs down, so a patch reaches further below its rivet than above it
            let down = if y as f32 > s.y { 0.62 } else { 1.2 };
            let (dx, dy) = (x as f32 - s.x, (y as f32 - s.y) * down);
            // ragged edges come from noise; water lying in the seams carries the rust along them
            let ragged = 0.65 + 0.7 * noise(x as f32 / 6.0, y as f32 / 4.5, s.salt);
            let wick = if seam { 0.8 } else { 1.0 };
            let d = (dx * dx + dy * dy).sqrt() * ragged * wick;
            most = most.max(r - d);
        }
        most
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let (w, h) = (self.wide(), self.tall());
        let (_, ph) = self.plate();
        let mut mixer = Blender::new(6);
        for y in 0..h {
            for x in 0..w {
                let (col, row, _, ly) = self.on_plate(x, y);
                let part = self.parts[(y * w + x) as usize];
                let colour = match part {
                    Part::Plate => {
                        // lit from the upper left, each plate a touch apart from its neighbours,
                        // with faint rolling marks running across it
                        let down = ly as f32 / ph as f32;
                        let own = unit(col, row, 3) * 0.3;
                        c.steel.at(down * 0.7 + own)
                    }
                    Part::Seam => c.seam,
                    Part::Lip => c.lip,
                    Part::RivetLight => c.rivet.0,
                    Part::Rivet => c.rivet.1,
                    Part::RivetShade => c.rivet.2,
                    Part::RivetCast => c.cast,
                };
                canvas.set(x, y, colour);
            }
        }

        // the stains first, so the patches they run from sit on top of them
        for streak in &self.streaks {
            let length = self.streak_length(streak);
            let top = streak.top.round() as i32;
            for k in 0..length as i32 {
                let fade = 1.0 - k as f32 / streak.length;
                let amount = 0.15 + 0.75 * fade;
                mixer.blend(canvas, streak.x, top + k, c.stain, amount);
                // a heavy stain is wider near the patch and narrows as it runs
                if streak.wide && k < (length * 0.45) as i32 {
                    mixer.blend(canvas, streak.x + 1, top + k, c.stain, amount * 0.7);
                }
            }
        }

        for y in 0..h {
            for x in 0..w {
                let depth = self.rust_at(x, y);
                if depth < 0.0 {
                    continue;
                }
                let part = self.parts[(y * w + x) as usize];
                let grain = noise(x as f32 / 1.8, y as f32 / 1.8, 41);
                let colour = if depth < 1.7 {
                    c.film
                } else if depth < 2.7 {
                    // a dark crusted rim, so each patch has a clear edge
                    c.scale.0
                } else if depth < 7.5 {
                    // crusted scale: a rivet under it still shows its lit side and its shadow
                    match part {
                        Part::RivetLight => c.scale.2,
                        Part::RivetShade | Part::RivetCast | Part::Seam => c.scale.0,
                        _ if grain > 0.8 => c.scale.0,
                        _ if grain < 0.14 => c.scale.2,
                        _ => c.scale.1,
                    }
                } else {
                    // the blistered core, with lit flakes lifting off it, each with a dark
                    // shadow under it
                    let flake = noise(x as f32 / 1.8 + 3.0, y as f32 / 1.8, 59);
                    let under = noise(x as f32 / 1.8 + 3.0, (y - 1) as f32 / 1.8, 59);
                    if flake > 0.66 {
                        c.scale.2
                    } else if under > 0.66 {
                        c.core
                    } else if grain > 0.45 {
                        c.scale.1
                    } else {
                        c.core
                    }
                };
                canvas.set(x, y, colour);
            }
        }

        // a drop sliding down each stain once it has grown, then resting before the next
        for streak in &self.streaks {
            if let Some(y) = self.drop_row(streak) {
                let x = streak.x;
                if self.rust_at(x.clamp(0, w - 1), y.clamp(0, h - 1)) < 2.0 {
                    canvas.set(x, y, c.drop);
                    canvas.set(x, y - 1, c.drop);
                }
            }
        }
    }

    /// How far a stain has run so far.
    fn streak_length(&self, streak: &Streak) -> f32 {
        ((self.t - HEAD_START * 0.5) * streak.pace)
            .max(0.0)
            .min(streak.length)
    }

    /// The row of the drop on a stain, if one is sliding down it now.
    fn drop_row(&self, streak: &Streak) -> Option<i32> {
        let length = self.streak_length(streak);
        if length < streak.length * 0.5 {
            return None;
        }
        // the clock already runs at the motion setting's speed, so the drops follow it
        let travel = length / DROP_SPEED;
        let into = (self.t + streak.phase).rem_euclid(travel + DROP_REST);
        (into <= travel).then(|| (streak.top + into * DROP_SPEED).floor() as i32)
    }
}

impl Scene for Corrosion {
    fn name(&self) -> &'static str {
        "corrosion"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // the rust is laid out again for the new wall at the same age, so it keeps its extent
        self.lay_out();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += dt * self.motion.speed();
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        if self.parts.len() == canvas.width() * canvas.height() {
            self.paint(&mut canvas, &colours);
        }
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
        check_scene("corrosion");
        check_determinism("corrosion");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("corrosion", 40.0);
    }

    fn rusty(wall: &Corrosion) -> usize {
        let (w, h) = (wall.wide(), wall.tall());
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| wall.rust_at(x, y) >= 2.0)
            .count()
    }

    #[test]
    fn rust_creeps_slowly_then_holds_and_never_takes_the_whole_wall() {
        let mut rng = Rng::new(3);
        let mut wall = Corrosion::new(80, 24, Motion::FULL, &mut rng);
        let first = rusty(&wall);
        assert!(first > 0, "no rust at the start");
        for _ in 0..(30 * 20) {
            wall.step(0.05, &mut rng);
        }
        let later = rusty(&wall);
        assert!(later > first, "{first} then {later}");
        for _ in 0..(10 * 60 * 20) {
            wall.step(0.05, &mut rng);
        }
        let most = rusty(&wall);
        for _ in 0..(60 * 20) {
            wall.step(0.05, &mut rng);
        }
        assert_eq!(rusty(&wall), most, "the rust still spreads at its widest");
        let all = (wall.wide() * wall.tall()) as usize;
        assert!(most * 2 < all, "{most} of {all}");
        let frame = render(&mut wall, 80, 24, 1);
        let c = Colours::new(theme::default_theme());
        let painted = pixels(&frame);
        assert!(painted.contains(&c.core) && painted.contains(&c.scale.1));
    }

    #[test]
    fn the_edge_of_a_patch_creeps_well_under_a_pixel_a_second() {
        let mut rng = Rng::new(3);
        let wall = Corrosion::new(80, 24, Motion::LIVELY, &mut rng);
        let fastest = wall
            .sources
            .iter()
            .map(|s| s.reach * 0.85 * 1.5 / GROW_SECONDS)
            .fold(0.0, f32::max);
        assert!(fastest * Motion::LIVELY.speed() < 0.5, "{fastest}");
    }

    #[test]
    fn a_resize_keeps_the_age_of_the_rust() {
        let mut rng = Rng::new(4);
        let mut wall = Corrosion::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(40 * 20) {
            wall.step(0.05, &mut rng);
        }
        let t = wall.t;
        wall.resize(20, 8, &mut rng);
        wall.resize(80, 24, &mut rng);
        assert_eq!(wall.t, t);
        assert!(rusty(&wall) > 0);
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.steel.at(0.0),
                c.steel.at(1.0),
                0.05,
                "the lit and shaded steel",
            );
            apart(theme, c.seam, c.lip, 0.05, "the seam and its lip");
            apart(
                theme,
                c.film,
                c.steel.at(0.5),
                0.04,
                "the rust film and steel",
            );
            apart(theme, c.scale.1, c.film, 0.08, "the scale and the film");
            apart(theme, c.core, c.scale.2, 0.08, "the core and its flakes");
            for colour in c
                .steel
                .steps()
                .iter()
                .chain([&c.seam, &c.lip, &c.film, &c.stain, &c.cast])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.scale.0, c.scale.1, c.scale.2, c.core, c.drop] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
