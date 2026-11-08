//! Cat nap, painted in solid pixels: a ginger tabby curled up asleep on a plump cushion, lit
//! from the upper left, its back rising and falling slowly as it breathes and the tip of its
//! tail stirring now and then, with z's drifting up from it, a ball of wool on a patterned rug,
//! a night window above and, on a wide screen, a bookcase.
//!
//! Every colour comes from the active theme. The wall, the floor, the rug and the night sky are
//! tints of the background held below the non-text floor, so they read as background; the cat,
//! the cushion, the wool, the moon and the books are shaded from the theme's own drawing
//! colours, so they stand out as shapes and keep their outline when colour is off.

use super::count_for;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A painted cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Nothing to wake up for. The build is fine.",
    "Clean build. Back to sleep.",
    "Even the cat approves.",
    "All quiet. Do not disturb.",
    "Purring along nicely.",
];

/// Seconds one breath takes. The back changes shape twice a breath, once every two seconds at
/// the standard pace.
pub const BREATH_PERIOD: f32 = 4.0;
/// Seconds the tail takes to stir and settle again.
pub const TAIL_PERIOD: f32 = 7.0;
/// Seconds between two z's.
const Z_GAP: f32 = 3.0;
/// Seconds a z lasts.
const Z_LIFE: f32 = 5.0;
/// Steps of light across the fur.
const FUR_STEPS: usize = 6;

/// The cat asleep facing right, breathing out: `l` lit fur, `f` fur, `d` shaded fur, `s` a
/// tabby stripe, `p` the pale muzzle and paws, `e` the inside of an ear, `k` a closed eye, `n`
/// the nose and `t` and `T` the tail and its stripes.
const CAT_OUT: [&str; 14] = [
    ".........................d.....d..",
    "........................ld....fd..",
    "........................led..fed..",
    ".......................llllfffffd.",
    "..........lllffff.....llfffffffffd",
    ".......llllllfsffssf..lffffffffffd",
    ".....lllllfsfffsfffsfdlffffffffffd",
    "....llllfsfffsfffsfffdlfkkfffkkffd",
    "...lllfffsfffsfffsfffdffffffnffffd",
    "..llffffffffffffffffdddffppppppfdd",
    ".lfffffffffffffffffddddfpppppppdd.",
    "lffffffffffffffffdddddddppp.ppp...",
    "fffffffffffffffdddddddddppp.ppp...",
    "fdtttTtttTtttTtttTtttTtttd........",
];
/// The rows of the back that change while breathing in: the back rises by a pixel.
const CAT_IN_BACK: [&str; 3] = [
    "...........llfff.......llllfffffd.",
    "........llllllffff....llfffffffffd",
    "......lllllllfsffssff.lffffffffffd",
];
/// The first row `CAT_IN_BACK` replaces.
const BACK_ROW: usize = 3;
/// The pixel the tail tip flicks out to, beside the last pixel of the tail.
const TAIL_TIP: (i32, i32) = (26, 13);

/// A smaller cat for a small screen, in the same key.
const SMALL_OUT: [&str; 7] = [
    ".........d..d.",
    "........ld.fd.",
    "...lllf.lllffd",
    ".llfsfsdlkfkfd",
    "lfffsffdffnffd",
    "fffffffddfppfd",
    "fttTttTtdpp.pp",
];
const SMALL_IN_BACK: [&str; 2] = ["...ll...ld.fd.", "..llllf.lllffd"];
const SMALL_BACK_ROW: usize = 1;
const SMALL_TAIL_TIP: (i32, i32) = (-1, 5);

/// A small z and a large one, in pixels.
const Z_SMALL: [&str; 4] = ["###", "..#", ".#.", "###"];
const Z_LARGE: [&str; 5] = ["####", "...#", "..#.", ".#..", "####"];

#[derive(Clone, Debug)]
struct Zed {
    x: f32,
    y: f32,
    age: f32,
}

