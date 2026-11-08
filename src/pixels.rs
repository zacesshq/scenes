//! A pixel canvas for solid imagery. Each character cell holds two pixels stacked one above the
//! other: the upper half block drawn with the top pixel as its foreground and the bottom pixel as
//! its background. Shades come from the active theme's own colours, mixed in the OKLab space so
//! steps look even; background imagery is kept below the non-text contrast floor so it reads
//! as background and never competes with text.

use crate::colour::{MIN_ADJACENT_CONTRAST, Rgb};
use crate::frame::{Cell, Frame};
use crate::theme::ThemeData;

/// The contrast against the theme background that background imagery stays below. Anything at or
/// above it reads as a shape, which is also how a pixel is drawn when colour is turned off.
pub const IMAGERY_CEILING: f64 = MIN_ADJACENT_CONTRAST;

fn to_linear(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(value: f32) -> u8 {
    let c = value.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// A colour in OKLab: lightness and two opposing colour axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Lab {
    pub fn from_rgb(colour: Rgb) -> Lab {
        let (r, g, b) = (
            to_linear(colour.r),
            to_linear(colour.g),
            to_linear(colour.b),
        );
        let l = (0.412_221_46 * r + 0.536_332_54 * g + 0.051_445_995 * b).cbrt();
        let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
        let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
        Lab {
            l: 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            a: 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            b: 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        }
    }

    pub fn to_rgb(self) -> Rgb {
        let l = (self.l + 0.396_337_78 * self.a + 0.215_803_76 * self.b).powi(3);
        let m = (self.l - 0.105_561_346 * self.a - 0.063_854_17 * self.b).powi(3);
        let s = (self.l - 0.089_484_18 * self.a - 1.291_485_5 * self.b).powi(3);
        Rgb::new(
            from_linear(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
            from_linear(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
            from_linear(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
        )
    }

    /// The perceptual distance between two colours, about 0.02 for a just visible step.
    pub fn distance(self, other: Lab) -> f32 {
        ((self.l - other.l).powi(2) + (self.a - other.a).powi(2) + (self.b - other.b).powi(2))
            .sqrt()
    }
}

/// A perceptual mix: `t` of 0 is `a`, 1 is `b`.
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    let (x, y) = (Lab::from_rgb(a), Lab::from_rgb(b));
    Lab {
        l: x.l + (y.l - x.l) * t,
        a: x.a + (y.a - x.a) * t,
        b: x.b + (y.b - x.b) * t,
    }
    .to_rgb()
}

/// How far apart two colours look, in OKLab.
pub fn distance(a: Rgb, b: Rgb) -> f32 {
    Lab::from_rgb(a).distance(Lab::from_rgb(b))
}

/// Shades built from one theme's colours, so pixel imagery always matches the chosen theme.
#[derive(Clone, Copy, Debug)]
pub struct Shades<'a> {
    pub theme: &'a ThemeData,
}

impl<'a> Shades<'a> {
    pub fn new(theme: &'a ThemeData) -> Self {
        Self { theme }
    }

    /// The theme's lightest colour: the foreground on a dark theme, the background on a light one.
    pub fn lightest(&self) -> Rgb {
        if self.theme.dark {
            self.theme.foreground
        } else {
            self.theme.background
        }
    }

    /// The theme's darkest colour.
    pub fn darkest(&self) -> Rgb {
        if self.theme.dark {
            self.theme.background
        } else {
            self.theme.foreground
        }
    }

    /// `colour` moved `t` of the way towards the theme's lightest colour.
    pub fn lighter(&self, colour: Rgb, t: f32) -> Rgb {
        mix(colour, self.lightest(), t)
    }

    /// `colour` moved `t` of the way towards the theme's darkest colour.
    pub fn darker(&self, colour: Rgb, t: f32) -> Rgb {
        mix(colour, self.darkest(), t)
    }

    /// The background tinted `strength` of the way towards `hue`, held below `ceiling` contrast
    /// against the background so it stays background imagery.
    pub fn tint(&self, hue: Rgb, strength: f32, ceiling: f64) -> Rgb {
        let background = self.theme.background;
        let at = |t: f32| mix(background, hue, t);
        let wanted = at(strength);
        if wanted.contrast(background) < ceiling {
            return wanted;
        }
        // the contrast grows with the mix towards a hue that already clears the ceiling, so a
        // bisection finds the strongest tint still under it
        let (mut low, mut high) = (0.0_f32, strength.clamp(0.0, 1.0));
        for _ in 0..24 {
            let mid = (low + high) / 2.0;
            if at(mid).contrast(background) < ceiling {
                low = mid;
            } else {
                high = mid;
            }
        }
        at(low)
    }

    /// A ramp of `steps` colours from `from` to `to`.
    pub fn ramp(&self, from: Rgb, to: Rgb, steps: usize) -> Ramp {
        Ramp::new(from, to, steps)
    }

    /// `from` moved up to `t` of the way towards `to`, stopping short where it would fall below
    /// `floor` contrast against the background. A shaded part of a shape keeps its outline that
    /// way, with colour off as well as on. A `from` already below the floor comes back unchanged.
    pub fn held(&self, from: Rgb, to: Rgb, t: f32, floor: f64) -> Rgb {
        let background = self.theme.background;
        let keeps = |c: Rgb| c.contrast(background) >= floor;
        let wanted = mix(from, to, t);
        if keeps(wanted) {
            return wanted;
        }
        if !keeps(from) {
            return from;
        }
        let (mut low, mut high) = (0.0_f32, t.clamp(0.0, 1.0));
        for _ in 0..24 {
            let mid = (low + high) / 2.0;
            if keeps(mix(from, to, mid)) {
                low = mid;
            } else {
                high = mid;
            }
        }
        mix(from, to, low)
    }

    /// The strongest tint of the background towards `hue`, up to `strength`, that every colour
    /// in `text` still keeps `floor` against, so characters can sit on imagery without losing
    /// any of the contrast they are promised.
    pub fn under_text(&self, hue: Rgb, strength: f32, text: &[Rgb], floor: f64) -> Rgb {
        let background = self.theme.background;
        let at = |t: f32| mix(background, hue, t);
        let keeps = |tint: Rgb| text.iter().all(|c| c.contrast(tint) >= floor);
        if keeps(at(strength)) {
            return at(strength);
        }
        let (mut low, mut high) = (0.0_f32, strength.clamp(0.0, 1.0));
        for _ in 0..24 {
            let mid = (low + high) / 2.0;
            if keeps(at(mid)) {
                low = mid;
            } else {
                high = mid;
            }
        }
        at(low)
    }
}

/// Evenly spaced perceptual steps between two colours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ramp {
    steps: Vec<Rgb>,
}

impl Ramp {
    pub fn new(from: Rgb, to: Rgb, steps: usize) -> Self {
        let steps = steps.max(2);
        Self {
            steps: (0..steps)
                .map(|i| mix(from, to, i as f32 / (steps - 1) as f32))
                .collect(),
        }
    }

    /// The step nearest to `t`, from 0 for the first to 1 for the last. Picking a step rather
    /// than mixing keeps the number of colours small, which keeps the escape output small.
    pub fn at(&self, t: f32) -> Rgb {
        let last = self.steps.len() - 1;
        let i = (t.clamp(0.0, 1.0) * last as f32).round() as usize;
        self.steps[i.min(last)]
    }

    pub fn steps(&self) -> &[Rgb] {
        &self.steps
    }
}

/// A buffer of pixels twice as tall as the frame it is drawn into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Rgb>,
}

