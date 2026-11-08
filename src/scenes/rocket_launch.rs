//! Rocket launch: a launch pad on a coast at dusk, the sea behind it running out to a far
//! headland. A rocket rolls in upright on its tracked platform and stops beside a lattice
//! gantry, whose access arm swings in to it and away again as a board on the apron counts down.
//! At zero the engine lights: a glow swells slowly under the rocket and a cloud of smoke billows
//! out along the ground. The rocket lifts off slowly at first and climbs out of sight on a long
//! trail that drifts with the wind and thins away. The empty platform rolls off and the next
//! rocket rolls in.
//!
//! The sky, the sea, the beach, the apron, the smoke and the glow are tints of the background
//! held below the non-text floor. The rocket, its platform, the gantry, the flame and the board's
//! frame are solid, so with colour off the rocket and its tower still read. Nothing flashes: the
//! glow and the flame grow over four seconds and the flame's waver takes three.
//!
//! The pad, its colours and its painting are shared with `launch_scrub`, so the pair match.

use super::cycle;
use super::paint::{self, Cloud, ease, noise, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

pub const SUMMARY: &str = "A rocket counting down on a coastal pad at dusk, lifting off in a billow of smoke on a long trail, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Lift-off. The build cleared the tower.",
    "All systems go.",
    "A clean launch. Every stage passed.",
];

// the launch runs on its own clock, which keeps the motion pace, so a lively setting never
// counts down faster than the standard one

/// Seconds for the platform to roll the rocket in to the pad.
pub const ROLL_IN: f32 = 14.0;
/// When the access arm swings in to the rocket and when it has arrived.
const ARM_IN: (f32, f32) = (14.0, 18.0);
/// When the countdown starts, ten seconds before ignition.
pub const COUNT: f32 = 20.0;
/// When the arm swings away again before ignition.
const ARM_OUT: (f32, f32) = (24.0, 27.0);
/// When the engine lights.
pub const IGNITE: f32 = 30.0;
/// When the rocket leaves the pad, after the glow has swelled for four seconds.
pub const LIFT: f32 = 34.0;
/// Seconds from lift-off until the rocket is out of sight.
pub const CLIMB: f32 = 16.0;
/// Seconds for the trail and the cloud to thin away once the rocket has gone.
const FADE: f32 = 22.0;
/// When the empty platform rolls off the pad and when it is out of sight.
const ROLL_OUT: (f32, f32) = (64.0, 76.0);
/// One whole launch, after which the next rocket is rolling in.
pub const CYCLE: f32 = 76.0;
/// Where on the clock the scene opens: just before lift-off, so the first look shows it.
const START: f32 = 35.0;
/// Seconds for one slow waver of the flame, long enough that it never flickers.
pub const FLAME_PERIOD: f32 = 3.0;

