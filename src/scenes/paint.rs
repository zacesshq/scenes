//! Painting shared by the pixel scenes: hashes and smooth noise for texture that stays put,
//! colour moves made from the theme, a blender that keeps the number of colours small, filled
//! shapes, sprites, skies and clouds, an overcast sky with slanting rain, bare trees and text
//! labels.
//!
//! Background imagery is made of tints held below the non-text floor. Anything that carries
//! meaning (a sign, a crack, a red lamp) is drawn solid, at or above the floor that keeps its
//! outline when colour is turned off, so its shape carries the meaning as well as its colour.
//!
//! The checks every pixel scene is held to live in `testing`, beside the other scene checks.

use std::collections::HashMap;

use super::wrap;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::theme::ThemeData;

/// The contrast a solid shape keeps against the background, a little over the ceiling so a
/// shape never sits right on the line between showing and vanishing when colour is off.
pub(crate) const SOLID: f64 = IMAGERY_CEILING + 0.2;

// noise

/// A well mixed hash of a pixel position, so texture is fixed to the place it is drawn and never
/// shimmers from one frame to the next.
pub(crate) fn hash(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1)
        ^ (y as u32).wrapping_mul(0x85eb_ca77)
        ^ salt.wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

/// The hash as a value in `[0, 1)`.
pub(crate) fn unit(x: i32, y: i32, salt: u32) -> f32 {
    (hash(x, y, salt) >> 8) as f32 / (1u32 << 24) as f32
}

/// Eases from 0 to 1 and holds, so a slow change slows again as it finishes.
pub(crate) fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Smooth value noise in `[0, 1]`: neighbouring points are close, so drifting it moves soft
/// shapes rather than speckle.
pub(crate) fn noise(x: f32, y: f32, salt: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (sx, sy) = (ease(x - x0), ease(y - y0));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let top = unit(ix, iy, salt) + (unit(ix + 1, iy, salt) - unit(ix, iy, salt)) * sx;
    let bottom =
        unit(ix, iy + 1, salt) + (unit(ix + 1, iy + 1, salt) - unit(ix, iy + 1, salt)) * sx;
    top + (bottom - top) * sy
}

/// Two octaves of noise, for billows with some fine texture on them.
pub(crate) fn billow(x: f32, y: f32, seed: u32) -> f32 {
    0.68 * noise(x, y, seed) + 0.32 * noise(x * 2.3 + 17.0, y * 2.1, !seed)
}

// colour moves

/// A background tint: `strength` of the way from the background towards `hue`, held below
/// `ceiling` contrast so it stays imagery.
pub(crate) fn tint(theme: &ThemeData, hue: Rgb, strength: f32, ceiling: f64) -> Rgb {
    Shades::new(theme).tint(hue, strength, ceiling)
}

/// `from` moved `t` of the way towards `to`, stopping short of the ceiling against `background`,
/// so a reflection or a glow over imagery stays imagery however strong the colour it borrows.
/// A `from` already at or above the ceiling comes back unchanged.
pub(crate) fn held_imagery(background: Rgb, from: Rgb, to: Rgb, t: f32) -> Rgb {
    let below = |c: Rgb| c.contrast(background) < IMAGERY_CEILING;
    let wanted = mix(from, to, t);
    if below(wanted) || !below(from) {
        return if below(from) { wanted } else { from };
    }
    let (mut low, mut high) = (0.0_f32, t.clamp(0.0, 1.0));
    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        if below(mix(from, to, mid)) {
            low = mid;
        } else {
            high = mid;
        }
    }
    mix(from, to, low)
}

/// The theme's colour furthest from the background: its lightest on a dark theme, its darkest
/// on a light one.
fn far_end(theme: &ThemeData) -> Rgb {
    let shades = Shades::new(theme);
    if theme.dark {
        shades.lightest()
    } else {
        shades.darkest()
    }
}

/// `colour` moved `t` of the way away from the background: lighter on a dark theme, darker on a
/// light one. A shape shaded this way only gains contrast, so its lit side never drops below
/// the floor its base colour keeps.
pub(crate) fn stronger(theme: &ThemeData, colour: Rgb, t: f32) -> Rgb {
    let moved = mix(colour, far_end(theme), t);
    // a colour already past the theme's text colour, such as pure white on an off-white text
    // theme, would lose contrast, so it stays as it is
    if theme.contrast_of(moved) < theme.contrast_of(colour) {
        colour
    } else {
        moved
    }
}

