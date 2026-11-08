//! A campfire in a clearing, painted in solid pixels: flames whose rows change shape one at a
//! time, each no more than once every 0.6 seconds, over crossed logs and a bed of coals inside a
//! ring of stones, with embers rising, pines standing round the clearing and a steady glow on the
//! ground and in the air. A dark theme shows it at night under a few stars; a light theme shows
//! it on a late afternoon, the sky pale and warm low down and the flame a bright yellow heart
//! inside a red outline.
//!
//! The sky, the trees, the ground and the glow are tints of the background held below the
//! non-text floor, so they read as background; the glow never changes, so it can never flash.
//! The flame's outline, the logs, the stones and the embers keep the theme's own drawing colours,
//! so they stand out as shapes and keep their outline when colour is off.

use super::paint;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The time between two shapes of one flame row. A row switches straight from one shape to the
/// next, with no blend that could cross a shade boundary several times on the way, so no flame
/// cell changes more than once in this time: under twice a second.
pub const FLICKER_PERIOD: f32 = 0.6;
/// The flame row period under reduced motion.
pub const CALM_FLICKER_PERIOD: f32 = 1.5;
const SKY_STEPS: usize = 8;
/// The hues a daylit flame is tinted towards, so its heart reads as fire rather than as a pale
/// stain of a theme colour chosen for text.
const FIRE_YELLOW: Rgb = Rgb::new(255, 205, 60);
const FIRE_ORANGE: Rgb = Rgb::new(245, 120, 30);
const GLOW_STEPS: usize = 8;

/// One row of the flame, a cell tall: where its centre leans, how wide it is and how long since
/// it changed. Both pixel rows of the cell follow it, so a cell changes only when its row does.
#[derive(Clone, Debug)]
struct FlameRow {
    offset: f32,
    factor: f32,
    since: f32,
}

#[derive(Clone, Debug)]
struct Ember {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
    life: f32,
}

/// The colours of the campfire in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct FireColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub star: Rgb,
    pub far_trees: Rgb,
    pub trees: Rgb,
    pub trees_lit: Rgb,
    pub trunk: Rgb,
    pub ground: Rgb,
    pub glow: Rgb,
    pub air_glow: Rgb,
    pub log: Rgb,
    pub log_dark: Rgb,
    pub log_end: Rgb,
    pub stone: Rgb,
    pub stone_lit: Rgb,
    pub stone_dark: Rgb,
    pub coal: Rgb,
    pub coal_hot: Rgb,
    pub core: Rgb,
    pub flame: Rgb,
    pub edge: Rgb,
    /// The embers as they rise and cool: the flame's own colours at night, its red outline by
    /// day, where a pale spark would vanish against the sky.
    pub spark_hot: Rgb,
    pub spark: Rgb,
    /// The ground from its own colour to the glow under the fire.
    ground_glow: Ramp,
    /// For each sky step, the air from the sky to its glow near the fire.
    air: Vec<Ramp>,
    /// The flame from its outer edge to its core.
    heat: Ramp,
}