/// The pad's colours in one theme.
pub(crate) struct Colours {
    pub sky: Ramp,
    pub star: Rgb,
    pub cloud: (Rgb, Rgb),
    pub headland: Rgb,
    pub sea: Ramp,
    pub glint: Rgb,
    pub surf: Rgb,
    pub sand: Rgb,
    pub apron: Ramp,
    pub seam: Rgb,
    /// Smoke and steam from the lit side to the shaded side.
    pub smoke: [Rgb; 3],
    pub glow: Rgb,
    pub glow_far: Rgb,
    /// The gantry's steel from shaded to lit.
    pub steel: [Rgb; 3],
    /// The rocket's body from its shaded side to its highlight.
    pub hull: [Rgb; 4],
    /// The nose, the band and the fins, shaded and lit.
    pub trim: (Rgb, Rgb),
    pub bell: Rgb,
    /// The platform from shaded to lit.
    pub deck: [Rgb; 3],
    pub tread: Rgb,
    /// The flame's outer body and its core.
    pub flame: (Rgb, Rgb),
    pub beacon: Rgb,
    pub face: Rgb,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let t = |hue: Rgb, strength: f32, ceiling: f64| shades.tint(hue, strength, ceiling);
        let dusk = mix(theme.info, theme.warning, 0.35);
        let sea_hue = mix(theme.blue(), theme.info, 0.3);
        let land = mix(theme.muted, theme.green(), 0.3);
        let sand_hue = mix(theme.yellow(), theme.muted, 0.35);
        // a dark theme's launch is at dusk, the sky warming towards the horizon over a dark
        // sea; a light theme's is a clear afternoon, a pale sky over a deeper sea
        let (sky_top, sky_low, sea_far, sea_near, glint, headland, sand, surf) = if theme.dark {
            (
                t(mix(theme.blue(), theme.info, 0.5), 0.12, 1.3),
                t(dusk, 0.42, 1.95),
                t(sea_hue, 0.34, 1.7),
                t(sea_hue, 0.2, 1.4),
                t(mix(dusk, theme.foreground, 0.4), 0.55, 2.35),
                t(land, 0.16, 1.3),
                t(sand_hue, 0.32, 1.65),
                t(theme.foreground, 0.5, 2.4),
            )
        } else {
            (
                t(theme.info, 0.42, 1.75),
                t(mix(theme.info, theme.warning, 0.15), 0.1, 1.12),
                t(sea_hue, 0.48, 1.95),
                t(sea_hue, 0.62, 2.35),
                t(theme.info, 0.06, 1.08),
                t(land, 0.5, 2.05),
                t(sand_hue, 0.45, 1.55),
                t(theme.info, 0.02, 1.03),
            )
        };
        let (apron_far, apron_near, seam) = if theme.dark {
            (
                t(theme.muted, 0.2, 1.4),
                t(theme.muted, 0.3, 1.65),
                t(theme.muted, 0.14, 1.28),
            )
        } else {
            (
                t(theme.muted, 0.22, 1.35),
                t(theme.muted, 0.32, 1.55),
                t(theme.muted, 0.4, 1.75),
            )
        };
        // smoke and the rocket's paint are white whatever colour the theme's text is
        let white = mix(theme.bright(), theme.muted, 0.15);
        let smoke = if theme.dark {
            [
                t(white, 0.72, 2.85),
                t(white, 0.55, 2.45),
                t(mix(theme.muted, white, 0.4), 0.4, 1.95),
            ]
        } else {
            [
                t(theme.info, 0.02, 1.03),
                t(theme.muted, 0.18, 1.32),
                t(theme.muted, 0.42, 1.9),
            ]
        };
        let (glow, glow_far) = if theme.dark {
            (t(theme.warning, 0.62, 2.7), t(theme.warning, 0.34, 2.0))
        } else {
            (t(theme.warning, 0.34, 1.7), t(theme.warning, 0.16, 1.28))
        };
        let steel_hue = mix(theme.muted, theme.info, 0.2);
        let hull_hue = mix(theme.muted, white, 0.75);
        let deck_hue = mix(theme.muted, theme.foreground, 0.15);
        let flame = if theme.dark {
            (
                paint::strong(theme, mix(theme.warning, theme.red(), 0.35)),
                paint::strong(theme, theme.bright_yellow()),
            )
        } else {
            // in daylight the flame's core is a pale glare inside its solid edge
            (
                paint::strong(theme, mix(theme.red(), theme.warning, 0.5)),
                t(mix(theme.yellow(), theme.warning, 0.5), 0.35, 1.6),
            )
        };
        Self {
            sky: Ramp::new(sky_top, sky_low, 8),
            star: t(theme.foreground, 0.45, 2.2),
            cloud: paint::cloud_tones(theme, 2.6),
            headland,
            sea: Ramp::new(sea_far, sea_near, 5),
            glint,
            surf,
            sand,
            apron: Ramp::new(apron_far, apron_near, 4),
            seam,
            smoke,
            glow,
            glow_far,
            steel: [
                tone(theme, steel_hue, 0.1),
                tone(theme, steel_hue, 0.4),
                tone(theme, steel_hue, 0.75),
            ],
            hull: [
                tone(theme, hull_hue, 0.3),
                tone(theme, hull_hue, 0.58),
                tone(theme, hull_hue, 0.82),
                tone(theme, hull_hue, 1.0),
            ],
            trim: (
                tone(theme, theme.red(), 0.25),
                tone(theme, theme.red(), 0.7),
            ),
            bell: tone(theme, theme.muted, 0.05),
            deck: [
                tone(theme, deck_hue, 0.0),
                tone(theme, deck_hue, 0.22),
                tone(theme, deck_hue, 0.5),
            ],
            tread: tone(theme, theme.muted, 0.0),
            flame,
            beacon: error_colour(theme),
            face: theme.background,
        }
    }

    /// Every colour that is background imagery.
    #[cfg(test)]
    pub(crate) fn imagery(&self) -> Vec<(&'static str, Rgb)> {
        let mut out = vec![
            ("star", self.star),
            ("cloud", self.cloud.0),
            ("cloud underside", self.cloud.1),
            ("headland", self.headland),
            ("glint", self.glint),
            ("surf", self.surf),
            ("sand", self.sand),
            ("seam", self.seam),
            ("glow", self.glow),
            ("far glow", self.glow_far),
        ];
        for c in self
            .sky
            .steps()
            .iter()
            .chain(self.sea.steps())
            .chain(self.apron.steps())
            .chain(&self.smoke)
        {
            out.push(("backdrop", *c));
        }
        out
    }

    /// Every colour that is a solid shape.
    #[cfg(test)]
    pub(crate) fn shapes(&self) -> Vec<(&'static str, Rgb)> {
        let mut out = vec![
            ("nose", self.trim.0),
            ("band", self.trim.1),
            ("bell", self.bell),
            ("tread", self.tread),
            ("flame", self.flame.0),
            ("beacon", self.beacon),
        ];
        for c in self.steel.iter().chain(&self.hull).chain(&self.deck) {
            out.push(("metal", *c));
        }
        out
    }
}

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight.
/// The lit side moves towards the theme's light end on a dark theme and towards the background
/// on a light one, never falling below the solid contrast, so the shape keeps its outline.
pub(crate) fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.2
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

/// One round puff of smoke or steam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Puff {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub seed: u32,
}