/// `colour` moved `t` of the way towards `toward`, held at or above the solid contrast so the
/// shape keeps its outline with colour off. A colour already under it is returned as it is.
pub(crate) fn solid(theme: &ThemeData, colour: Rgb, toward: Rgb, t: f32) -> Rgb {
    let at = |t: f32| mix(colour, toward, t);
    let wanted = at(t);
    if theme.contrast_of(wanted) >= SOLID || theme.contrast_of(colour) < SOLID {
        return wanted;
    }
    let (mut low, mut high) = (0.0_f32, t.clamp(0.0, 1.0));
    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        if theme.contrast_of(at(mid)) >= SOLID {
            low = mid;
        } else {
            high = mid;
        }
    }
    at(low)
}

/// `colour` pushed away from the background, if it needs it, until it reaches the solid contrast.
pub(crate) fn strong(theme: &ThemeData, colour: Rgb) -> Rgb {
    if theme.contrast_of(colour) >= SOLID {
        return colour;
    }
    let far = far_end(theme);
    (1..=20)
        .map(|i| mix(colour, far, i as f32 / 20.0))
        .find(|&c| theme.contrast_of(c) >= SOLID)
        .unwrap_or(far)
}

/// A solid shade and a solid highlight of a shape's colour, for shading a rounded or lit form.
pub(crate) fn shaded(theme: &ThemeData, colour: Rgb) -> (Rgb, Rgb, Rgb) {
    let shades = Shades::new(theme);
    let dark = solid(theme, colour, shades.darkest(), 0.35);
    let light = solid(theme, colour, shades.lightest(), 0.35);
    (dark, colour, light)
}

/// Mixes colours in a fixed number of steps and remembers each result. Quantising the amount
/// keeps the number of colours small, which keeps the escape output small. Remembering each
/// result keeps a large frame quick to paint, since imagery uses few colours and most mixes
/// repeat.
pub(crate) struct Blender {
    levels: f32,
    cache: HashMap<(Rgb, Rgb, u8), Rgb>,
}

impl Blender {
    pub(crate) fn new(levels: u8) -> Self {
        Self {
            levels: f32::from(levels.max(1)),
            cache: HashMap::new(),
        }
    }

    /// `a` moved `t` of the way towards `b`, `t` rounded to the nearest step.
    pub(crate) fn mix(&mut self, a: Rgb, b: Rgb, t: f32) -> Rgb {
        let step = (t.clamp(0.0, 1.0) * self.levels).round() as u8;
        if step == 0 {
            return a;
        }
        let levels = self.levels;
        *self
            .cache
            .entry((a, b, step))
            .or_insert_with(|| mix(a, b, f32::from(step) / levels))
    }

    /// Mixes `colour` over one pixel of the canvas.
    pub(crate) fn blend(&mut self, canvas: &mut Canvas, x: i32, y: i32, colour: Rgb, t: f32) {
        if let Some(under) = canvas.get(x, y) {
            let mixed = self.mix(under, colour, t);
            canvas.set(x, y, mixed);
        }
    }
}

// shapes and sprites

/// Fills every pixel whose centre lies inside an ellipse.
pub(crate) fn ellipse(canvas: &mut Canvas, cx: f32, cy: f32, rx: f32, ry: f32, colour: Rgb) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let (x0, x1) = ((cx - rx).floor() as i32, (cx + rx).ceil() as i32);
    let (y0, y1) = ((cy - ry).floor() as i32, (cy + ry).ceil() as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                canvas.set(x, y, colour);
            }
        }
    }
}

/// Fills a disc of radius `r` around a point. The edge is eased out by a third of a pixel, so a
/// small disc is round rather than a cross with single pixels sticking out.
pub(crate) fn disc(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, colour: Rgb) {
    let reach = r.ceil() as i32 + 1;
    let (px, py) = (cx.round() as i32, cy.round() as i32);
    let edge = r + 0.33;
    for y in py - reach..=py + reach {
        for x in px - reach..=px + reach {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            if dx * dx + dy * dy <= edge * edge {
                canvas.set(x, y, colour);
            }
        }
    }
}