impl FireColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dark = theme.dark;
        let ember_hue = mix(theme.yellow(), theme.red(), 0.5);
        // a clear night on a dark theme; a late afternoon on a light one, blue overhead and
        // warm towards the trees
        let (sky_top, sky_low) = if dark {
            (
                shades.tint(theme.info, 0.08, 1.15),
                shades.tint(theme.info, 0.25, 1.45),
            )
        } else {
            (
                shades.tint(theme.info, 0.4, 1.8),
                shades.tint(theme.warning, 0.25, 1.3),
            )
        };
        // stars only at night; by day the sky stays clear
        let star = if dark {
            shades.tint(theme.foreground, 0.4, 2.2)
        } else {
            sky_top
        };
        let (far_trees, trees, trees_lit, trunk, ground) = if dark {
            (
                shades.tint(mix(theme.green(), theme.info, 0.5), 0.12, 1.2),
                mix(theme.background, theme.green(), 0.06),
                shades.tint(mix(theme.green(), theme.warning, 0.5), 0.3, 1.7),
                shades.tint(ember_hue, 0.18, 1.3),
                shades.tint(mix(theme.green(), ember_hue, 0.6), 0.2, 1.35),
            )
        } else {
            (
                shades.tint(mix(theme.green(), theme.info, 0.5), 0.35, 1.6),
                shades.tint(theme.green(), 0.7, 2.6),
                shades.tint(mix(theme.green(), theme.warning, 0.4), 0.45, 1.9),
                shades.tint(mix(ember_hue, theme.foreground, 0.4), 0.6, 2.5),
                shades.tint(mix(theme.green(), theme.yellow(), 0.4), 0.45, 1.85),
            )
        };
        let glow = if dark {
            shades.tint(theme.warning, 0.8, IMAGERY_CEILING - 0.15)
        } else {
            shades.tint(theme.warning, 0.15, 1.2)
        };
        let air_glow = if dark {
            shades.tint(theme.warning, 0.4, 1.9)
        } else {
            shades.tint(theme.warning, 0.12, 1.15)
        };
        let sky = Ramp::new(sky_top, sky_low, SKY_STEPS);
        let air = sky
            .steps()
            .iter()
            .map(|&step| Ramp::new(step, air_glow, GLOW_STEPS))
            .collect();
        let edge = theme.red();
        let (core, flame) = if dark {
            (theme.bright_yellow(), theme.yellow())
        } else {
            // by day the heart of the flame is pale and bright, held as imagery inside the
            // solid red outline
            (
                shades.tint(FIRE_YELLOW, 1.0, 1.6),
                shades.tint(FIRE_ORANGE, 1.0, 2.6),
            )
        };
        // the logs are the flame's red burnt down towards the dark, held so they keep a shape
        let log = shades.held(
            mix(edge, theme.yellow(), 0.3),
            shades.darkest(),
            0.5,
            IMAGERY_CEILING + 0.4,
        );
        let log_dark = paint::stronger(theme, log, 0.3);
        let log_end = shades.held(theme.yellow(), shades.darkest(), 0.2, IMAGERY_CEILING + 0.8);
        let stone = paint::strong(theme, mix(theme.grey(), theme.muted, 0.5));
        let stone_lit = shades.held(
            stone,
            mix(theme.warning, shades.lightest(), 0.5),
            0.3,
            paint::SOLID,
        );
        let stone_dark = paint::stronger(theme, stone, 0.35);
        let coal = paint::strong(theme, mix(theme.red(), shades.darkest(), 0.15));
        let coal_hot = paint::strong(theme, theme.warning);
        Self {
            sky_top,
            sky_low,
            star,
            far_trees,
            trees,
            trees_lit,
            trunk,
            ground,
            glow,
            air_glow,
            log,
            log_dark,
            log_end,
            stone,
            stone_lit,
            stone_dark,
            coal,
            coal_hot,
            core,
            flame,
            edge,
            spark_hot: if dark { core } else { coal_hot },
            spark: if dark { flame } else { edge },
            ground_glow: Ramp::new(ground, glow, GLOW_STEPS),
            air,
            heat: Ramp::new(edge, flame, 3),
        }
    }

    /// The flame's colour at a heat from 0 at its edge to 1 at its core.
    fn flame_at(&self, heat: f32) -> Rgb {
        if heat >= 0.72 {
            self.core
        } else {
            self.heat.at(heat / 0.72)
        }
    }
}

/// A pine of the clearing: where it stands, how tall it is and whether it is one of the near
/// trees framing the fire or the far treeline.
#[derive(Clone, Copy, Debug)]
struct Pine {
    x: i32,
    height: i32,
    near: bool,
}

/// Paints a pine as a narrow triangle with a notch where each tier of branches starts, the side
/// facing the fire lit by it.
fn pine(
    canvas: &mut Canvas,
    x: i32,
    foot: i32,
    height: i32,
    body: Rgb,
    lit: Rgb,
    fire_right: bool,
) {
    let tiers = (height / 4).max(3);
    let widest = (height as f32 * 0.3).max(1.0);
    for i in 0..height {
        let notch = i32::from(i > 0 && i % tiers == 0);
        let spread = ((widest * i as f32 / height as f32).round() as i32 - notch).max(0);
        let y = foot - height + i;
        canvas.fill_rect(x - spread, y, 2 * spread + 1, 1, body);
        if spread >= 1 {
            let edge = if fire_right { x + spread } else { x - spread };
            canvas.set(edge, y, lit);
        }
    }
}

pub struct Campfire {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    rows: Vec<FlameRow>,
    shapes_rolled: u32,
    embers: Vec<Ember>,
    pines: Vec<Pine>,
}