impl Canvas {
    /// A canvas for a frame of `columns` by `rows` cells, filled with one colour.
    pub fn new(columns: usize, rows: usize, fill: Rgb) -> Self {
        let width = columns.max(1);
        let height = rows.max(1) * 2;
        Self {
            width,
            height,
            pixels: vec![fill; width * height],
        }
    }

    /// A canvas the size of `frame`.
    pub fn for_frame(frame: &Frame, fill: Rgb) -> Self {
        Self::new(frame.width(), frame.height(), fill)
    }

    /// Width in pixels, the frame's width in cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in pixels, twice the frame's height in cells.
    pub fn height(&self) -> usize {
        self.height
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }

    /// Sets one pixel. Anything outside the canvas is dropped, as with the frame.
    pub fn set(&mut self, x: i32, y: i32, colour: Rgb) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = colour;
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Mixes `colour` over one pixel by `amount`, from 0 for no change to 1 for a plain set.
    pub fn blend(&mut self, x: i32, y: i32, colour: Rgb, amount: f32) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = mix(self.pixels[i], colour, amount);
        }
    }

    /// Fills a rectangle of pixels, clipped to the canvas.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, colour: Rgb) {
        for py in y.max(0)..(y + height).min(self.height as i32) {
            for px in x.max(0)..(x + width).min(self.width as i32) {
                self.set(px, py, colour);
            }
        }
    }

    /// Fills one row of pixels.
    pub fn fill_row(&mut self, y: i32, colour: Rgb) {
        self.fill_rect(0, y, self.width as i32, 1, colour);
    }

    /// Fills a disc of radius `r` around a point, clipped to the canvas.
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, colour: Rgb) {
        let Some((xs, ys)) = self.disc_box(cx, cy, r) else {
            return;
        };
        for y in ys.0..=ys.1 {
            for x in xs.0..=xs.1 {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.set(x, y, colour);
                }
            }
        }
    }

    /// The columns and rows a disc can touch, cut to the canvas. `None` means it misses it.
    /// The sums run in `i64` so a radius near the `i32` limit cannot overflow.
    fn disc_box(&self, cx: f32, cy: f32, r: f32) -> Option<((i32, i32), (i32, i32))> {
        let reach = i64::from(r.ceil() as i32) + 1;
        let span = |centre: f32, size: usize| {
            let middle = i64::from(centre.round() as i32);
            let low = (middle - reach).max(0);
            let high = (middle + reach).min(size as i64 - 1);
            (low <= high).then_some((low as i32, high as i32))
        };
        Some((span(cx, self.width)?, span(cy, self.height)?))
    }

    /// How many pixels a disc's box covers once cut to the canvas, which is what filling it
    /// costs.
    pub fn disc_cost(&self, cx: f32, cy: f32, r: f32) -> u64 {
        self.disc_box(cx, cy, r).map_or(0, |((x0, x1), (y0, y1))| {
            (x1 - x0 + 1) as u64 * (y1 - y0 + 1) as u64
        })
    }

    /// Draws a one pixel line between two points.
    pub fn line(&mut self, from: (i32, i32), to: (i32, i32), colour: Rgb) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let steps = dx.abs().max(dy.abs()).max(1);
        let Some((first, last)) = self.line_steps(from, to) else {
            return;
        };
        for i in first..=last {
            let x = from.0 as f32 + dx as f32 * i as f32 / steps as f32;
            let y = from.1 as f32 + dy as f32 * i as f32 / steps as f32;
            self.set(x.round() as i32, y.round() as i32, colour);
        }
    }

    /// The steps of a line that can land on the canvas. `None` means none can. Each axis
    /// gives the range of steps whose position lies within a pixel of the canvas, widened by one
    /// step either side so rounding never drops a pixel the full walk would have set.
    fn line_steps(&self, from: (i32, i32), to: (i32, i32)) -> Option<(i32, i32)> {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let steps = dx.abs().max(dy.abs()).max(1);
        let axis = |start: i32, delta: i32, size: usize| -> Option<(i32, i32)> {
            let (low, high) = (-1.0, size as f64);
            if delta == 0 {
                let start = f64::from(start);
                return (low <= start && start <= high).then_some((0, steps));
            }
            let at = |edge: f64| (edge - f64::from(start)) * f64::from(steps) / f64::from(delta);
            let (a, b) = (at(low), at(high));
            let first = (a.min(b).floor() as i64 - 1).max(0);
            let last = (a.max(b).ceil() as i64 + 1).min(i64::from(steps));
            (first <= last).then_some((first as i32, last as i32))
        };
        let (x0, x1) = axis(from.0, dx, self.width)?;
        let (y0, y1) = axis(from.1, dy, self.height)?;
        let (first, last) = (x0.max(y0), x1.min(y1));
        (first <= last).then_some((first, last))
    }

    /// How many steps of a line fall on or next to the canvas, which is what drawing it costs.
    pub fn line_cost(&self, from: (i32, i32), to: (i32, i32)) -> u64 {
        self.line_steps(from, to)
            .map_or(0, |(first, last)| (last - first + 1) as u64)
    }

    /// Draws a pixel sprite: every character in `palette` takes its colour and anything else,
    /// a space or a dot, is left transparent.
    pub fn sprite(&mut self, x: i32, y: i32, rows: &[&str], palette: &[(char, Rgb)]) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if let Some(&(_, colour)) = palette.iter().find(|(key, _)| *key == ch) {
                    self.set(x + dx as i32, y + dy as i32, colour);
                }
            }
        }
    }

    /// Writes every pixel pair into the frame as one cell: the top pixel as the foreground of
    /// the upper half block and the bottom pixel as its background.
    pub fn blit(&self, frame: &mut Frame) {
        let rows = (self.height / 2).min(frame.height());
        let columns = self.width.min(frame.width());
        for y in 0..rows {
            for x in 0..columns {
                let top = self.pixels[2 * y * self.width + x];
                let bottom = self.pixels[(2 * y + 1) * self.width + x];
                frame.put_cell(x as i32, y as i32, Cell::pixels(top, bottom));
            }
        }
    }
}