/// The room's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct NapColours {
    pub wall_top: Rgb,
    pub wall_low: Rgb,
    pub floor: Rgb,
    pub seam: Rgb,
    pub rug: Rgb,
    pub rug_border: Rgb,
    pub glass_top: Rgb,
    pub glass_low: Rgb,
    pub star: Rgb,
    pub moon: Rgb,
    /// A night window on a dark theme; a daylit one with a pale moon on a light theme.
    pub night: bool,
    pub wood: Rgb,
    pub wood_lit: Rgb,
    pub fur: Rgb,
    pub stripe: Rgb,
    pub pale: Rgb,
    pub ear: Rgb,
    pub eye: Rgb,
    pub nose: Rgb,
    pub cushion: Rgb,
    pub cushion_lit: Rgb,
    pub cushion_shade: Rgb,
    pub wool: Rgb,
    pub wool_thread: Rgb,
    pub zed: Rgb,
    pub books: [Rgb; 6],
    fur_lit: Ramp,
    wall: Ramp,
    glass: Ramp,
}

impl NapColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let room_hue = mix(theme.info, theme.muted, 0.5);
        // a dark room is lit low by the window and the floor, a light one pales downwards
        let (wall_top, wall_low) = if theme.dark {
            (
                shades.tint(room_hue, 0.1, 1.15),
                shades.tint(room_hue, 0.25, 1.4),
            )
        } else {
            (
                shades.tint(theme.warning, 0.3, 1.45),
                shades.tint(theme.warning, 0.12, 1.15),
            )
        };
        let (glass_top, glass_low) = if theme.dark {
            (theme.background, shades.tint(theme.blue(), 0.3, 1.45))
        } else {
            (
                shades.tint(theme.info, 0.6, 2.2),
                shades.tint(theme.info, 0.3, 1.6),
            )
        };
        let wood_hue = mix(theme.yellow(), theme.red(), 0.35);
        let floor = shades.tint(wood_hue, 0.4, 1.9);
        let fur = theme.yellow();
        let cushion = theme.magenta();
        let wood = shades.darker(wood_hue, 0.25);
        Self {
            wall_top,
            wall_low,
            floor,
            seam: shades.darker(floor, 0.25),
            rug: shades.tint(theme.red(), 0.5, 2.2),
            // a tint keeps no floor, so it takes the palette's own yellow rather than the
            // stronger colour a light theme with a 7 to 1 floor swaps in for text
            rug_border: shades.tint(theme.ansi[3], 0.5, 2.4),
            glass_top,
            glass_low,
            star: shades.tint(shades.lighter(glass_low, 0.6), 1.0, IMAGERY_CEILING - 0.2),
            // on a light theme the window shows a daytime sky, where the moon is a pale disc
            // and no stars show, rather than a dark crescent and dark specks
            moon: if theme.dark {
                theme.bright_yellow()
            } else {
                theme.background
            },
            night: theme.dark,
            wood,
            wood_lit: shades.lighter(wood, 0.25),
            fur,
            stripe: shades.darker(fur, 0.35),
            pale: shades.lighter(fur, 0.5),
            ear: mix(fur, theme.red(), 0.5),
            eye: shades.darker(fur, 0.75),
            nose: theme.red(),
            cushion,
            cushion_lit: shades.lighter(cushion, 0.3),
            cushion_shade: shades.darker(cushion, 0.35),
            wool: theme.cyan(),
            wool_thread: shades.darker(theme.cyan(), 0.35),
            zed: theme.muted,
            books: [
                theme.red(),
                theme.blue(),
                theme.green(),
                theme.yellow(),
                theme.magenta(),
                theme.cyan(),
            ],
            fur_lit: Ramp::new(shades.darker(fur, 0.3), shades.lighter(fur, 0.2), FUR_STEPS),
            wall: Ramp::new(wall_top, wall_low, 8),
            glass: Ramp::new(glass_top, glass_low, 6),
        }
    }
}

pub struct CatNap {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    zeds: Vec<Zed>,
    since_z: f32,
    stars: Vec<(i32, i32)>,
}