/// Paints puffs as one billowing mass. Each pixel takes the puff it lies deepest inside, so
/// neighbouring puffs meet in the lumpy edges of a real cloud rather than a smooth blob, and
/// each puff's own upper left catches the light. The mass as a whole is lit along its top and
/// shaded along its underside, so it reads as soft smoke rather than a pile of stones.
pub(crate) fn paint_puffs(canvas: &mut Canvas, puffs: &[Puff], tones: &[Rgb; 3]) {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    // how deep inside its puff each pixel lies and whether that puff lights it
    let mut best: Vec<(f32, u8)> = vec![(0.0, 0); (w * h).max(0) as usize];
    let mut any = false;
    for p in puffs {
        if p.r < 0.7 {
            continue;
        }
        // a small puff keeps a round edge, a large one a ragged one
        let wobble = 0.34 * ((p.r - 1.5) / 3.0).clamp(0.0, 1.0);
        let reach = (p.r * (1.0 + wobble)).ceil() as i32 + 1;
        let (cx, cy) = (p.x.round() as i32, p.y.round() as i32);
        for y in (cy - reach).max(0)..=(cy + reach).min(h - 1) {
            for x in (cx - reach).max(0)..=(cx + reach).min(w - 1) {
                let (dx, dy) = (x as f32 + 0.5 - p.x, y as f32 + 0.5 - p.y);
                let d = (dx * dx + dy * dy).sqrt();
                let edge = p.r
                    * (1.0 - wobble / 2.0
                        + wobble * noise(x as f32 * 0.4, y as f32 * 0.4, p.seed % 997));
                if d >= edge {
                    continue;
                }
                let v = 1.0 - d / edge;
                let i = (y * w + x) as usize;
                if v > best[i].0 {
                    let light = -(dx * 0.55 + dy * 0.85) / p.r.max(1.0);
                    best[i] = (v, if light > 0.2 { 1 } else { 2 });
                    any = true;
                }
            }
        }
    }
    if !any {
        return;
    }
    let smoke = |x: i32, y: i32| -> bool {
        x < 0 || y < 0 || x >= w || y >= h || best[(y * w + x) as usize].1 > 0
    };
    for y in 0..h {
        for x in 0..w {
            let tone = best[(y * w + x) as usize].1;
            if tone == 0 {
                continue;
            }
            let colour = if !smoke(x, y - 1) || !smoke(x - 1, y - 2) {
                tones[0]
            } else if !smoke(x, y + 1) || !smoke(x + 1, y + 2) {
                tones[2]
            } else {
                tones[usize::from(tone) - 1]
            };
            canvas.set(x, y, colour);
        }
    }
}

/// Where everything on the pad sits at one size, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pad {
    pub w: i32,
    pub h: i32,
    pub horizon: i32,
    pub shore: i32,
    /// The first row of the apron.
    pub ground: i32,
    /// The row the platform's tracks and the gantry's legs stand on.
    pub foot: i32,
    /// The rocket's centre column on the pad.
    pub px: i32,
    pub rw: i32,
    pub rh: i32,
    pub nose: i32,
    pub bell: i32,
    pub deck_top: i32,
    pub deck_w: i32,
    pub tread: i32,
    /// The gantry's left leg, its width and its top row.
    pub gx: i32,
    pub gw: i32,
    pub gtop: i32,
    pub arm_y: i32,
    /// The board's first text column and its text row, in cells, if there is room for it.
    pub board: Option<(i32, i32)>,
}

/// The board's text is centred in a face this many cells wide.
pub(crate) const BOARD_CELLS: i32 = 8;

impl Pad {
    pub(crate) fn new(columns: usize, rows: usize) -> Self {
        let w = columns.max(1) as i32;
        let h = rows.max(1) as i32 * 2;
        let horizon = (h as f32 * 0.56).round() as i32;
        let shore = (h as f32 * 0.7).round() as i32;
        let ground = shore + (h / 40).max(1);
        let foot = ground + ((h - ground) as f32 * 0.55).round() as i32;
        let deck_h = (h / 16).clamp(2, 7);
        let deck_top = foot - deck_h;
        let tread = (deck_h / 3).max(1);
        let margin = ((h as f32 * 0.08).round() as i32).max(2);
        let rough = ((deck_top - margin) as f32 * 0.84).round() as i32;
        let rw = ((rough as f32 / 6.5).round() as i32).clamp(3, 15) | 1;
        let bell = (rw / 3).clamp(1, 4);
        let rh = (rough - bell).max(6);
        let nose = ((rw as f32 * 1.7).round() as i32).min(rh / 3).max(2);
        let px = (w as f32 * 0.5).round() as i32;
        let gw = ((rw as f32 * 0.8).round() as i32).clamp(3, 11);
        let gx = px + rw / 2 + 1 + rw.max(3);
        let top = deck_top - bell - rh;
        let board = (columns >= 50 && rows >= 14).then(|| {
            let stand = ground + ((h - ground) as f32 * 0.82).round() as i32;
            let legs = (h / 14).max(2);
            let row = (stand - legs - 4) / 2;
            let col = (w as f32 * 0.2).round() as i32 - BOARD_CELLS / 2;
            (col.max(2), row)
        });
        Self {
            w,
            h,
            horizon,
            shore,
            ground,
            foot,
            px,
            rw,
            rh,
            nose,
            bell,
            deck_top,
            deck_w: rw * 3 + 4,
            tread,
            gx,
            gw,
            gtop: top - (rh / 12).max(2),
            arm_y: top + nose + (rh / 9).max(1),
            board,
        }
    }

    /// The bottom row of the rocket's body when it stands on the pad.
    pub(crate) fn body_bottom(&self) -> i32 {
        self.deck_top - self.bell
    }

    /// How far the rocket must climb to be out of sight, flame and all.
    pub(crate) fn climb(&self) -> f32 {
        (self.body_bottom() + self.rw * 4) as f32
    }
}