/// The character a pixel cell becomes when colour is turned off: each pixel that stands out from
/// the background at the non-text floor or more is drawn as solid, the rest as nothing, so shapes
/// keep their outline while the background imagery drops away.
pub fn plain_glyph(top: Rgb, bottom: Rgb, background: Rgb) -> char {
    let ink = |c: Rgb| c.contrast(background) >= IMAGERY_CEILING;
    match (ink(top), ink(bottom)) {
        (false, false) => ' ',
        (true, false) => '\u{2580}',
        (false, true) => '\u{2584}',
        (true, true) => '\u{2588}',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::HALF_BLOCK;
    use crate::theme;

    #[test]
    fn oklab_round_trips_and_mixes_stay_between_their_ends() {
        for theme in theme::all() {
            for colour in theme.drawable() {
                let back = Lab::from_rgb(colour).to_rgb();
                let close = |a: u8, b: u8| a.abs_diff(b) <= 1;
                assert!(
                    close(back.r, colour.r) && close(back.g, colour.g) && close(back.b, colour.b),
                    "{} came back as {}",
                    colour.hex(),
                    back.hex()
                );
            }
        }
        let (black, white) = (Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        assert_eq!(mix(black, white, 0.0), black);
        assert_eq!(mix(black, white, 1.0), white);
        let mid = mix(black, white, 0.5);
        assert!(
            mid.luminance() > 0.1 && mid.luminance() < 0.3,
            "{}",
            mid.hex()
        );
    }

    #[test]
    fn blit_pairs_two_pixels_into_one_half_block_cell() {
        let (a, b, c) = (Rgb::new(10, 0, 0), Rgb::new(0, 20, 0), Rgb::new(0, 0, 30));
        let mut canvas = Canvas::new(3, 2, c);
        assert_eq!((canvas.width(), canvas.height()), (3, 4));
        canvas.set(0, 0, a);
        canvas.set(0, 1, b);
        canvas.set(1, 2, a);
        let mut frame = Frame::new(3, 2);
        canvas.blit(&mut frame);
        let first = frame.get(0, 0).unwrap();
        assert_eq!(
            (first.ch, first.colour, first.bg),
            (HALF_BLOCK, Some(a), Some(b))
        );
        // an untouched pair is one colour, so it is a plain space on that background
        let plain = frame.get(2, 0).unwrap();
        assert_eq!((plain.ch, plain.colour, plain.bg), (' ', None, Some(c)));
        assert_eq!(frame.get(1, 1).unwrap().pixel_pair(), Some((a, c)));
    }

    #[test]
    fn drawing_off_the_canvas_is_dropped() {
        let fill = Rgb::new(1, 1, 1);
        let mut canvas = Canvas::new(2, 1, fill);
        let red = Rgb::new(255, 0, 0);
        canvas.set(-1, 0, red);
        canvas.set(2, 0, red);
        canvas.set(0, 2, red);
        canvas.fill_rect(-5, -5, 6, 6, red);
        canvas.sprite(1, -1, &["##", "#."], &[('#', red)]);
        assert_eq!(canvas.get(0, 0), Some(red));
        assert_eq!(canvas.get(1, 0), Some(red));
        assert_eq!(canvas.get(1, 1), Some(fill));
        assert_eq!(canvas.get(5, 5), None);
        // a canvas bigger than the frame is clipped and a smaller one leaves the rest alone
        let mut small = Frame::new(1, 1);
        canvas.blit(&mut small);
        assert_eq!(small.get(0, 0).unwrap().pixel_pair(), Some((red, fill)));
        let mut big = Frame::new(4, 3);
        Canvas::new(1, 1, red).blit(&mut big);
        assert_eq!(big.get(3, 2), Some(Cell::BLANK));
        assert_eq!(Canvas::new(0, 0, fill).height(), 2);
    }

    #[test]
    fn blending_moves_part_of_the_way() {
        let mut canvas = Canvas::new(1, 1, Rgb::new(0, 0, 0));
        canvas.blend(0, 0, Rgb::new(255, 255, 255), 0.0);
        assert_eq!(canvas.get(0, 0), Some(Rgb::new(0, 0, 0)));
        canvas.blend(0, 0, Rgb::new(255, 255, 255), 0.5);
        let half = canvas.get(0, 0).unwrap();
        assert!(half.r > 50 && half.r < 200, "{}", half.hex());
        canvas.blend(0, 0, Rgb::new(255, 255, 255), 1.0);
        assert_eq!(canvas.get(0, 0), Some(Rgb::new(255, 255, 255)));
    }

    #[test]
    fn tints_stay_below_the_ceiling_in_every_theme() {
        for theme in theme::all() {
            let shades = Shades::new(theme);
            for hue in theme.drawable() {
                for strength in [0.2, 0.6, 1.0] {
                    let tint = shades.tint(hue, strength, 2.5);
                    assert!(theme.contrast_of(tint) < 2.5, "{}", theme.name);
                }
            }
            assert!(shades.lightest().luminance() > shades.darkest().luminance());
            let grey = mix(theme.background, theme.foreground, 0.5);
            assert!(shades.lighter(grey, 0.5).luminance() > grey.luminance());
            assert!(shades.darker(grey, 0.5).luminance() < grey.luminance());
        }
    }

    #[test]
    fn held_shades_keep_their_floor_and_text_tints_keep_the_text_floor() {
        for theme in theme::all() {
            let shades = Shades::new(theme);
            for colour in theme.drawable() {
                for target in [theme.background, shades.darkest(), shades.lightest()] {
                    let held = shades.held(colour, target, 0.8, IMAGERY_CEILING);
                    assert!(
                        theme.contrast_of(held) >= IMAGERY_CEILING
                            || theme.contrast_of(colour) < IMAGERY_CEILING,
                        "{}",
                        theme.name
                    );
                }
            }
            let text = [theme.foreground, theme.bright()];
            let tint = shades.under_text(theme.accent, 0.5, &text, theme.floor());
            for colour in text {
                assert!(colour.contrast(tint) >= theme.floor(), "{}", theme.name);
            }
        }
    }

    #[test]
    fn discs_and_lines_fill_the_pixels_between_their_ends() {
        let (fill, ink) = (Rgb::new(0, 0, 0), Rgb::new(9, 9, 9));
        let mut canvas = Canvas::new(9, 5, fill);
        canvas.disc(4.0, 4.0, 2.0, ink);
        for (x, y) in [(4, 4), (2, 4), (6, 4), (4, 2), (4, 6)] {
            assert_eq!(canvas.get(x, y), Some(ink), "{x},{y}");
        }
        assert_eq!(canvas.get(2, 2), Some(fill));
        let mut canvas = Canvas::new(9, 5, fill);
        canvas.line((0, 0), (8, 4), ink);
        assert_eq!(canvas.get(0, 0), Some(ink));
        assert_eq!(canvas.get(8, 4), Some(ink));
        assert_eq!(canvas.get(4, 2), Some(ink));
        // a disc or a line off the canvas is clipped, never a panic
        canvas.disc(-50.0, 90.0, 4.0, ink);
        canvas.line((-20, -20), (40, 40), ink);
    }

    /// The unclipped disc and line from before shapes were cut to the canvas, kept to check the
    /// clipped ones paint exactly the same pixels.
    fn walk_disc(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, colour: Rgb) {
        let reach = r.ceil() as i32 + 1;
        let (x0, y0) = (cx.round() as i32, cy.round() as i32);
        for y in y0 - reach..=y0 + reach {
            for x in x0 - reach..=x0 + reach {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                if dx * dx + dy * dy <= r * r {
                    canvas.set(x, y, colour);
                }
            }
        }
    }

    fn walk_line(canvas: &mut Canvas, from: (i32, i32), to: (i32, i32), colour: Rgb) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let steps = dx.abs().max(dy.abs()).max(1);
        for i in 0..=steps {
            let x = from.0 as f32 + dx as f32 * i as f32 / steps as f32;
            let y = from.1 as f32 + dy as f32 * i as f32 / steps as f32;
            canvas.set(x.round() as i32, y.round() as i32, colour);
        }
    }

    #[test]
    fn clipped_shapes_paint_the_same_pixels_as_the_full_walk() {
        let (fill, ink) = (Rgb::new(0, 0, 0), Rgb::new(9, 9, 9));
        let mut rng = crate::rng::Rng::new(7);
        let mut spread = |scale: f32| (rng.unit() - 0.5) * scale;
        for _ in 0..400 {
            let (cx, cy, r) = (spread(80.0), spread(60.0), spread(90.0).abs());
            let (mut clipped, mut full) = (Canvas::new(12, 5, fill), Canvas::new(12, 5, fill));
            clipped.disc(cx, cy, r, ink);
            walk_disc(&mut full, cx, cy, r, ink);
            assert_eq!(clipped, full, "disc at {cx},{cy} radius {r}");
            let from = (spread(3000.0) as i32, spread(3000.0) as i32);
            let to = (spread(3000.0) as i32, spread(3000.0) as i32);
            let (mut clipped, mut full) = (Canvas::new(12, 5, fill), Canvas::new(12, 5, fill));
            clipped.line(from, to, ink);
            walk_line(&mut full, from, to, ink);
            assert_eq!(clipped, full, "line from {from:?} to {to:?}");
        }
        // lines that only graze the canvas or run along an edge
        for (from, to) in [
            ((-9000, 0), (9000, 0)),
            ((-9000, -1), (9000, 10)),
            ((11, -5000), (11, 5000)),
            ((-10, 20), (20, -10)),
            ((12, 0), (12, 9)),
            ((-1, -1), (-1, -1)),
        ] {
            let (mut clipped, mut full) = (Canvas::new(12, 5, fill), Canvas::new(12, 5, fill));
            clipped.line(from, to, ink);
            walk_line(&mut full, from, to, ink);
            assert_eq!(clipped, full, "line from {from:?} to {to:?}");
        }
    }

    #[test]
    fn a_huge_disc_on_a_small_canvas_is_quick_and_fills_it() {
        let (fill, ink) = (Rgb::new(0, 0, 0), Rgb::new(9, 9, 9));
        let mut canvas = Canvas::new(12, 5, fill);
        let start = std::time::Instant::now();
        canvas.disc(6.0, 5.0, 1.0e9, ink);
        canvas.disc(1.0e9, 1.0e9, 1.0e9, ink);
        assert!(start.elapsed() < std::time::Duration::from_millis(50));
        assert_eq!(canvas, Canvas::new(12, 5, ink));
        // the same as the full walk for a radius it can still afford
        let (mut clipped, mut full) = (Canvas::new(12, 5, fill), Canvas::new(12, 5, fill));
        clipped.disc(6.0, 5.0, 900.0, ink);
        walk_disc(&mut full, 6.0, 5.0, 900.0, ink);
        assert_eq!(clipped, full);
        assert_eq!(canvas.disc_cost(6.0, 5.0, 1.0e9), 12 * 10);
        assert_eq!(canvas.disc_cost(-500.0, 5.0, 3.0), 0);
        assert_eq!(canvas.line_cost((-9000, 3), (-20, 3)), 0);
    }

    #[test]
    fn ramps_step_evenly_and_pick_the_nearest_step() {
        let ramp = Ramp::new(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255), 5);
        assert_eq!(ramp.steps().len(), 5);
        assert_eq!(ramp.at(0.0), Rgb::new(0, 0, 0));
        assert_eq!(ramp.at(1.0), Rgb::new(255, 255, 255));
        assert_eq!(ramp.at(2.0), Rgb::new(255, 255, 255));
        assert_eq!(ramp.at(0.49), ramp.steps()[2]);
        let lum: Vec<f64> = ramp.steps().iter().map(|c| c.luminance()).collect();
        assert!(lum.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn without_colour_only_shapes_keep_an_outline() {
        let theme = theme::default_theme();
        let ground = Shades::new(theme).tint(theme.blue(), 0.6, IMAGERY_CEILING);
        let shape = theme.yellow();
        let bg = theme.background;
        assert_eq!(plain_glyph(ground, ground, bg), ' ');
        assert_eq!(plain_glyph(shape, ground, bg), '\u{2580}');
        assert_eq!(plain_glyph(ground, shape, bg), '\u{2584}');
        assert_eq!(plain_glyph(shape, shape, bg), '\u{2588}');
    }
}