/// Draws a line of pixels from one point to another, `thick` pixels wide across its run. A one
/// pixel line between whole pixels is `Canvas::line`.
pub(crate) fn line(canvas: &mut Canvas, from: (f32, f32), to: (f32, f32), thick: i32, colour: Rgb) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    let across_x = dy.abs() >= dx.abs();
    for i in 0..=steps {
        let s = i as f32 / steps as f32;
        let (x, y) = (
            (from.0 + dx * s).round() as i32,
            (from.1 + dy * s).round() as i32,
        );
        for k in 0..thick.max(1) {
            if across_x {
                canvas.set(x + k, y, colour);
            } else {
                canvas.set(x, y + k, colour);
            }
        }
    }
}

/// Mirrors a sprite left to right.
pub(crate) fn mirrored(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.chars().rev().collect()).collect()
}

/// Draws a sprite facing right as given or mirrored to face left.
pub(crate) fn sprite_facing(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    rows: &[&str],
    right: bool,
    palette: &[(char, Rgb)],
) {
    if right {
        canvas.sprite(x, y, rows, palette);
    } else {
        let flipped = mirrored(rows);
        let flipped: Vec<&str> = flipped.iter().map(String::as_str).collect();
        canvas.sprite(x, y, &flipped, palette);
    }
}

/// Fills rows `top` to `bottom` (exclusive) with a vertical gradient along `ramp`.
pub(crate) fn gradient(canvas: &mut Canvas, ramp: &Ramp, top: i32, bottom: i32) {
    let span = (bottom - top - 1).max(1) as f32;
    for y in top.max(0)..bottom.min(canvas.height() as i32) {
        canvas.fill_row(y, ramp.at((y - top) as f32 / span));
    }
}

// fair weather: ridges and clouds

/// A smooth wavy line from a few sine waves with seeded phases, in `[-1, 1]`, for ridges,
/// banks and shorelines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Wave {
    phases: [f32; 3],
    scale: f32,
}

impl Wave {
    /// `scale` is about how many pixels one rise and fall takes.
    pub(crate) fn new(rng: &mut Rng, scale: f32) -> Self {
        Self {
            phases: [
                rng.range(0.0, std::f32::consts::TAU),
                rng.range(0.0, std::f32::consts::TAU),
                rng.range(0.0, std::f32::consts::TAU),
            ],
            scale: scale.max(1.0),
        }
    }

    pub(crate) fn at(&self, x: f32) -> f32 {
        let u = x / self.scale;
        (0.55 * (u + self.phases[0]).sin()
            + 0.3 * (u * 2.3 + self.phases[1]).sin()
            + 0.15 * (u * 5.1 + self.phases[2]).sin())
        .clamp(-1.0, 1.0)
    }
}

/// A cloud drifting across the sky: puffs along a flat base.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cloud {
    pub x: f32,
    /// The pixel row of the cloud's flat base.
    pub base: f32,
    pub width: f32,
    /// Pixels per second, already scaled by the motion setting.
    pub speed: f32,
    pub seed: u32,
}

impl Cloud {
    pub(crate) fn new(x: f32, base: f32, width: f32, speed: f32, seed: u32) -> Self {
        Self {
            x,
            base,
            width: width.max(4.0),
            speed,
            seed,
        }
    }

    /// A cloud of a random width in `widths`, with its base somewhere in `bases`, placed
    /// anywhere across a canvas `canvas_width` wide and drifting at `speed` times a gentle pace.
    pub(crate) fn random(
        rng: &mut Rng,
        canvas_width: f32,
        bases: (f32, f32),
        widths: (f32, f32),
        speed: f32,
    ) -> Self {
        let width = rng.range(widths.0, widths.1.max(widths.0));
        let x = rng.range(-width, canvas_width.max(1.0));
        let base = rng.range(bases.0, bases.1.max(bases.0));
        let pace = rng.range(0.4, 0.9) * speed;
        Self::new(x, base, width, pace, rng.next_u64() as u32)
    }

