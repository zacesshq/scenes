//! Tea, painted in solid pixels: a warm room with a striped wall, a shaded mug of tea on a
//! wooden table with steam curling up from it, a stack of books and a potted plant beside it, a
//! hanging lamp and, behind, a curtained window with rain or snow falling over the rooftops
//! outside.
//!
//! Every colour comes from the active theme. The wall, the glass and the rooftops are tints of
//! the background held below the non-text floor, so they read as background; the mug, the
//! table, the books, the plant and the lamp are shaded from the theme's own drawing colours, so
//! they stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A painted mug of tea with curling steam beside books and a plant, with weather at the window";

pub const CAPTIONS: &[&str] = &[
    "Build passed. Put the kettle on.",
    "A clean build and a warm mug.",
    "Time for a proper cup of tea.",
    "Nothing to fix. Let it brew.",
    "Green build, hot tea, quiet room.",
];

/// Seconds a curl of steam takes to sway from one side to the other and back. Its pixels follow
/// the sway, so each moves across a column twice a period at most.
pub const SWAY_PERIOD: f32 = 2.4;
/// Seconds a curl of steam lasts.
const STEAM_LIFE: f32 = 5.0;
/// Pixels a second the steam rises at, at the standard speed.
const RISE: f32 = 2.2;
const WALL_STEPS: usize = 8;

/// The large mug: rim, tea, body with a painted band, handle and a rounded foot. `l` is the lit
/// side, `s` the shaded side.
const MUG: [&str; 11] = [
    "lrrrrrrrrrrs....",
    "lttgtttttttts...",
    "lbbbbbbbbbbbs...",
    "lbbbbbbbbbbbshh.",
    "ldddddddddddsh.h",
    "lbbbbbbbbbbbs..h",
    "lbbbbbbbbbbbs..h",
    "lbbbbbbbbbbbsh.h",
    "lbbbbbbbbbbbshh.",
    ".lbbbbbbbbbs....",
    "..ssssssssss....",
];
const SMALL_MUG: [&str; 6] = [
    "lrrrrrs..",
    "lttgtts..",
    "lbbbbbshh",
    "lddddds.h",
    "lbbbbbshh",
    ".sssss...",
];

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
}

/// The window in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

/// The room's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct TeaColours {
    pub wall_top: Rgb,
    pub wall_low: Rgb,
    pub stripe: Rgb,
    pub glass_top: Rgb,
    pub glass_low: Rgb,
    pub roofs: Rgb,
    pub rain: Rgb,
    pub snow: Rgb,
    pub steam: Rgb,
    pub glow: Rgb,
    pub wood: Rgb,
    pub wood_lit: Rgb,
    pub wood_dark: Rgb,
    pub curtain: Rgb,
    pub curtain_fold: Rgb,
    pub mug: Rgb,
    pub mug_lit: Rgb,
    pub mug_shade: Rgb,
    pub mug_band: Rgb,
    pub tea: Rgb,
    pub glint: Rgb,
    pub books: [Rgb; 3],
    pub pages: Rgb,
    pub leaf: Rgb,
    pub leaf_lit: Rgb,
    pub pot: Rgb,
    pub pot_shade: Rgb,
    pub lamp: Rgb,
    pub cord: Rgb,
    wall: Ramp,
    glass: Ramp,
}