impl Campfire {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut fire = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            rows: Vec::new(),
            shapes_rolled: 0,
            embers: Vec::new(),
            pines: Vec::new(),
        };
        fire.resize(width, height, rng);
        fire
    }

    fn flame_height(&self) -> usize {
        ((self.height as f32 * 0.42) as usize).clamp(2, 20)
    }

    fn base_width(&self) -> usize {
        let w = (self.width / 4).clamp(3, 31);
        if w % 2 == 0 { w - 1 } else { w }
    }

    fn period(&self) -> f32 {
        if self.motion.calm {
            CALM_FLICKER_PERIOD
        } else {
            FLICKER_PERIOD
        }
    }

    fn roll_row(row: &mut FlameRow, rng: &mut Rng) {
        row.offset = rng.range(-1.2, 1.2);
        row.factor = rng.range(0.7, 1.15);
    }

    /// The cell row the logs lie on, a little up from the bottom so the stones and the lit
    /// ground in front of the fire have room.
    fn ground(&self) -> i32 {
        let h = self.height as i32;
        ((h as f32 * 0.84) as i32).clamp(1, (h - 1).max(1))
    }

    /// The pixel row where the clearing meets the trees.
    fn horizon(&self) -> i32 {
        (self.height as f32 * 2.0 * 0.58) as i32
    }

    /// Paints the flame alone, without the logs, the glow or the embers.
    fn paint_flame(&self, canvas: &mut Canvas, c: &FireColours) {
        let centre = self.width as f32 / 2.0;
        let base = self.base_width() as f32;
        let pixels = self.rows.len() * 2;
        // the bottom pixel of the cell row just above the logs
        let bottom = self.ground() * 2 - 1;
        for j in 0..pixels {
            let row = &self.rows[j / 2];
            let rise = j as f32 / pixels as f32;
            let half = (base / 2.0) * row.factor;
            let row_centre = centre + row.offset * rise;
            let y = bottom - j as i32;
            // three pointed tongues over a rounded body, the middle one tallest, so the fire
            // splits into licks of flame rather than rising as one cone
            let lick = |u: f32, at: f32, width: f32, height: f32| {
                height * (1.0 - ((u - at).abs() / width).min(1.0)).powf(0.8)
            };
            let top = |u: f32| {
                lick(u, 0.0, 0.62, 1.0)
                    .max(lick(u, -0.5, 0.42, 0.6 + 0.12 * row.factor))
                    .max(lick(u, 0.52, 0.4, 0.5 + 0.1 * row.offset.abs()))
                    .max(0.32 * (1.0 - u * u))
            };
            let left = (row_centre - half).floor() as i32;
            let right = (row_centre + half).ceil() as i32;
            let inside = |x: i32| {
                let u = (x as f32 - row_centre) / half.max(0.5);
                u.abs() <= 1.0 && rise <= top(u)
            };
            for x in left..=right {
                if !inside(x) {
                    continue;
                }
                let u = (x as f32 - row_centre) / half.max(0.5);
                // the outline: the tip of a tongue or a side with nothing beside it
                let rim = !inside(x - 1) || !inside(x + 1) || rise >= top(u) - 0.1;
                let heat = (1.0 - u.abs() * 1.3).max(0.0) * (1.0 - rise * 0.6) + 0.15;
                let colour = if rim { c.edge } else { c.flame_at(heat) };
                canvas.set(x, y, colour);
            }
        }
    }

    /// Draws the flame alone into a frame, for the flicker test.
    #[cfg(test)]
    fn draw_flame(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = FireColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint_flame(&mut canvas, &colours);
        canvas.blit(frame);
    }

    fn paint(&self, canvas: &mut Canvas, c: &FireColours) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let ground = self.ground() * 2;
        let horizon = self.horizon().min(ground - 1);
        let centre = self.width as f32 / 2.0;
        let cx = centre as i32;
        let base = self.base_width() as f32;
        let reach = base * 1.6 + 4.0;
        let flame_top = ground as f32 - self.rows.len() as f32 * 2.0;

        // the sky, warmed round the fire by a still glow that is strongest at the flame
        for y in 0..horizon.min(tall) {
            let step = ((y as f32 / horizon.max(1) as f32) * (SKY_STEPS - 1) as f32).round();
            let air = &c.air[(step as usize).min(SKY_STEPS - 1)];
            let dy = ((y as f32 - (flame_top + ground as f32) / 2.0) * 1.2).abs();
            for x in 0..wide {
                let dx = x as f32 - centre;
                let d = (dx * dx + dy * dy).sqrt() / reach;
                canvas.set(x, y, air.at((1.0 - d).max(0.0).powf(1.4)));
            }
        }
        // a few still stars high in a night sky, kept clear of the glow
        if c.star != c.sky_top && self.height >= 8 {
            for y in 0..horizon / 2 {
                for x in 0..wide {
                    let near_fire = (x as f32 - centre).abs() < reach * 0.8;
                    if !near_fire && paint::hash(x, y, 31) % 97 == 0 {
                        canvas.set(x, y, c.star);
                    }
                }
            }
        }

        // the far treeline along the edge of the clearing
        for p in self.pines.iter().filter(|p| !p.near) {
            pine(
                canvas,
                p.x,
                horizon + 1,
                p.height,
                c.far_trees,
                c.far_trees,
                true,
            );
        }
        // the clearing, lit in a wide pool round the fire
        for y in horizon..tall {
            let dy = (y - ground) as f32 * 3.2;
            for x in 0..wide {
                let dx = x as f32 - centre;
                let d = ((dx * dx + dy * dy).sqrt() / (reach * 1.3)).min(1.0);
                canvas.set(x, y, c.ground_glow.at((1.0 - d).powf(1.3)));
            }
        }
        // the near pines framing the fire, lit on the side that faces it
        for p in self.pines.iter().filter(|p| p.near) {
            let foot = horizon + 2 + (p.height / 6);
            canvas.fill_rect(p.x, foot - 1, 1, 3, c.trunk);
            pine(
                canvas,
                p.x,
                foot - 1,
                p.height,
                c.trees,
                c.trees_lit,
                p.x < cx,
            );
        }

        let half = (base / 2.0 + 1.5) as i32;
        let ring = half + 2;
        if ground + 1 < tall && self.width >= 12 {
            // the back of the stone ring, behind the logs
            self.stones(canvas, c, cx, ground, ring, true);
        }

        // the bed of coals the logs lie in, its hottest coals in the middle
        if ground < tall {
            for x in cx - half..=cx + half {
                let hot = (x - cx).abs() < half / 2 && paint::hash(x, ground, 5) % 3 != 0;
                canvas.set(x, ground, if hot { c.coal_hot } else { c.coal });
            }
        }

        self.paint_flame(canvas, c);

        // a log laid across the coals and three more leant together over the fire, bark on
        // top and shade below, their cut ends lighter
        if ground < tall {
            canvas.fill_rect(cx - half, ground - 1, 2 * half + 1, 1, c.log);
            canvas.fill_rect(cx - half, ground, 2 * half + 1, 1, c.log_dark);
            for end in [cx - half, cx + half] {
                canvas.set(end, ground - 1, c.log_end);
                canvas.set(end, ground, c.log_end);
            }
            let apex = (cx, ground - 1 - (self.rows.len() as i32 * 2 / 3).max(2));
            let feet = [cx - half + 1, cx - half / 3, cx + half - 1];
            for (i, &foot) in feet.iter().enumerate() {
                let colour = if i == 1 { c.log_dark } else { c.log };
                canvas.line((foot, ground - 1), apex, colour);
            }
            canvas.set(apex.0, apex.1, c.log_dark);
        }

        if ground + 1 < tall && self.width >= 12 {
            // the front of the stone ring, over the logs' feet
            self.stones(canvas, c, cx, ground, ring, false);
        }

        for ember in &self.embers {
            let life = ember.age / ember.life;
            let colour = if life < 0.4 {
                c.spark_hot
            } else if life < 0.75 {
                c.spark
            } else {
                c.edge
            };
            canvas.set(
                ember.x.round() as i32,
                (ember.y * 2.0).round() as i32,
                colour,
            );
        }
    }

    /// One half of the ring of stones round the fire: the back half sits a pixel above the
    /// coals, the front half a pixel below, each stone lit on top and shaded underneath.
    fn stones(
        &self,
        canvas: &mut Canvas,
        c: &FireColours,
        cx: i32,
        ground: i32,
        ring: i32,
        back: bool,
    ) {
        let mut x = cx - ring - 1;
        while x < cx + ring {
            let across = ((x + 1 - cx) as f32 / ring as f32).clamp(-1.0, 1.0);
            let depth = (1.0 - across * across).sqrt();
            let end = depth < 0.6;
            // the back half shows only at the two ends, the rest hidden behind the fire
            if back && !end {
                x += 4;
                continue;
            }
            let lift = (depth * 1.6).round() as i32;
            let y = if back {
                ground - 1 - lift
            } else {
                ground + lift
            };
            // a rounded stone three pixels wide: lit on top, shaded on its lower right
            canvas.fill_rect(x, y, 3, 1, c.stone_lit);
            canvas.fill_rect(x, y + 1, 3, 1, c.stone);
            canvas.set(x + 2, y + 1, c.stone_dark);
            canvas.set(x + 2, y, c.stone);
            x += 4;
        }
    }

    #[cfg(test)]
    pub(crate) fn shapes_rolled(&self) -> u32 {
        self.shapes_rolled
    }
}