    /// Moves the cloud on, bringing it back in at the left once it has left on the right.
    pub(crate) fn drift(&mut self, dt: f32, canvas_width: f32) {
        let span = canvas_width + 2.0 * self.width;
        self.x = wrap(self.x + self.speed * dt + self.width, span) - self.width;
    }

    /// Paints the cloud: a lighter body with a shaded underside, so it reads as a solid shape
    /// rather than a smudge.
    pub(crate) fn paint(&self, canvas: &mut Canvas, body: Rgb, shade: Rgb) {
        let puffs = ((self.width / 4.0).round() as i32).max(3);
        let step = self.width / puffs as f32;
        // neighbouring puffs overlap, so the cloud is one body rather than a row of beads
        let base_r = (step * 0.65).max(1.5);
        let shade_from = self.base - (base_r * 0.6).max(1.0);
        let put = |canvas: &mut Canvas, x: i32, y: i32| {
            let low = y as f32 + 0.5 > shade_from;
            canvas.set(x, y, if low { shade } else { body });
        };
        for i in 0..puffs {
            let h = unit(i, 0, self.seed);
            // the middle puffs are tallest, so the cloud has a crown
            let middle = 1.0 - ((i as f32 + 0.5) / puffs as f32 - 0.5).abs() * 2.0;
            let r = base_r * (1.0 + 0.5 * middle.max(0.0)) * (0.85 + 0.3 * h);
            let cx = self.x + step * (i as f32 + 0.5);
            let cy = self.base - r * 0.75;
            let (x0, x1) = ((cx - r).floor() as i32, (cx + r).ceil() as i32);
            let (y0, y1) = ((cy - r).floor() as i32, self.base.floor() as i32);
            for y in y0..y1 {
                for x in x0..=x1 {
                    let dx = x as f32 + 0.5 - cx;
                    let dy = y as f32 + 0.5 - cy;
                    if dx * dx + dy * dy <= r * r {
                        put(canvas, x, y);
                    }
                }
            }
        }
        // a flat underside joins the puffs along the base
        let (left, right) = (self.x + step * 0.4, self.x + self.width - step * 0.4);
        for y in (self.base - base_r).floor() as i32..self.base.floor() as i32 {
            for x in left.round() as i32..right.round() as i32 {
                put(canvas, x, y);
            }
        }
    }
}

/// The body and underside of a cloud in a theme: lighter than the sky on a dark theme, near
/// the background on a light one, with the underside a step towards the sky.
pub(crate) fn cloud_tones(theme: &ThemeData, ceiling: f64) -> (Rgb, Rgb) {
    let shades = Shades::new(theme);
    if theme.dark {
        (
            shades.tint(theme.bright(), 0.4, ceiling),
            shades.tint(theme.muted, 0.24, ceiling - 0.6),
        )
    } else {
        (
            shades.tint(theme.info, 0.03, 1.08),
            shades.tint(theme.muted, 0.22, 1.5),
        )
    }
}

// foul weather: an overcast sky, banks of cloud and rain

/// The colours of a heavy, overcast sky and its rain in one theme.
#[derive(Clone, Debug)]
pub(crate) struct Overcast {
    pub sky: Ramp,
    pub cloud: Ramp,
    pub rain_far: Rgb,
    pub rain_near: Rgb,
}

impl Overcast {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let grey = mix(theme.muted, theme.info, 0.25);
        // a dark theme's storm sky lightens towards the horizon under brighter cloud; a light
        // theme's is a pale grey under darker cloud, so cloud reads as weather in both
        let (top, low, shadow, light) = if theme.dark {
            (
                shades.tint(grey, 0.12, 1.25),
                shades.tint(grey, 0.3, 1.6),
                shades.tint(grey, 0.32, 1.75),
                shades.tint(grey, 0.62, 2.6),
            )
        } else {
            (
                shades.tint(grey, 0.42, 1.9),
                shades.tint(grey, 0.18, 1.3),
                shades.tint(grey, 0.75, 2.75),
                shades.tint(grey, 0.4, 1.7),
            )
        };
        let rain = mix(theme.info, theme.foreground, 0.35);
        Self {
            sky: Ramp::new(top, low, 7),
            cloud: Ramp::new(shadow, light, 6),
            rain_far: shades.tint(rain, 0.4, 1.8),
            rain_near: shades.tint(rain, 0.7, 2.5),
        }
    }
}