impl CatNap {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut nap = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            zeds: Vec::new(),
            since_z: 0.0,
            stars: Vec::new(),
        };
        nap.resize(width, height, rng);
        nap
    }

    fn breath_period(&self) -> f32 {
        BREATH_PERIOD / self.motion.pace()
    }

    fn tail_period(&self) -> f32 {
        TAIL_PERIOD / self.motion.pace()
    }

    /// Whether the cat is breathing in at the moment.
    fn breathing_in(&self) -> bool {
        (self.t / f64::from(self.breath_period())).rem_euclid(1.0) < 0.5
    }

    /// Whether the tail tip is lifted at the moment.
    fn stirring(&self) -> bool {
        (self.t / f64::from(self.tail_period())).rem_euclid(1.0) < 0.3
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The first pixel row of the floor.
    fn floor(&self) -> i32 {
        let tall = self.tall();
        if self.height >= 8 { tall - 4 } else { tall - 2 }
    }

    /// The top pixel row of the cushion, which is four sprite pixels deep.
    fn cushion_top(&self) -> i32 {
        self.floor() - 4 * self.scale() + 1
    }

    fn large(&self) -> bool {
        self.width >= 40 && self.cushion_top() >= CAT_OUT.len() as i32 + 4
    }

    /// How many pixels each sprite pixel of the cat covers: two on a large screen, so the cat
    /// keeps its place in the room.
    fn scale(&self) -> i32 {
        if self.width >= 110 && self.height >= 40 {
            2
        } else {
            1
        }
    }

    /// The sprite rows for the current size and breath.
    fn sprite(&self) -> Vec<&'static str> {
        let (rows, back, from): (&[&str], &[&str], usize) = if self.large() {
            (&CAT_OUT, &CAT_IN_BACK, BACK_ROW)
        } else {
            (&SMALL_OUT, &SMALL_IN_BACK, SMALL_BACK_ROW)
        };
        let mut out = rows.to_vec();
        if self.breathing_in() {
            out[from..from + back.len()].copy_from_slice(back);
        }
        out
    }

    /// The cat's top left pixel, its width and its height.
    fn cat_at(&self) -> (i32, i32, i32, i32) {
        let (w, h) = if self.large() {
            (CAT_OUT[0].len() as i32, CAT_OUT.len() as i32)
        } else {
            (SMALL_OUT[0].len() as i32, SMALL_OUT.len() as i32)
        };
        let (w, h) = (w * self.scale(), h * self.scale());
        let wide = self.width as i32;
        // the cushion reaches three pixels past the cat at either end, so the cat keeps that
        // much room from the edges
        let x = (wide * 2 / 5 - w / 2).min(wide - w - 3).max(3);
        (x, self.cushion_top() - h, w, h)
    }

    /// The window's left, top, width and height in pixels, when there is room for it.
    fn window(&self) -> Option<(i32, i32, i32, i32)> {
        let (_, cat_y, _, _) = self.cat_at();
        let height = (cat_y - 8).min(14 + 8 * self.scale());
        (self.width >= 36 && height >= 10).then(|| {
            (
                5,
                3,
                (self.width as i32 / 4).clamp(14, 26 * self.scale()),
                height,
            )
        })
    }

    fn paint_room(&self, canvas: &mut Canvas, c: &NapColours) {
        let floor = self.floor();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        for y in 0..floor {
            canvas.fill_row(y, c.wall.at(y as f32 / floor.max(1) as f32));
        }
        // floorboards with seams that never line up row to row
        for y in floor..tall {
            canvas.fill_row(y, c.floor);
            let offset = (y - floor) * 7;
            let mut x = offset % 13;
            while x < wide {
                canvas.set(x, y, c.seam);
                x += 13;
            }
        }
        // a rug under the cushion and the wool, with a border and a simple diamond pattern
        if self.height >= 8 {
            let left = (wide / 8).max(1);
            let right = wide - left;
            let top = floor + 1;
            canvas.fill_rect(left, top, right - left, tall - top, c.rug_border);
            canvas.fill_rect(left + 1, top + 1, right - left - 2, tall - top - 1, c.rug);
            for x in (left + 3)..(right - 3) {
                if (x - left) % 6 == 0 {
                    canvas.set(x, top + 2, c.rug_border);
                }
            }
        }
    }

    fn paint_window(&self, canvas: &mut Canvas, c: &NapColours) {
        let Some((left, top, w, h)) = self.window() else {
            return;
        };
        for y in 0..h {
            canvas.fill_rect(left, top + y, w, 1, c.glass.at(y as f32 / h as f32));
        }
        if c.night {
            for &(x, y) in &self.stars {
                canvas.set(x, y, c.star);
            }
        }
        // a still crescent moon in the upper right pane
        let (mx, my) = (left + w - 7, top + 6);
        for dy in -4_i32..=4 {
            for dx in -4_i32..=4 {
                let (fx, fy) = (dx as f32, dy as f32);
                let lit = fx * fx + fy * fy <= 16.0;
                let bitten = (fx - 2.2).powi(2) + (fy + 1.2).powi(2) <= 14.0;
                if lit && !bitten {
                    canvas.set(mx + dx, my + dy, c.moon);
                }
            }
        }
        canvas.fill_rect(left - 2, top - 2, w + 4, 2, c.wood_lit);
        canvas.fill_rect(left - 2, top, 2, h, c.wood);
        canvas.fill_rect(left + w, top, 2, h, c.wood);
        canvas.fill_rect(left + w / 2, top, 1, h, c.wood);
        canvas.fill_rect(left, top + h / 2, w, 1, c.wood);
        canvas.fill_rect(left - 3, top + h, w + 6, 1, c.wood_lit);
        canvas.fill_rect(left - 3, top + h + 1, w + 6, 1, c.wood);
    }

    /// A bookcase against the right wall, on a screen wide enough to keep it clear of the cat.
    fn paint_shelf(&self, canvas: &mut Canvas, c: &NapColours) {
        let wide = self.width as i32;
        if wide < 70 || self.height < 16 {
            return;
        }
        let floor = self.floor();
        let left = wide - 24;
        let right = wide - 5;
        let shelves = ((floor - 10) / 9).min(4);
        if shelves < 1 {
            return;
        }
        let top = floor - shelves * 9 - 1;
        canvas.fill_rect(left, top, right - left + 1, floor - top, c.wood);
        canvas.fill_rect(
            left + 1,
            top + 1,
            right - left - 1,
            floor - top - 1,
            c.wall_top,
        );
        for shelf in 0..shelves {
            let base = top + 9 * (shelf + 1);
            canvas.fill_rect(left, base, right - left + 1, 1, c.wood_lit);
            // books of a fixed pattern per shelf, so the case looks lived in but never changes
            let mut x = left + 2;
            let mut i = shelf as usize * 3;
            while x < right - 2 {
                let tall = 5 + (i * 5 % 3) as i32;
                let thick = 1 + (i % 3 == 0) as i32;
                let colour = c.books[(i * 7 + shelf as usize) % c.books.len()];
                canvas.fill_rect(x, base - tall, thick, tall, colour);
                // a band across the spine, as a printed title would sit
                canvas.fill_rect(x, base - tall + 1, thick, 1, mix(colour, c.wall_top, 0.4));
                x += thick + if i % 5 == 4 { 2 } else { 0 };
                i += 1;
            }
        }
        canvas.fill_rect(left, top, right - left + 1, 1, c.wood_lit);
    }

    fn paint_cushion(&self, canvas: &mut Canvas, c: &NapColours) {
        let (x, _, w, _) = self.cat_at();
        let top = self.cushion_top();
        let k = self.scale();
        let (left, right) = (x - 3 * k, x + w + 3 * k - 1);
        for (dy, colour) in [c.cushion_lit, c.cushion, c.cushion, c.cushion_shade]
            .iter()
            .enumerate()
        {
            // rounded ends: the top and bottom rows are a little shorter at each end
            let inset = if dy == 0 || dy == 3 { k } else { 0 };
            canvas.fill_rect(
                left + inset,
                top + dy as i32 * k,
                right - left + 1 - 2 * inset,
                k,
                *colour,
            );
        }
        // tufted buttons along the middle
        let mut bx = left + 3 * k;
        while bx < right - 2 * k {
            canvas.fill_rect(bx, top + 2 * k, k, k, c.cushion_shade);
            bx += 6 * k;
        }
    }

    fn paint_cat(&self, canvas: &mut Canvas, c: &NapColours) {
        let (x, y, _, _) = self.cat_at();
        let palette = [
            ('l', c.fur_lit.at(1.0)),
            ('f', c.fur),
            ('d', c.fur_lit.at(0.0)),
            ('s', c.stripe),
            ('p', c.pale),
            ('e', c.ear),
            ('k', c.eye),
            ('n', c.nose),
            ('t', c.fur),
            ('T', c.stripe),
        ];
        let k = self.scale();
        for (dy, row) in self.sprite().iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if let Some(&(_, colour)) = palette.iter().find(|(key, _)| *key == ch) {
                    canvas.fill_rect(x + dx as i32 * k, y + dy as i32 * k, k, k, colour);
                }
            }
        }
        // the tip of the tail flicks out a pixel now and then
        if self.stirring() {
            let (tx, ty) = if self.large() {
                TAIL_TIP
            } else {
                SMALL_TAIL_TIP
            };
            canvas.fill_rect(x + tx * k, y + ty * k, k, k, c.stripe);
        }
    }

    fn paint_wool(&self, canvas: &mut Canvas, c: &NapColours) {
        if self.width < 40 {
            return;
        }
        let (x, _, _, _) = self.cat_at();
        let k = self.scale();
        let r = 3 * k;
        let cx = x - 4 * k - r;
        let cy = self.floor() - k;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r + k {
                    continue;
                }
                // the wound thread shows as diagonal bands
                let thread = ((dx + dy) / k).rem_euclid(3) == 0;
                canvas.set(
                    cx + dx,
                    cy + dy,
                    if thread { c.wool_thread } else { c.wool },
                );
            }
        }
        // a loose end trailing across the rug towards the cushion
        let bottom = canvas.height() as i32 - 1;
        for i in 0..6 * k {
            let ty = (cy + r - ((i / k % 4) == 1) as i32).min(bottom);
            canvas.set(cx + r + 1 + i, ty, c.wool);
        }
    }

    fn paint_zeds(&self, canvas: &mut Canvas, c: &NapColours) {
        for z in &self.zeds {
            let sprite: &[&str] = if z.age > Z_LIFE * 0.35 {
                &Z_LARGE
            } else {
                &Z_SMALL
            };
            // a z fades towards the wall over its last quarter
            let fade = if z.age > Z_LIFE * 0.75 { 0.5 } else { 1.0 };
            let x0 = z.x.round() as i32;
            let y0 = z.y.round() as i32;
            let k = self.scale();
            for (dy, row) in sprite.iter().enumerate() {
                for (dx, ch) in row.chars().enumerate() {
                    if ch != '#' {
                        continue;
                    }
                    for (px, py) in (0..k).flat_map(|i| (0..k).map(move |j| (i, j))) {
                        let (zx, zy) = (x0 + dx as i32 * k + px, y0 + dy as i32 * k + py);
                        canvas.blend(zx, zy, c.zed, fade);
                    }
                }
            }
        }
    }
}