/// The parts of a moment on the pad that change, so both scenes paint through one function.
#[derive(Clone, Debug, Default)]
pub(crate) struct Moment {
    /// The platform's centre column.
    pub platform: f32,
    /// How far the rocket has climbed off its platform, if it is in sight.
    pub rocket: Option<f32>,
    /// How far the access arm reaches towards the rocket, from 0.25 swung away to 1 on it.
    pub arm: f32,
    /// How far the engine has lit, from 0 to 1.
    pub burn: f32,
    /// The flame's slow waver, from -1 to 1.
    pub waver: f32,
    pub puffs: Vec<Puff>,
    /// How far the sea's glints and the surf have drifted.
    pub drift: f32,
}

/// Paints the sky, the sea, the beach and the apron.
pub(crate) fn paint_setting(
    canvas: &mut Canvas,
    pad: &Pad,
    c: &Colours,
    theme: &ThemeData,
    clouds: &[Cloud],
    drift: f32,
) {
    let (w, h) = (pad.w, pad.h);
    paint::gradient(canvas, &c.sky, 0, pad.horizon);
    if theme.dark {
        // a few early stars in the darker upper sky, fixed in place
        let top = (pad.horizon as f32 * 0.6) as i32;
        for y in 0..top {
            for x in 0..w {
                if unit(x, y, 91) < 0.012 {
                    canvas.set(x, y, c.star);
                }
            }
        }
    }
    for cloud in clouds {
        cloud.paint(canvas, c.cloud.0, c.cloud.1);
    }
    // a low headland far across the water on the left
    let reach = w as f32 * 0.3;
    let rise = (h as f32 * 0.05).max(1.0);
    for x in 0..reach as i32 {
        let k = x as f32 / reach;
        let lift = rise * (1.0 - k * k) * (0.75 + 0.25 * noise(x as f32 / 7.0, 0.0, 12));
        let top = pad.horizon - lift.round() as i32;
        canvas.fill_rect(x, top, 1, pad.horizon - top, c.headland);
    }
    // the sea, deeper towards the shore, with glints drifting slowly along it
    let span = (pad.shore - pad.horizon).max(1) as f32;
    for y in pad.horizon..pad.shore {
        let v = (y - pad.horizon) as f32 / span;
        canvas.fill_row(y, c.sea.at(v));
        let scale = 1.6 + v * 3.0;
        for x in 0..w {
            let n = noise((x as f32 + drift * (0.6 + v)) / scale, y as f32 * 1.7, 3);
            if n > 0.86 {
                canvas.set(x, y, c.glint);
            }
        }
    }
    // the beach and the surf along it, the foam sliding slowly with the swell
    for y in pad.shore..pad.ground {
        canvas.fill_row(y, c.sand);
    }
    for x in 0..w {
        if noise((x as f32 - drift * 0.5) / 5.0, 0.5, 8) > 0.42 {
            canvas.set(x, pad.shore, c.surf);
        }
    }
    // the apron, a little lighter towards the viewer, laid in slabs that widen as they near
    let span = (h - pad.ground).max(1) as f32;
    for y in pad.ground..h {
        let v = (y - pad.ground) as f32 / span;
        canvas.fill_row(y, c.apron.at(v));
    }
    let mut y = pad.ground + 3;
    let mut gap = 4;
    while y < h {
        canvas.fill_row(y, c.seam);
        y += gap;
        gap += 3;
    }
    canvas.fill_row(pad.ground, c.seam);
}

/// The lattice gantry beside the pad, with its cab, mast and steady beacon.
pub(crate) fn paint_gantry(canvas: &mut Canvas, pad: &Pad, c: &Colours) {
    let (x0, x1) = (pad.gx, pad.gx + pad.gw - 1);
    let (top, bottom) = (pad.gtop, pad.foot);
    // a zigzag of single braces between the levels keeps the lattice open to the sky
    let seg = (pad.gw * 3 / 2).max(3);
    let mut y = bottom;
    let mut flip = false;
    while y > top {
        let y2 = (y - seg).max(top);
        if flip {
            canvas.line((x0, y2), (x1, y), c.steel[0]);
        } else {
            canvas.line((x1, y2), (x0, y), c.steel[0]);
        }
        canvas.fill_rect(x0, y2, pad.gw, 1, c.steel[1]);
        flip = !flip;
        y = y2;
    }
    canvas.fill_rect(x0, top, 1, bottom - top, c.steel[2]);
    canvas.fill_rect(x1, top, 1, bottom - top, c.steel[1]);
    // the cab on top, then the mast with its beacon held steady, never blinking
    let cab = (pad.gw / 3).max(2);
    canvas.fill_rect(x0 - 1, top - cab, pad.gw + 2, cab, c.steel[1]);
    canvas.fill_rect(x0 - 1, top - cab, pad.gw + 2, 1, c.steel[2]);
    let mid = x0 + pad.gw / 2;
    let mast = (pad.rh / 6).max(2);
    canvas.fill_rect(mid, top - cab - mast, 1, mast, c.steel[1]);
    canvas.set(mid, top - cab - mast - 1, c.beacon);
}

/// The access arm from the gantry towards the rocket's upper stage.
pub(crate) fn paint_arm(canvas: &mut Canvas, pad: &Pad, c: &Colours, reach: f32) {
    let start = pad.gx - 1;
    let full = (start - (pad.px + pad.rw / 2)).max(1);
    let len = (full as f32 * reach.clamp(0.0, 1.0)).round() as i32;
    let thick = (pad.rw / 4).clamp(2, 4);
    let y = pad.arm_y;
    canvas.fill_rect(start - len + 1, y, len, 1, c.steel[2]);
    canvas.fill_rect(start - len + 1, y + thick - 1, len, 1, c.steel[0]);
    let mut x = start;
    while x > start - len {
        canvas.fill_rect(x, y, 1, thick, c.steel[1]);
        x -= 2;
    }
    // the hood at the end closes over the hatch once the arm is home
    if reach > 0.93 {
        let end = start - len + 1;
        canvas.fill_rect(end, y - 1, 2, thick + 2, c.steel[2]);
    }
}