impl Scene for Campfire {
    fn name(&self) -> &'static str {
        "campfire"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // the rows already burning keep their shape and their place in the cycle; only rows the
        // new size adds are rolled, each starting at its own point so they never change together
        let wanted = self.flame_height();
        let period = self.period();
        self.rows.truncate(wanted);
        while self.rows.len() < wanted {
            let mut row = FlameRow {
                offset: 0.0,
                factor: 1.0,
                since: rng.range(0.0, period),
            };
            Self::roll_row(&mut row, rng);
            self.rows.push(row);
        }
        let (w, h) = (self.width as f32, self.height as f32);
        self.embers.retain(|e| e.x >= 0.0 && e.x < w && e.y < h);
        // the far treeline closes the clearing; a few tall pines stand near on each side
        let horizon = self.horizon() as f32;
        self.pines.clear();
        let mut x = rng.range(0.0, 3.0);
        while x < w {
            let height = (horizon * rng.range(0.16, 0.3)).round() as i32;
            self.pines.push(Pine {
                x: x as i32,
                height: height.max(3),
                near: false,
            });
            x += rng.range(2.5, 5.0);
        }
        if self.width >= 24 && self.height >= 8 {
            for side in [0.0, 1.0] {
                let count = 1 + usize::from(self.width >= 60) + usize::from(self.width >= 140);
                for i in 0..count {
                    let along = (i as f32 + rng.range(0.2, 0.7)) * w * 0.07;
                    let x = if side == 0.0 { along } else { w - 1.0 - along };
                    let height = (horizon * rng.range(0.7, 0.95)).round() as i32;
                    self.pines.push(Pine {
                        x: x as i32,
                        height: height.max(4),
                        near: true,
                    });
                }
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let period = self.period();
        for row in &mut self.rows {
            row.since += dt;
            if row.since >= period {
                row.since = (row.since - period).min(period);
                Self::roll_row(row, rng);
                self.shapes_rolled += 1;
            }
        }

        let speed = self.motion.speed();
        let rate = if self.motion.calm { 0.7 } else { 2.0 };
        let centre = self.width as f32 / 2.0;
        let top = (self.ground() - self.flame_height() as i32) as f32;
        if self.embers.len() < 24 && rng.chance(rate * dt) {
            let spread = self.base_width() as f32 / 4.0;
            self.embers.push(Ember {
                x: centre + rng.range(-spread, spread),
                y: top + 0.5,
                vx: rng.range(-0.5, 0.5) * speed,
                vy: -rng.range(1.0, 2.5) * speed,
                age: 0.0,
                life: rng.range(1.5, 3.0),
            });
        }
        for ember in &mut self.embers {
            ember.x += ember.vx * dt;
            ember.y += ember.vy * dt;
            ember.age += dt;
        }
        self.embers.retain(|e| e.age < e.life && e.y >= -1.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = FireColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::distance;
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, max_changes_per_second, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("campfire");
        check_determinism("campfire");
        check_pixel_scene("campfire", 6.0);
    }

    #[test]
    fn the_night_glow_and_fire_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = FireColours::new(theme);
            let name = theme.name;
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky's top and bottom"),
                (c.ground, c.sky_low, "the ground and the sky"),
                (c.glow, c.ground, "the glow and the ground"),
                (c.air_glow, c.sky_low, "the warm air and the sky"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} collapse in {name}");
            }
            for colour in [
                c.sky_top,
                c.sky_low,
                c.ground,
                c.glow,
                c.air_glow,
                c.far_trees,
                c.trees,
                c.trees_lit,
                c.trunk,
                c.star,
            ] {
                assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
            }
            // the flame's outline always keeps the floor; at night its body does too, while by
            // day its bright heart is imagery inside that outline
            let solid_flame: &[Rgb] = if theme.dark {
                &[c.core, c.flame, c.edge]
            } else {
                &[c.edge]
            };
            for &colour in solid_flame {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                assert!(distance(colour, c.glow) >= 0.08, "{name}");
            }
            if theme.dark {
                for heat in [0.0, 0.3, 0.6, 0.9] {
                    assert!(
                        theme.contrast_of(c.flame_at(heat)) >= IMAGERY_CEILING,
                        "{name}"
                    );
                }
            } else {
                for colour in [c.core, c.flame] {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                    assert!(distance(colour, c.sky_low) >= 0.08, "{name}");
                    assert!(distance(colour, c.edge) >= 0.08, "{name}");
                }
            }
            for colour in [
                c.log,
                c.log_end,
                c.log_dark,
                c.stone,
                c.stone_lit,
                c.stone_dark,
                c.coal,
                c.coal_hot,
            ] {
                assert!(theme.contrast_of(colour) >= IMAGERY_CEILING, "{name}");
                assert!(distance(colour, c.glow) >= 0.05, "{name}");
            }
        }
    }

    #[test]
    fn the_fire_sits_on_its_logs_and_sends_up_embers() {
        let c = FireColours::new(theme::default_theme());
        let mut rng = Rng::new(17);
        let mut fire = Campfire::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut fire, 60, 20, 60);
        let row = fire.ground() - 1;
        let logs: Vec<Rgb> = (0..60)
            .flat_map(|x| [row, row + 1].map(|y| frame.get(x, y).unwrap().pixel_pair().unwrap()))
            .flat_map(|(a, b)| [a, b])
            .collect();
        assert!(logs.contains(&c.log) && logs.contains(&c.log_end));
        let all = pixels(&frame);
        assert!(all.contains(&c.core) && all.contains(&c.edge));
        let mut embers_seen = false;
        for _ in 0..100 {
            let frame = render(&mut fire, 60, 20, 1);
            let top = fire.ground() - fire.flame_height() as i32;
            embers_seen |= (0..=top).any(|y| {
                (0..60).any(|x| {
                    let (a, b) = frame.get(x, y).unwrap().pixel_pair().unwrap();
                    [a, b].contains(&c.core)
                })
            });
        }
        assert!(embers_seen);
    }

    #[test]
    fn no_flame_cell_changes_more_than_twice_a_second() {
        for (motion, fps, limit) in [(Motion::FULL, 30, 2), (Motion::CALM, 12, 1)] {
            for &(width, height) in &[(20, 8), (80, 24), (120, 40)] {
                let mut rng = Rng::new(1);
                let mut fire = Campfire::new(width, height, motion, &mut rng);
                let theme = crate::theme::default_theme();
                let frames: Vec<Frame> = (0..(30 * fps))
                    .map(|_| {
                        fire.step(1.0 / fps as f32, &mut rng);
                        let mut frame = Frame::new(width, height);
                        fire.draw_flame(&mut frame, theme);
                        frame
                    })
                    .collect();
                let most = max_changes_per_second(&frames, fps as usize);
                assert!(
                    most <= limit,
                    "{most} changes a second at {width}x{height} ({motion:?})"
                );
            }
        }
    }

    #[test]
    fn each_row_takes_a_new_shape_once_a_period() {
        let mut rng = Rng::new(1);
        let mut fire = Campfire::new(40, 12, Motion::FULL, &mut rng);
        let rows = fire.rows.len() as f32;
        for _ in 0..(10 * 20) {
            fire.step(0.05, &mut rng);
        }
        let per_row = fire.shapes_rolled() as f32 / rows;
        assert!((15.0..=17.0).contains(&per_row), "{per_row}");
        assert!(fire.period() >= 0.5);
        let calm = Campfire::new(40, 12, Motion::CALM, &mut rng);
        assert!(calm.period() >= 1.0);
    }

    #[test]
    fn a_resize_keeps_the_flame_rows_it_still_has() {
        let mut rng = Rng::new(5);
        let mut fire = Campfire::new(80, 24, Motion::FULL, &mut rng);
        let before: Vec<f32> = fire.rows.iter().map(|r| r.offset).collect();
        fire.resize(81, 24, &mut rng);
        let after: Vec<f32> = fire.rows.iter().map(|r| r.offset).collect();
        assert_eq!(before, after);
    }
}