/// A bank of cloud across rows `top` to `bottom`, built from drifting noise. Each puff is lit
/// from above, so its top is the light end of the cloud ramp and its underside the dark end.
/// `cover` from 0 to 1 sets how much of the band is cloud.
#[allow(clippy::too_many_arguments)]
pub(crate) fn clouds(
    canvas: &mut Canvas,
    blender: &mut Blender,
    ramp: &Ramp,
    top: i32,
    bottom: i32,
    drift: f32,
    seed: u32,
    cover: f32,
) {
    let rows = (bottom - top).max(1) as f32;
    let scale = 15.0;
    for y in top.max(0)..bottom.min(canvas.height() as i32) {
        let v = (y - top) as f32 / rows;
        // thinner towards both edges of the band, so it has no hard top or bottom
        let edge = (v * 3.0).min((1.0 - v) * 2.5).clamp(0.0, 1.0);
        for x in 0..canvas.width() as i32 {
            let n = billow((x as f32 + drift) / scale, y as f32 / (scale * 0.4), seed);
            let density = ((n - (1.0 - cover)) / 0.14).clamp(0.0, 1.0) * edge;
            if density <= 0.05 {
                continue;
            }
            // lit from above: the top of the bank and the thickest billows are lightest and
            // the underside is darkest
            let light = (1.1 - v * 1.3 + (n - 0.5) * 1.6).clamp(0.0, 1.0);
            blender.blend(canvas, x, y, ramp.at(light), density);
        }
    }
}

/// A raindrop: where it is in pixels, how fast it falls and whether it is near, which draws it
/// longer and brighter.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Drop {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
    pub near: bool,
}

/// Draws each drop as a short streak leaning with the wind, `slant` pixels across for each
/// pixel down. A drop shows only above the row `floor` gives for its column.
pub(crate) fn rain(
    canvas: &mut Canvas,
    blender: &mut Blender,
    drops: &[Drop],
    overcast: &Overcast,
    slant: f32,
    floor: impl Fn(i32) -> i32,
) {
    for drop in drops {
        let (length, colour, amount) = if drop.near {
            (3, overcast.rain_near, 0.8)
        } else {
            (2, overcast.rain_far, 0.6)
        };
        for k in 0..length {
            let y = drop.y.floor() as i32 - k;
            let x = (drop.x - slant * k as f32).round() as i32;
            if y >= 0 && y < floor(x) {
                // the tail fades, so a streak reads as a falling drop
                let fade = 1.0 - k as f32 / (length as f32 + 1.0);
                blender.blend(canvas, x, y, colour, amount * fade);
            }
        }
    }
}

// bare trees

// signs and labels