/// The tracked platform the rocket stands on, its wheels turning as it rolls.
pub(crate) fn paint_platform(canvas: &mut Canvas, pad: &Pad, c: &Colours, centre: f32) {
    let left = (centre - pad.deck_w as f32 / 2.0).round() as i32;
    let tread_top = pad.foot - pad.tread;
    canvas.fill_rect(
        left,
        pad.deck_top,
        pad.deck_w,
        tread_top - pad.deck_top,
        c.deck[1],
    );
    canvas.fill_rect(left, pad.deck_top, pad.deck_w, 1, c.deck[2]);
    canvas.fill_rect(
        left + pad.deck_w - 1,
        pad.deck_top,
        1,
        tread_top - pad.deck_top,
        c.deck[0],
    );
    let inset = (pad.tread + 1).min(pad.deck_w / 4);
    canvas.fill_rect(
        left + inset,
        tread_top,
        pad.deck_w - 2 * inset,
        pad.tread,
        c.tread,
    );
    let turn = left.rem_euclid(3);
    for x in left + inset..left + pad.deck_w - inset {
        if (x - turn).rem_euclid(3) == 0 {
            canvas.set(x, pad.foot - 1, c.deck[1]);
        }
    }
}

/// The rocket with its body bottom at `bottom`: an ogive nose, a red fairing band, a stage
/// line, swept fins and the engine bell, lit from the left.
pub(crate) fn paint_rocket(canvas: &mut Canvas, pad: &Pad, c: &Colours, cx: i32, bottom: i32) {
    let (rw, rh, nose) = (pad.rw, pad.rh, pad.nose);
    let top = bottom - rh;
    let half_w = rw as f32 / 2.0;
    let centre = cx as f32 + 0.5;
    let band = (rw / 3).max(1);
    let stage = top + (rh as f32 * 0.4).round() as i32;
    for y in top..bottom {
        let r = y - top;
        let half = if r < nose {
            half_w * ((r as f32 + 0.6) / nose as f32).powf(0.5)
        } else {
            half_w
        };
        let (x0, x1) = (
            (centre - half).round() as i32,
            (centre + half).round() as i32,
        );
        let x1 = x1.max(x0 + 1);
        let width = (x1 - x0) as f32;
        for x in x0..x1 {
            let u = (x - x0) as f32 / width;
            let red = r < nose * 2 / 3 || (r > nose && r <= nose + band);
            let colour = if red {
                if u < 0.45 { c.trim.1 } else { c.trim.0 }
            } else if y == stage {
                c.hull[0]
            } else if u < 0.2 {
                c.hull[2]
            } else if u < 0.45 {
                c.hull[3]
            } else if u < 0.78 {
                c.hull[1]
            } else {
                c.hull[0]
            };
            canvas.set(x, y, colour);
        }
    }
    // swept fins on both sides of the base
    let fin_h = rw.clamp(3, 10);
    let fin_out = (rw / 3).max(1);
    let left = (centre - half_w).round() as i32;
    let right = (centre + half_w).round() as i32;
    for i in 0..fin_h {
        let out = ((i + 1) * fin_out + fin_h - 1) / fin_h;
        let y = bottom - fin_h + i;
        canvas.fill_rect(left - out, y, out, 1, c.trim.1);
        canvas.fill_rect(right, y, out, 1, c.trim.0);
    }
    // the engine bell, flaring towards its mouth
    for i in 0..pad.bell {
        let k = (i + 1) as f32 / pad.bell as f32;
        let half = (rw as f32 * (0.2 + 0.15 * k)).max(1.0);
        let (x0, x1) = (
            (centre - half).round() as i32,
            (centre + half).round() as i32,
        );
        canvas.fill_rect(x0, bottom + i, (x1 - x0).max(1), 1, c.bell);
    }
}

/// The flame under the bell, with a slow waver in its length, never a flicker.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_flame(
    canvas: &mut Canvas,
    pad: &Pad,
    c: &Colours,
    cx: i32,
    mouth: i32,
    burn: f32,
    waver: f32,
) {
    if burn <= 0.05 {
        return;
    }
    let length = pad.rw as f32 * (0.8 + 2.4 * burn) + waver * 0.8;
    let half_top = pad.rw as f32 * 0.38 * (0.6 + 0.4 * burn);
    let centre = cx as f32 + 0.5;
    for i in 0..length.round() as i32 {
        let v = i as f32 / length;
        let half = half_top * (1.0 - v).powf(0.7) * (1.0 + 0.35 * (v * 3.0).min(1.0));
        let core = half * 0.5 * (1.0 - v);
        let (x0, x1) = (
            (centre - half).round() as i32,
            (centre + half).round() as i32,
        );
        // the flame goes down into the trench under the platform, out of sight
        if mouth + i >= pad.deck_top {
            break;
        }
        for x in x0..x1.max(x0 + 1) {
            let inner = (x as f32 + 0.5 - centre).abs() < core.max(0.5) && v < 0.6;
            canvas.set(x, mouth + i, if inner { c.flame.1 } else { c.flame.0 });
        }
    }
}