impl TeaColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // the wall warms towards the lamp-lit table on a dark theme and pales towards it on a
        // light one, so the table always sits in the brightest part of the room
        let (wall_top, wall_low) = if theme.dark {
            (
                shades.tint(theme.warning, 0.14, 1.25),
                shades.tint(theme.warning, 0.32, 1.6),
            )
        } else {
            (
                shades.tint(theme.warning, 0.35, 1.55),
                shades.tint(theme.warning, 0.2, 1.28),
            )
        };
        let (glass_top, glass_low, roofs) = if theme.dark {
            (
                shades.tint(theme.info, 0.08, 1.1),
                shades.tint(theme.blue(), 0.35, 1.6),
                theme.background,
            )
        } else {
            (
                shades.tint(theme.info, 0.55, 2.0),
                shades.tint(theme.info, 0.25, 1.45),
                shades.tint(theme.muted, 0.6, 2.6),
            )
        };
        let rain = shades.tint(shades.lighter(glass_low, 0.45), 1.0, IMAGERY_CEILING - 0.1);
        // the palette's own yellow, since on a light theme with a 7 to 1 floor the yellow kept
        // for text leans so far to red that the table would match the red book
        let wood = shades.darker(mix(theme.ansi[3], theme.red(), 0.35), 0.25);
        let curtain = theme.blue();
        let mug = theme.cyan();
        let tea = shades.darker(mix(theme.yellow(), theme.red(), 0.5), 0.45);
        let leaf = theme.green();
        let pot = mix(theme.red(), theme.yellow(), 0.4);
        Self {
            wall_top,
            wall_low,
            stripe: mix(wall_top, wall_low, 0.5),
            glass_top,
            glass_low,
            roofs,
            rain,
            snow: shades.lighter(glass_low, 0.8),
            // steam is paler than the wall in every theme; a light theme's wall is tinted
            // enough for the white of the page to show against it
            steam: if theme.dark {
                shades.lighter(wall_low, 0.55)
            } else {
                theme.background
            },
            glow: theme.bright_yellow(),
            wood,
            wood_lit: shades.lighter(wood, 0.25),
            wood_dark: shades.darker(wood, 0.35),
            curtain,
            curtain_fold: shades.darker(curtain, 0.35),
            mug,
            mug_lit: shades.lighter(mug, 0.4),
            mug_shade: shades.darker(mug, 0.35),
            mug_band: theme.magenta(),
            tea,
            glint: shades.lighter(tea, 0.45),
            books: [theme.red(), theme.blue(), theme.green()],
            pages: shades.lighter(theme.muted, 0.55),
            leaf,
            leaf_lit: shades.lighter(leaf, 0.3),
            pot,
            pot_shade: shades.darker(pot, 0.35),
            lamp: theme.bright_yellow(),
            cord: theme.grey(),
            wall: Ramp::new(wall_top, wall_low, WALL_STEPS),
            glass: Ramp::new(glass_top, glass_low, WALL_STEPS),
        }
    }
}

pub struct Tea {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    snow: bool,
    puffs: Vec<Puff>,
    since_puff: f32,
    window: Option<Window>,
    weather: Vec<Flake>,
}