/// Writes a label as text after the canvas, so it keeps the theme's text floor on the plain
/// background around it.
pub(crate) fn label(frame: &mut Frame, x: i32, y: i32, text: &str, colour: Rgb) {
    let width = frame.width() as i32;
    let len = text.chars().count() as i32;
    if y < 0 || y >= frame.height() as i32 || len > width {
        return;
    }
    frame.put_str(x.clamp(0, width - len), y, text, Some(colour));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    #[test]
    fn noise_is_smooth_fixed_and_in_range() {
        for i in 0..400 {
            let x = i as f32 * 0.05;
            let v = noise(x, 1.3, 4);
            assert!((0.0..=1.0).contains(&v));
            assert_eq!(v, noise(x, 1.3, 4));
            assert!((noise(x + 0.01, 1.3, 4) - v).abs() < 0.05);
            assert!((0.0..1.0).contains(&unit(i, 1, 2)));
        }
        assert_ne!(hash(1, 2, 3), hash(2, 1, 3));
        assert!((0.0..=1.0).contains(&billow(3.3, 7.1, 9)));
        assert_eq!(ease(2.0), 1.0);
        assert_eq!(ease(-1.0), 0.0);
    }

    #[test]
    fn shapes_fill_where_they_should_and_clip_at_the_edges() {
        let (fill, ink) = (Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        let mut canvas = Canvas::new(10, 5, fill);
        disc(&mut canvas, 5.0, 5.0, 1.0, ink);
        assert_eq!(canvas.get(5, 4), Some(ink));
        assert_eq!(canvas.get(6, 4), Some(fill));
        assert_eq!(canvas.get(7, 5), Some(fill));
        let mut canvas = Canvas::new(10, 5, fill);
        ellipse(&mut canvas, 5.0, 5.0, 2.0, 2.0, ink);
        assert_eq!(canvas.get(5, 5), Some(ink));
        assert_eq!(canvas.get(0, 0), Some(fill));
        line(&mut canvas, (-5.0, 9.0), (20.0, 9.0), 1, ink);
        assert_eq!(canvas.get(9, 9), Some(ink));
        assert_eq!(canvas.get(9, 8), Some(fill));
        // shapes far off the canvas and degenerate ones are dropped without a panic
        ellipse(&mut canvas, -50.0, -50.0, 3.0, 0.0, ink);
        disc(&mut canvas, -100.0, 300.0, 4.0, ink);
        line(&mut canvas, (-50.0, -50.0), (-40.0, -45.0), 3, ink);
        let rows = mirrored(&["ab", "c."]);
        assert_eq!(rows, vec!["ba".to_string(), ".c".to_string()]);
    }

    #[test]
    fn the_blender_rounds_to_its_steps() {
        let (a, b) = (Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        let mut blender = Blender::new(8);
        assert_eq!(blender.mix(a, b, 0.01), a);
        assert_eq!(blender.mix(a, b, 1.0), b);
        assert_eq!(blender.mix(a, b, 0.5), blender.mix(a, b, 0.51));
        let mut coarse = Blender::new(4);
        assert_eq!(coarse.mix(a, b, 0.49), coarse.mix(a, b, 0.51));
    }

    #[test]
    fn shades_keep_their_outline_and_imagery_stays_below_the_ceiling_in_every_theme() {
        for theme in theme::all() {
            for colour in theme.drawable() {
                let (dark, _, light) = shaded(theme, colour);
                assert!(theme.contrast_of(dark) >= SOLID - 0.01, "{}", theme.name);
                assert!(theme.contrast_of(light) >= SOLID - 0.01, "{}", theme.name);
                assert!(theme.contrast_of(strong(theme, colour)) >= SOLID - 0.01);
                if theme.contrast_of(colour) >= theme.floor() {
                    let lit = stronger(theme, colour, 0.4);
                    assert!(
                        theme.contrast_of(lit) >= theme.contrast_of(colour) - 0.05,
                        "{} in {}",
                        colour.hex(),
                        theme.name
                    );
                }
            }
            let o = Overcast::new(theme);
            for colour in o
                .sky
                .steps()
                .iter()
                .chain(o.cloud.steps())
                .chain([&o.rain_far, &o.rain_near])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            let (body, shade) = cloud_tones(theme, IMAGERY_CEILING);
            assert!(theme.contrast_of(body) < IMAGERY_CEILING, "{}", theme.name);
            assert!(theme.contrast_of(shade) < IMAGERY_CEILING, "{}", theme.name);
        }
    }

    #[test]
    fn waves_stay_in_range_and_a_cloud_wraps_round_with_a_body_and_an_underside() {
        let mut rng = Rng::new(1);
        let wave = Wave::new(&mut rng, 10.0);
        for x in -100..100 {
            assert!((-1.0..=1.0).contains(&wave.at(x as f32)));
        }
        let mut cloud = Cloud::random(&mut rng, 40.0, (10.0, 10.0), (16.0, 16.0), 4.0);
        for _ in 0..1000 {
            cloud.drift(0.1, 40.0);
            assert!(cloud.x >= -cloud.width && cloud.x < 40.0 + cloud.width);
        }
        let (bg, body, shade) = (Rgb::new(0, 0, 0), Rgb::new(9, 9, 9), Rgb::new(5, 5, 5));
        let mut canvas = Canvas::new(40, 8, bg);
        cloud.x = 10.0;
        cloud.paint(&mut canvas, body, shade);
        let mut seen = (false, false);
        for y in 0..16 {
            for x in 0..40 {
                let c = canvas.get(x, y).unwrap();
                seen.0 |= c == body;
                seen.1 |= c == shade;
            }
        }
        assert!(seen.0 && seen.1);
    }
}