/// The soft glow of the engine, swelling with the burn.
pub(crate) fn paint_glow(canvas: &mut Canvas, pad: &Pad, c: &Colours, x: f32, y: f32, burn: f32) {
    if burn <= 0.05 {
        return;
    }
    let r = pad.rw as f32 * (0.5 + 0.9 * burn);
    paint::ellipse(canvas, x, y, r * 1.6, r, c.glow_far);
    paint::ellipse(canvas, x, y, r, r * 0.6, c.glow);
}

/// Paints the board's frame, its face cleared to the background and its two legs.
pub(crate) fn paint_board(canvas: &mut Canvas, pad: &Pad, c: &Colours) {
    let Some((col, row)) = pad.board else {
        return;
    };
    let (x0, y0) = (col - 2, row * 2 - 2);
    let (bw, bh) = (BOARD_CELLS + 4, 6);
    let stand = pad.ground + ((pad.h - pad.ground) as f32 * 0.82).round() as i32;
    for x in [x0 + 2, x0 + bw - 3] {
        canvas.fill_rect(x, y0 + bh, 1, stand - y0 - bh, c.steel[0]);
    }
    canvas.fill_rect(x0, y0, bw, bh, c.steel[1]);
    canvas.fill_rect(x0, y0, bw, 1, c.steel[2]);
    canvas.fill_rect(x0 + 1, y0 + 1, bw - 2, bh - 2, c.face);
}

/// Writes the board's text centred on its face.
pub(crate) fn write_board(frame: &mut Frame, pad: &Pad, text: &str, colour: Rgb) {
    let Some((col, row)) = pad.board else {
        return;
    };
    let len = text.chars().count() as i32;
    paint::label(frame, col + (BOARD_CELLS - len) / 2, row, text, colour);
}

/// Clouds for the sky above the pad.
pub(crate) fn sky_clouds(pad: &Pad, motion: Motion, rng: &mut Rng) -> Vec<Cloud> {
    let count = ((pad.w / 45) as usize).clamp(1, 6);
    let low = (pad.horizon as f32 * 0.5).max(3.0);
    (0..count)
        .map(|_| {
            Cloud::random(
                rng,
                pad.w as f32,
                (pad.horizon as f32 * 0.18, low),
                (pad.w as f32 * 0.08, pad.w as f32 * 0.18),
                1.2 * motion.speed(),
            )
        })
        .collect()
}

/// Paints one moment on the pad, everything but the board's text.
pub(crate) fn paint_moment(
    canvas: &mut Canvas,
    pad: &Pad,
    c: &Colours,
    theme: &ThemeData,
    clouds: &[Cloud],
    m: &Moment,
) {
    paint_setting(canvas, pad, c, theme, clouds, m.drift);
    if pad.h < 6 {
        return;
    }
    // the glow lights the apron under the pad while the rocket is still on it
    // the engine's glow swells behind the foot of the rocket while it is still low
    if let Some(alt) = m.rocket.filter(|&alt| alt < pad.rw as f32 * 4.0) {
        let fade = 1.0 - alt / (pad.rw as f32 * 4.0);
        paint_glow(
            canvas,
            pad,
            c,
            pad.px as f32 + 0.5,
            pad.deck_top as f32,
            m.burn * fade,
        );
    }
    paint_gantry(canvas, pad, c);
    if let Some(alt) = m.rocket {
        let bottom = pad.body_bottom() - alt.round() as i32;
        let cx = if alt > 0.0 {
            pad.px
        } else {
            m.platform.floor() as i32
        };
        paint_flame(canvas, pad, c, cx, bottom + pad.bell, m.burn, m.waver);
        paint_rocket(canvas, pad, c, cx, bottom);
    }
    paint_platform(canvas, pad, c, m.platform);
    // the smoke billows in front of the platform and the foot of the rocket; the trail starts
    // below the flame, so it never hides it
    paint_puffs(canvas, &m.puffs, &c.smoke);
    // the arm stands clear in front of the steam that vents past it
    paint_arm(canvas, pad, c, m.arm);
    paint_board(canvas, pad, c);
}

/// How far `clock` is between `from` and `to`, eased.
pub(crate) fn between(clock: f32, from: f32, to: f32) -> f32 {
    ease((clock - from) / (to - from).max(0.01))
}

pub struct RocketLaunch {
    width: usize,
    height: usize,
    motion: Motion,
    /// The launch's own clock, on the motion pace, wrapped to one launch.
    clock: f64,
    /// Time on the motion speed, for the sea and the surf.
    drift: f64,
    /// Time on the motion pace, for the flame's waver.
    t: f64,
    clouds: Vec<Cloud>,
}

impl RocketLaunch {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            clock: f64::from(START),
            drift: 0.0,
            t: 0.0,
            clouds: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    fn pad(&self) -> Pad {
        Pad::new(self.width, self.height)
    }

    fn now(&self) -> f32 {
        self.clock as f32
    }

    /// How far the rocket has climbed off the pad, in pixels. `None` once it is gone.
    fn altitude(&self, pad: &Pad, s: f32) -> Option<f32> {
        if s < LIFT {
            return Some(0.0);
        }
        let k = (s - LIFT) / CLIMB;
        // a slow start that gathers pace, as a heavy rocket does
        (k <= 1.0).then(|| pad.climb() * (0.25 * k + 0.75 * k * k))
    }