impl Scene for CatNap {
    fn name(&self) -> &'static str {
        "cat-nap"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.zeds.clear();
        self.stars.clear();
        if let Some((left, top, w, h)) = self.window() {
            let wanted = count_for(w as usize, h as usize, 24, 2, 30);
            self.stars = (0..wanted)
                .map(|_| {
                    (
                        left + rng.below(w as usize) as i32,
                        top + rng.below((h / 2).max(1) as usize) as i32,
                    )
                })
                .collect();
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.since_z += dt;
        if self.since_z >= Z_GAP / self.motion.pace() {
            self.since_z = 0.0;
            let (x, y, w, _) = self.cat_at();
            self.zeds.push(Zed {
                x: (x + w - 4 * self.scale()) as f32,
                y: (y - 6 * self.scale()) as f32,
                age: 0.0,
            });
        }
        for z in &mut self.zeds {
            z.age += dt;
            z.x += 0.45 * speed * dt;
            z.y -= 1.2 * speed * dt;
        }
        self.zeds.retain(|z| z.age < Z_LIFE && z.y >= -4.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = NapColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint_room(&mut canvas, &colours);
        self.paint_window(&mut canvas, &colours);
        self.paint_shelf(&mut canvas, &colours);
        self.paint_cushion(&mut canvas, &colours);
        self.paint_cat(&mut canvas, &colours);
        self.paint_wool(&mut canvas, &colours);
        self.paint_zeds(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, max_changes_per_second, pixels,
        render,
    };
    use crate::theme;

    fn drawn(nap: &CatNap, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        nap.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("cat-nap");
        check_determinism("cat-nap");
        check_pixel_scene("cat-nap", 6.0);
    }

    #[test]
    fn the_cat_breathes_slowly_and_its_tail_stirs_rarely() {
        let mut rng = Rng::new(3);
        for motion in Motion::ALL {
            let nap = CatNap::new(80, 24, motion, &mut rng);
            assert!(nap.breath_period() >= BREATH_PERIOD);
            assert!(nap.tail_period() >= TAIL_PERIOD);
        }
        let mut nap = CatNap::new(80, 24, Motion::FULL, &mut rng);
        let mut shapes = Vec::new();
        for _ in 0..(BREATH_PERIOD as usize * 20) {
            nap.step(0.05, &mut rng);
            shapes.push(nap.breathing_in());
        }
        let changes = shapes.windows(2).filter(|w| w[0] != w[1]).count();
        assert!((1..=2).contains(&changes), "{changes}");
        // the cat's own cells change no more than the breath and the tail allow
        let mut nap = CatNap::new(60, 20, Motion::FULL, &mut rng);
        let frames: Vec<Frame> = (0..200)
            .map(|_| {
                nap.step(0.05, &mut rng);
                nap.zeds.clear();
                drawn(&nap, 60, 20, theme::default_theme())
            })
            .collect();
        assert!(max_changes_per_second(&frames, 20) <= 2);
    }

    #[test]
    fn a_z_drifts_up_from_the_cat_and_the_cushion_fits() {
        let theme = theme::default_theme();
        let c = NapColours::new(theme);
        let mut rng = Rng::new(3);
        let mut nap = CatNap::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut nap, 60, 20, 100);
        assert!(!nap.zeds.is_empty());
        let (x, y, _, _) = nap.cat_at();
        for z in &nap.zeds {
            assert!(z.y < y as f32 && z.x > x as f32);
        }
        let all = pixels(&frame);
        for colour in [c.fur, c.stripe, c.eye, c.nose, c.cushion, c.wool] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        // on the smallest screen the whole cushion fits with both ends included, as does the
        // cat
        let small = CatNap::new(20, 8, Motion::CALM, &mut rng);
        let frame = drawn(&small, 20, 8, theme);
        let (x, y, w, _) = small.cat_at();
        assert!(x - 3 >= 0 && x + w + 2 < 20 && y >= 0, "{x} {y} {w}");
        let all = pixels(&frame);
        assert!(all.contains(&c.cushion) && all.contains(&c.eye));
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = NapColours::new(theme);
            let name = theme.name;
            for (what, colour) in [
                ("wall top", c.wall_top),
                ("wall low", c.wall_low),
                ("floor", c.floor),
                ("rug", c.rug),
                ("rug border", c.rug_border),
                ("glass", c.glass_low),
                ("star", c.star),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            apart(theme, c.wall_top, c.wall_low, 0.03, "the wall's gradient");
            apart(theme, c.floor, c.wall_low, 0.04, "the floor and the wall");
            apart(theme, c.rug, c.rug_border, 0.04, "the rug and its border");
            apart(theme, c.rug, c.floor, 0.03, "the rug and the floor");
            apart(theme, c.star, c.glass_top, 0.05, "the stars and the sky");
            apart(theme, c.fur, c.cushion, 0.1, "the cat and the cushion");
            apart(theme, c.fur, c.stripe, 0.05, "the fur and its stripes");
            apart(theme, c.fur, c.eye, 0.1, "the fur and the eyes");
            apart(theme, c.fur, c.wall_low, 0.1, "the cat and the wall");
            apart(theme, c.cushion, c.rug, 0.1, "the cushion and the rug");
            apart(theme, c.wool, c.rug, 0.1, "the wool and the rug");
            apart(theme, c.zed, c.wall_top, 0.1, "a z and the wall");
            for colour in [c.fur, c.cushion, c.wool, c.zed] {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
            }
            if theme.dark {
                assert!(theme.contrast_of(c.moon) >= theme.floor(), "{name}");
            } else {
                apart(theme, c.moon, c.glass_top, 0.05, "the day moon and the sky");
            }
        }
    }
}