impl Tea {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tea = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            snow: rng.chance(0.5),
            puffs: Vec::new(),
            since_puff: 0.0,
            window: None,
            weather: Vec::new(),
        };
        tea.resize(width, height, rng);
        tea
    }

    fn large(&self) -> bool {
        self.width >= 40 && self.height >= 14
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the table top.
    fn table(&self) -> i32 {
        let tall = self.tall();
        if self.large() { tall - 6 } else { tall - 3 }
    }

    fn mug_rows(&self) -> &'static [&'static str] {
        if self.large() { &MUG } else { &SMALL_MUG }
    }

    /// The column of the mug's left edge and the column steam rises from.
    fn mug_x(&self) -> (i32, i32) {
        let rows = self.mug_rows();
        let w = rows[0].len() as i32;
        let left = (self.width as i32 * 3 / 5 - w / 2).max(0);
        // the steam rises from the middle of the cup, not of the cup and handle together
        (left, left + (w - 4) / 2)
    }

    fn mug_top(&self) -> i32 {
        self.table() - self.mug_rows().len() as i32
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    fn puff_gap(&self) -> f32 {
        let gap = if self.motion.calm { 0.9 } else { 0.45 };
        gap / self.motion.busy()
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let Some(w) = self.window else {
            return Flake {
                x: 0.0,
                y: 0.0,
                speed: 1.0,
            };
        };
        let speed = if self.snow {
            rng.range(2.0, 4.0)
        } else {
            rng.range(10.0, 16.0)
        };
        Flake {
            x: rng.range(0.0, (w.width - 4).max(1) as f32),
            y: if from_top {
                0.0
            } else {
                rng.range(0.0, (w.height - 4).max(1) as f32)
            },
            speed: speed * self.motion.speed(),
        }
    }

    fn paint_window(&self, canvas: &mut Canvas, c: &TeaColours) {
        let Some(win) = self.window else {
            return;
        };
        let (l, t, w, h) = (win.left, win.top, win.width, win.height);
        let (inner_l, inner_t, inner_w, inner_h) = (l + 2, t + 2, w - 4, h - 4);
        // the evening outside, darkest at the top
        for y in 0..inner_h {
            let colour = c.glass.at(y as f32 / inner_h.max(1) as f32);
            canvas.fill_rect(inner_l, inner_t + y, inner_w, 1, colour);
        }
        // rooftops and chimneys across the bottom of the glass
        let roof_base = inner_t + inner_h;
        for x in 0..inner_w {
            let hash = ((x / 5) as u32).wrapping_mul(0x9e37_79b1) >> 27;
            let rise = 3 + (hash % 4) as i32 + if x % 5 == 2 { 1 } else { 0 };
            let chimney = x % 11 == 7 && hash % 2 == 0;
            let top = roof_base - rise - if chimney { 2 } else { 0 };
            canvas.fill_rect(inner_l + x, top, 1, roof_base - top, c.roofs);
        }
        for flake in &self.weather {
            let x = inner_l + flake.x as i32;
            let y = inner_t + flake.y as i32;
            if self.snow {
                canvas.set(x, y, c.snow);
            } else {
                // a short streak, as rain looks at a glance
                for dy in 0..3 {
                    if y + dy < roof_base - 3 {
                        canvas.set(x, y + dy, c.rain);
                    }
                }
            }
        }
        // the frame, a cross of glazing bars and a deeper sill
        let mid_x = l + w / 2;
        let mid_y = t + h / 2;
        canvas.fill_rect(l, t, w, 2, c.wood);
        canvas.fill_rect(l, t + h - 2, w, 2, c.wood);
        canvas.fill_rect(l, t, 2, h, c.wood);
        canvas.fill_rect(l + w - 2, t, 2, h, c.wood);
        canvas.fill_rect(mid_x, t, 1, h, c.wood);
        canvas.fill_rect(l, mid_y, w, 1, c.wood);
        canvas.fill_rect(l, t, w, 1, c.wood_lit);
        canvas.fill_rect(l - 2, t + h, w + 4, 1, c.wood_lit);
        canvas.fill_rect(l - 2, t + h + 1, w + 4, 1, c.wood_dark);
        // curtains gathered either side, in soft vertical folds
        let drape = 4;
        for (side, x0) in [(0, l - drape - 1), (1, l + w + 1)] {
            for dx in 0..drape {
                let fold = (dx + side) % 2 == 0;
                let colour = if fold { c.curtain_fold } else { c.curtain };
                canvas.fill_rect(x0 + dx, t - 1, 1, h + 1, colour);
            }
        }
    }

    fn paint_lamp(&self, canvas: &mut Canvas, c: &TeaColours) {
        let wide = self.width as i32;
        let x = wide * 5 / 6;
        let clear = self
            .window
            .is_some_and(|win| win.left + win.width + 10 < x - 4);
        if !self.large() || !clear {
            return;
        }
        let drop = (self.table() / 5).max(2);
        canvas.fill_rect(x, 0, 1, drop, c.cord);
        canvas.sprite(
            x - 4,
            drop,
            &[
                "...lll...",
                "..lllll..",
                ".lllllll.",
                "lllllllll",
                "...ggg...",
            ],
            &[('l', c.lamp), ('g', c.glow)],
        );
        // a faint halo round the bulb, which never changes
        for dy in -2..=3 {
            for dx in -7..=7 {
                let d = ((dx * dx) as f32 / 49.0 + (dy * dy) as f32 / 9.0).sqrt();
                if d <= 1.0 && canvas.get(x + dx, drop + 5 + dy) != Some(c.lamp) {
                    canvas.blend(x + dx, drop + 5 + dy, c.glow, 0.12 * (1.0 - d));
                }
            }
        }
    }

    fn paint_table(&self, canvas: &mut Canvas, c: &TeaColours) {
        let table = self.table();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        canvas.fill_row(table, c.wood_lit);
        canvas.fill_rect(0, table + 1, wide, 1, c.wood);
        if self.large() {
            canvas.fill_rect(0, table + 2, wide, 2, c.wood_dark);
            for x in [4, wide - 7] {
                canvas.fill_rect(x, table + 4, 3, tall - table - 4, c.wood);
                canvas.fill_rect(x, table + 4, 1, tall - table - 4, c.wood_lit);
            }
            // a little grain along the apron
            for x in (0..wide).step_by(9) {
                canvas.fill_rect(x + 2, table + 2, 4, 1, c.wood);
            }
        } else {
            canvas.fill_rect(0, table + 2, wide, tall - table - 2, c.wood_dark);
        }
    }

    fn paint_mug(&self, canvas: &mut Canvas, c: &TeaColours) {
        let (left, _) = self.mug_x();
        let top = self.mug_top();
        let palette = [
            ('l', c.mug_lit),
            ('b', c.mug),
            ('s', c.mug_shade),
            ('r', c.mug_lit),
            ('t', c.tea),
            ('g', c.glint),
            ('d', c.mug_band),
            ('h', c.mug),
        ];
        canvas.sprite(left, top, self.mug_rows(), &palette);
    }

    fn paint_books_and_plant(&self, canvas: &mut Canvas, c: &TeaColours) {
        if !self.large() {
            return;
        }
        let table = self.table();
        let (mug_left, _) = self.mug_x();
        let shades_for = |colour: Rgb| {
            (
                mix(colour, c.pages, 0.3),
                colour,
                mix(colour, c.wood_dark, 0.4),
            )
        };
        // three books lying flat, a little out of line, their page edges facing the mug
        let books = [(22, 0), (19, 2), (24, 1)];
        let mut y = table;
        for (i, &(length, offset)) in books.iter().enumerate() {
            y -= 3;
            let x = mug_left - 28 + offset;
            let (lit, body, dark) = shades_for(c.books[i]);
            canvas.fill_rect(x, y, length, 1, lit);
            canvas.fill_rect(x, y + 1, length, 1, body);
            canvas.fill_rect(x, y + 2, length, 1, dark);
            canvas.fill_rect(x + length - 2, y + 1, 1, 1, c.pages);
            canvas.fill_rect(x + 3, y + 1, 5, 1, lit);
        }
        // a potted plant on the other side of the mug
        let px = mug_left + 22;
        if px + 10 >= self.width as i32 {
            return;
        }
        canvas.sprite(
            px,
            table - 14,
            &[
                "....L.....",
                "...LG..L..",
                ".L.GG.LG..",
                ".GG.GGG...",
                "..GGGG.LG.",
                "LG.GGGGG..",
                ".GGG.GG...",
                "...GGG....",
                "..PPPPPP..",
                "..ppppps..",
                "...pppps..",
                "...pppps..",
                "....pps...",
                "....pps...",
            ],
            &[
                ('L', c.leaf_lit),
                ('G', c.leaf),
                ('P', c.pot),
                ('p', c.pot),
                ('s', c.pot_shade),
            ],
        );
    }

    fn paint_steam(&self, canvas: &mut Canvas, c: &TeaColours) {
        let period = self.sway_period();
        for puff in &self.puffs {
            let sway = cycle(self.t, TAU / period) + puff.phase * TAU;
            let x = (puff.x + sway.sin() * 1.2).round() as i32;
            let y = puff.y.round() as i32;
            // a curl thins as it rises and grows from one pixel to a small cloud
            let fade = (1.0 - puff.age / STEAM_LIFE).clamp(0.0, 1.0);
            let amount = (0.25 + 0.55 * fade).min(0.8);
            canvas.blend(x, y, c.steam, amount);
            if puff.age > STEAM_LIFE * 0.3 {
                canvas.blend(x + 1, y, c.steam, amount * 0.6);
                canvas.blend(x, y - 1, c.steam, amount * 0.5);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &TeaColours) {
        let table = self.table();
        let wide = canvas.width() as i32;
        for y in 0..table {
            let colour = c.wall.at(y as f32 / table.max(1) as f32);
            canvas.fill_row(y, colour);
            // a soft wallpaper stripe every few columns
            let mut x = 3;
            while x < wide {
                canvas.set(x, y, mix(colour, c.stripe, 0.4));
                x += 7;
            }
        }
        self.paint_window(canvas, c);
        self.paint_lamp(canvas, c);
        self.paint_table(canvas, c);
        self.paint_books_and_plant(canvas, c);
        self.paint_mug(canvas, c);
        self.paint_steam(canvas, c);
    }
}

impl Scene for Tea {
    fn name(&self) -> &'static str {
        "tea"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.puffs.clear();
        let w = self.width as i32;
        let h = self.height as i32;
        self.window = if w >= 30 && h >= 12 {
            let width = (w / 3).clamp(14, 40);
            let height = ((self.table() - 8) * 2 / 3).clamp(10, 36);
            Some(Window {
                left: (w / 6).max(6),
                top: 3,
                width,
                height,
            })
        } else {
            None
        };
        let wanted = self.window.map_or(0, |w| {
            self.motion
                .scaled(count_for(w.width as usize, w.height as usize, 30, 3, 80))
        });
        self.weather = (0..wanted).map(|_| self.new_flake(rng, false)).collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let (_, steam_x) = self.mug_x();
        self.since_puff += dt;
        if self.since_puff >= self.puff_gap() {
            self.since_puff = 0.0;
            self.puffs.push(Puff {
                x: steam_x as f32 + rng.range(-2.5, 2.5),
                y: (self.mug_top() - 1) as f32,
                age: 0.0,
                phase: rng.range(0.0, 1.0),
            });
        }
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.y -= RISE * speed * dt;
        }
        self.puffs.retain(|p| p.age < STEAM_LIFE && p.y >= 0.0);

        if let Some(w) = self.window {
            let inner = (w.height - 4).max(1) as f32;
            let across = (w.width - 4).max(1) as f32;
            for i in 0..self.weather.len() {
                let flake = &mut self.weather[i];
                flake.y += flake.speed * dt;
                if self.snow {
                    flake.x = wrap(flake.x + 0.8 * speed * dt, across);
                }
                if flake.y >= inner {
                    self.weather[i] = self.new_flake(rng, true);
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = TeaColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("tea");
        check_determinism("tea");
        check_pixel_scene("tea", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        assert!(MUG.iter().all(|r| r.len() == MUG[0].len()));
        assert!(SMALL_MUG.iter().all(|r| r.len() == SMALL_MUG[0].len()));
    }

    #[test]
    fn steam_rises_from_the_mug_and_sways_slowly() {
        let mut rng = Rng::new(5);
        for motion in Motion::ALL {
            let tea = Tea::new(80, 24, motion, &mut rng);
            assert!(tea.sway_period() >= SWAY_PERIOD);
            // a curl crosses a column twice a period
            assert!(2.0 / tea.sway_period() <= 3.0);
        }
        let mut tea = Tea::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut tea, 80, 24, 60);
        let c = TeaColours::new(theme::default_theme());
        assert!(pixels(&frame).contains(&c.mug));
        assert!(pixels(&frame).contains(&c.tea));
        let (left, _) = tea.mug_x();
        let top = tea.mug_top();
        assert!(tea.puffs.len() >= 3);
        for puff in &tea.puffs {
            assert!(puff.y < top as f32);
            assert!((puff.x - left as f32).abs() < 16.0);
        }
        // the steam shows: some pixel above the mug is neither wall nor stripe
        let wall = c.wall.steps().to_vec();
        let above_mug = (0..top / 2).any(|y| {
            (left..left + 12).any(|x| {
                let (a, b) = frame.get(x, y).unwrap().pixel_pair().unwrap();
                [a, b].iter().any(|p| {
                    !wall.contains(p) && !wall.iter().any(|&w| *p == mix(w, c.stripe, 0.4))
                })
            })
        });
        assert!(above_mug);
    }

    #[test]
    fn the_window_and_the_books_need_room() {
        let mut rng = Rng::new(1);
        let theme = theme::default_theme();
        let c = TeaColours::new(theme);
        let big = render(&mut Tea::new(80, 24, Motion::FULL, &mut rng), 80, 24, 20);
        let all = pixels(&big);
        assert!(all.contains(&c.glass_top) && all.contains(&c.curtain));
        assert!(c.books.iter().all(|b| all.contains(b)));
        assert!(all.contains(&c.leaf) && all.contains(&c.lamp));
        let small = render(&mut Tea::new(20, 8, Motion::FULL, &mut rng), 20, 8, 20);
        let all = pixels(&small);
        assert!(!all.contains(&c.glass_top));
        assert!(all.contains(&c.mug) && all.contains(&c.tea));
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = TeaColours::new(theme);
            let name = theme.name;
            for (what, colour) in [
                ("wall top", c.wall_top),
                ("wall low", c.wall_low),
                ("glass top", c.glass_top),
                ("glass low", c.glass_low),
                ("rooftops", c.roofs),
                ("rain", c.rain),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            apart(theme, c.wall_top, c.wall_low, 0.03, "the wall's gradient");
            apart(theme, c.glass_top, c.glass_low, 0.03, "the evening sky");
            apart(
                theme,
                c.roofs,
                c.glass_low,
                0.04,
                "the rooftops and the sky",
            );
            apart(theme, c.rain, c.glass_low, 0.04, "the rain and the sky");
            apart(theme, c.steam, c.wall_low, 0.04, "the steam and the wall");
            apart(theme, c.wood, c.wall_low, 0.1, "the table and the wall");
            apart(theme, c.mug, c.wall_low, 0.1, "the mug and the wall");
            apart(theme, c.mug, c.tea, 0.1, "the mug and the tea");
            apart(theme, c.mug, c.mug_band, 0.1, "the mug and its band");
            apart(theme, c.leaf, c.pot, 0.1, "the leaves and the pot");
            apart(
                theme,
                c.curtain,
                c.wall_top,
                0.1,
                "the curtains and the wall",
            );
            for colour in [c.mug, c.lamp, c.leaf, c.books[0], c.books[1], c.books[2]] {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
            }
            for book in c.books {
                apart(theme, book, c.wood, 0.08, "a book and the table");
            }
        }
    }
}