    /// The text on the board.
    fn board_text(&self) -> String {
        let s = self.now();
        if s < COUNT {
            "T-10".to_string()
        } else if s < IGNITE {
            format!("T-{:02}", 10 - (s - COUNT).floor() as i32)
        } else if s < LIFT {
            "T-00".to_string()
        } else {
            "LIFTOFF".to_string()
        }
    }

    fn platform_x(&self, pad: &Pad) -> f32 {
        let s = self.now();
        let off = pad.deck_w as f32;
        let home = pad.px as f32 + 0.5;
        if s < ROLL_IN {
            -off + (home + off) * between(s, 0.0, ROLL_IN)
        } else if s >= ROLL_OUT.0 {
            home + (pad.w as f32 + off - home) * between(s, ROLL_OUT.0, ROLL_OUT.1)
        } else {
            home
        }
    }

    fn arm(&self) -> f32 {
        let s = self.now();
        let swung = 0.25;
        if s < ARM_IN.0 || s >= ARM_OUT.1 {
            swung
        } else if s < ARM_IN.1 {
            swung + (1.0 - swung) * between(s, ARM_IN.0, ARM_IN.1)
        } else if s < ARM_OUT.0 {
            1.0
        } else {
            1.0 - (1.0 - swung) * between(s, ARM_OUT.0, ARM_OUT.1)
        }
    }

    /// How far the engine has lit: it swells over the four seconds before lift-off.
    fn burn(&self) -> f32 {
        let s = self.now();
        if s < IGNITE {
            0.0
        } else {
            between(s, IGNITE, LIFT)
        }
    }

    /// How far the smoke has thinned away, from 0 to 1, once the rocket has gone.
    fn thinned(&self) -> f32 {
        ((self.now() - LIFT - CLIMB) / FADE).clamp(0.0, 1.0)
    }

    /// The puffs of smoke: a cloud billowing out along the ground from ignition and a trail
    /// left behind the climbing rocket, each puff swelling and drifting with the wind as it
    /// ages and shrinking away once the rocket has gone.
    fn puffs(&self, pad: &Pad) -> Vec<Puff> {
        let s = self.now();
        let mut out = Vec::new();
        if s < IGNITE {
            return out;
        }
        let thin = self.thinned();
        // each puff thins at its own moment, so the cloud breaks up rather than shrinking whole
        let shrink = |k: u32| -> f32 {
            let start = unit(k as i32, 3, 41) * 0.6;
            1.0 - ((thin - start) / 0.4).clamp(0.0, 1.0)
        };
        let rw = pad.rw as f32;
        let since = s - IGNITE;
        let amount = between(s, IGNITE, LIFT + 5.0);
        let wind = 0.09 * rw;
        let blown = wind * since.min(36.0);
        // the ground cloud: puffs pushed out either side of the trench, the outer ones
        // smaller and further along the more time has passed
        let side = 6_i32;
        for i in -side..=side {
            let k = (i + side) as u32;
            let n = i.unsigned_abs() as f32 / side as f32;
            let spread = (1.0 + since * 0.08).min(1.7);
            let x = pad.px as f32 + 0.5 + i as f32 * rw * 0.55 * spread + blown;
            let r = rw * (1.5 - n * 0.75) * amount * shrink(k) * (0.85 + 0.3 * unit(i, 7, 2));
            out.push(Puff {
                x,
                y: pad.foot as f32 - r * (0.35 + 0.3 * unit(i, 8, 2)),
                r,
                seed: 100 + k,
            });
        }
        // a few crowning puffs above the middle of the cloud
        for i in -2..=2_i32 {
            let k = (i + 2) as u32 + 20;
            let r = rw * (1.0 - i.unsigned_abs() as f32 * 0.15) * amount * shrink(k);
            out.push(Puff {
                x: pad.px as f32 + 0.5 + i as f32 * rw * 0.9 + blown * 1.2,
                y: pad.foot as f32 - rw * 1.4 * amount,
                r,
                seed: 200 + k,
            });
        }
        // the trail, one puff each few pixels of climb, each aged from when the rocket passed
        if s > LIFT {
            let total = pad.climb();
            let climbed = self.altitude(pad, s).unwrap_or(total * 2.0);
            let step = (rw * 0.7).max(2.0);
            for k in 0..400 {
                let a = k as f32 * step;
                if a > climbed - rw * 3.6 {
                    break;
                }
                // the time the rocket passed this height, from solving the climb curve for it
                let q = a / total;
                let along = (-0.25 + (0.0625 + 3.0 * q).sqrt()) / 1.5;
                let passed = LIFT + CLIMB * along;
                let age = (s - passed).max(0.0);
                let r = (rw * 0.4 + age * rw * 0.12).min(rw * 2.4) * shrink(300 + k as u32);
                let jitter = (unit(k, 9, 5) - 0.5) * rw * 0.4;
                out.push(Puff {
                    x: pad.px as f32 + 0.5 + jitter + wind * age * 1.6,
                    y: pad.body_bottom() as f32 - a + rw * 0.4,
                    r,
                    seed: 300 + k as u32,
                });
            }
        }
        out
    }

    fn moment(&self, pad: &Pad) -> Moment {
        let s = self.now();
        let rocket = if s >= ROLL_OUT.0 {
            None
        } else {
            self.altitude(pad, s)
        };
        Moment {
            platform: self.platform_x(pad),
            rocket,
            arm: self.arm(),
            burn: self.burn(),
            waver: cycle(self.t, std::f32::consts::TAU / FLAME_PERIOD).sin(),
            puffs: self.puffs(pad),
            drift: self.drift as f32,
        }
    }
}

impl Scene for RocketLaunch {
    fn name(&self) -> &'static str {
        "rocket-launch"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.clouds = sky_clouds(&self.pad(), self.motion, rng);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        let pace = self.motion.pace();
        self.clock = (self.clock + f64::from(dt * pace)).rem_euclid(f64::from(CYCLE));
        self.t += f64::from(dt * pace);
        self.drift = (self.drift + f64::from(dt * self.motion.speed())).rem_euclid(100_000.0);
        let w = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, w);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let pad = self.pad();
        let mut canvas = Canvas::for_frame(frame, theme.background);
        paint_moment(
            &mut canvas,
            &pad,
            &colours,
            theme,
            &self.clouds,
            &self.moment(&pad),
        );
        canvas.blit(frame);
        let text = self.board_text();
        let colour = if text == "LIFTOFF" {
            theme.green()
        } else {
            theme.foreground
        };
        write_board(frame, &pad, &text, colour);
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
        check_scene("rocket-launch");
        check_determinism("rocket-launch");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("rocket-launch", 6.0);
    }

    #[test]
    fn the_board_counts_down_then_the_rocket_lifts_climbs_and_the_next_rolls_in() {
        let mut rng = Rng::new(3);
        let mut scene = RocketLaunch::new(100, 30, Motion::FULL, &mut rng);
        let pad = scene.pad();
        scene.clock = 0.0;
        let mut texts: Vec<String> = Vec::new();
        let mut last_alt = 0.0;
        let mut flew = false;
        for _ in 0..(CYCLE * 20.0) as usize - 2 {
            scene.step(0.05, &mut rng);
            let text = scene.board_text();
            if texts.last() != Some(&text) {
                texts.push(text);
            }
            let s = scene.now();
            if let Some(alt) = scene.moment(&pad).rocket {
                if s > LIFT && s < LIFT + CLIMB {
                    assert!(alt >= last_alt, "the rocket sank at {s}");
                    last_alt = alt;
                    flew = true;
                }
            }
        }
        assert!(flew);
        assert_eq!(texts.first().map(String::as_str), Some("T-10"));
        assert!(texts.iter().any(|t| t == "T-05"));
        assert!(texts.iter().any(|t| t == "LIFTOFF"));
        // the clock wraps round to a rocket rolling in from off the left of the screen
        scene.clock = 0.1;
        assert!(scene.platform_x(&pad) < 0.0);
        scene.clock = f64::from(ROLL_IN + 1.0);
        assert_eq!(scene.platform_x(&pad), pad.px as f32 + 0.5);
    }

    #[test]
    fn the_countdown_and_the_glow_change_slowly_on_every_level() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let mut scene = RocketLaunch::new(80, 24, motion, &mut rng);
            scene.clock = f64::from(COUNT - 1.0);
            let mut last = scene.board_text();
            let mut changed_at = -10.0_f32;
            let mut t = 0.0_f32;
            let mut burn = scene.burn();
            for _ in 0..(30 * 20) {
                scene.step(0.05, &mut rng);
                t += 0.05;
                let text = scene.board_text();
                if text != last {
                    assert!(t - changed_at >= 0.95, "{motion:?} changed after {t}");
                    changed_at = t;
                    last = text;
                }
                // the glow swells over seconds, never in a jump
                let now = scene.burn();
                assert!((now - burn).abs() < 0.04, "{motion:?}");
                burn = now;
            }
        }
    }

    #[test]
    fn the_rocket_and_its_tower_are_drawn_and_survive_a_resize() {
        let c = Colours::new(theme::default_theme());
        let mut rng = Rng::new(4);
        let mut scene = RocketLaunch::new(100, 30, Motion::FULL, &mut rng);
        scene.clock = f64::from(COUNT);
        let frame = render(&mut scene, 100, 30, 1);
        let all = pixels(&frame);
        for (what, colour) in [
            ("hull", c.hull[1]),
            ("nose", c.trim.0),
            ("steel", c.steel[1]),
            ("deck", c.deck[1]),
            ("beacon", c.beacon),
        ] {
            assert!(all.contains(&colour), "no {what}");
        }
        assert!(frame.to_text().contains("T-10"));
        let clock = scene.clock;
        scene.resize(20, 8, &mut rng);
        assert_eq!(scene.clock, clock);
        let small = render(&mut scene, 20, 8, 1);
        assert!(pixels(&small).contains(&c.hull[1]));
    }

    #[test]
    fn imagery_stays_below_the_ceiling_and_shapes_stand_out_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (what, colour) in c.imagery() {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {}",
                    theme.name
                );
            }
            for (what, colour) in c.shapes() {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "the {what} in {}",
                    theme.name
                );
            }
            apart(
                theme,
                c.sky.at(1.0),
                c.sea.at(0.0),
                0.03,
                "the sky and the sea",
            );
            apart(theme, c.sea.at(1.0), c.sand, 0.03, "the sea and the beach");
            apart(
                theme,
                c.smoke[0],
                c.smoke[2],
                0.05,
                "the smoke's light and shade",
            );
            apart(
                theme,
                c.hull[2],
                c.steel[1],
                0.06,
                "the rocket and the gantry",
            );
            apart(theme, c.trim.1, c.hull[2], 0.08, "the band and the body");
            apart(theme, c.flame.0, c.flame.1, 0.06, "the flame and its core");
        }
    }
}
