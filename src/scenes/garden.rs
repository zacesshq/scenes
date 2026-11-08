//! Garden, painted in solid pixels: a flower bed in front of a picket fence and a hedge under a
//! summer sky. Seeds sprout two seed leaves, stems climb and put out leaves, buds swell and open
//! into tulips, daisies, sunflowers, roses, bluebells and lavender until the bed is full of
//! flowers, with butterflies drifting over it, a still sun and slow clouds. When the season ends
//! the petals fall, the stems settle back into the soil and the bed starts again.
//!
//! Every plant's state is worked out from the garden's clock rather than stored, so a resize keeps
//! the season where it was.
//!
//! Every colour comes from the active theme. The sky, the clouds, the hedge, the fence, the lawn
//! and the soil are tints of the background held below the non-text floor, so they read as
//! background; the plants, the flowers, the butterflies and the sun keep the theme's own drawing
//! colours, so they stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::cycle;
use super::paint::{Cloud, Wave, cloud_tones, noise, solid, stronger};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A flower bed by a picket fence that sprouts, grows and blooms over a couple of minutes, with butterflies, before it starts again";

pub const CAPTIONS: &[&str] = &[
    "Everything is growing nicely.",
    "A clean build. Watch it bloom.",
    "No weeds in this bed.",
    "Green shoots all round.",
    "Tended and thriving.",
];

/// Seconds one season takes, from seed to bloom and back, at the standard pace.
pub const SEASON: f32 = 110.0;
/// Seconds a butterfly's wings take to open and close.
pub const FLUTTER_PERIOD: f32 = 1.2;
/// The share of the season by which every flower has bloomed and when petals start to fall.
const FALL_START: f32 = 0.84;
const SETTLE_START: f32 = 0.92;
/// The tallest a plant grows, in pixels. With the slowest growth share this keeps every stem
/// to at most a pixel a second.
const TALLEST: i32 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Tulip,
    Daisy,
    Sunflower,
    Bell,
    Lavender,
    Rose,
}

const KINDS: [Kind; 6] = [
    Kind::Tulip,
    Kind::Daisy,
    Kind::Sunflower,
    Kind::Bell,
    Kind::Lavender,
    Kind::Rose,
];

/// Flower heads in pixels, centred on the top of the stem: `p` a petal, `P` a deeper petal, `c`
/// the centre and `g` green.
const TULIP: [&str; 6] = [
    "p..p..p", "pp.p.pp", "ppPpPpp", "pPPPPPp", ".PPPPP.", "..PPP..",
];
const DAISY: [&str; 7] = [
    "..p.p..", "p.ppp.p", ".ppcpp.", "ppcccpp", ".ppcpp.", "p.ppp.p", "..p.p..",
];
const SUNFLOWER: [&str; 9] = [
    "...p.p...",
    ".p.ppp.p.",
    "..pPPPp..",
    "ppPcccPpp",
    ".pPcccPp.",
    "ppPcccPpp",
    "..pPPPp..",
    ".p.ppp.p.",
    "...p.p...",
];
const ROSE: [&str; 5] = [".pPPp.", "pPppPp", "PpPPpP", "pPppPP", ".gPPg."];
/// A bluebell's arching tip with bells hanging along it.
const BELL: [&str; 6] = [
    "..ggg...", ".g...g..", "pP....g.", "PP...pP.", "p.p..PP.", ".....p.p",
];
/// A butterfly with its wings open, half open and folded: `w` wing, `W` wing edge, `b` body.
const WINGS_OPEN: [&str; 4] = ["WWW.WWW", "WwwbwwW", ".WwbwW.", ".ww.ww."];
const WINGS_HALF: [&str; 4] = ["..W.W..", ".WwbwW.", "..wbw..", "..w.w.."];
const WINGS_SHUT: [&str; 4] = ["...W...", "..Wb...", "..wb...", "...w..."];

#[derive(Clone, Debug)]
struct Plant {
    x: i32,
    /// The row its stem rises from: the back row a little higher, so it sits further away.
    base: i32,
    kind: Kind,
    /// Full height in pixels.
    full: i32,
    /// When it sprouts and how long it grows for, as shares of the season.
    start: f32,
    grow: f32,
    /// Which side its first leaf grows on.
    leaf: bool,
}

#[derive(Clone, Debug)]
struct Butterfly {
    centre: f32,
    reach: f32,
    rate: f32,
    phase: f32,
    row: f32,
    colour: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Seed,
    Growing(i32),
    Bud,
    Bloom,
    /// Petals falling, for seconds so far.
    Falling(f32),
    Settling(i32),
}

/// The garden's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct GardenColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub halo: Rgb,
    pub hedge: Rgb,
    pub hedge_lit: Rgb,
    pub fence: Rgb,
    pub fence_lit: Rgb,
    pub fence_shade: Rgb,
    pub lawn: Rgb,
    pub soil: Rgb,
    pub soil_dark: Rgb,
    pub soil_lit: Rgb,
    pub seed: Rgb,
    pub sun: Rgb,
    pub stem: Rgb,
    pub leaf: Rgb,
    pub leaf_dark: Rgb,
    pub petals: [(Rgb, Rgb, Rgb); 6],
    pub wings: [Rgb; 3],
    pub body: Rgb,
    sky: Ramp,
}

impl GardenColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let (sky_top, sky_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.32, 1.5),
                shades.tint(theme.info, 0.12, 1.15),
            )
        } else {
            (
                shades.tint(theme.info, 0.5, 1.75),
                shades.tint(theme.info, 0.12, 1.15),
            )
        };
        let (cloud, cloud_shade) = cloud_tones(theme, IMAGERY_CEILING - 0.3);
        let green = theme.green();
        // the hedge and the lawn sit further back than the bed, so they are softer tints
        let (hedge, hedge_lit, lawn) = if theme.dark {
            (
                shades.tint(green, 0.2, 1.35),
                shades.tint(green, 0.3, 1.6),
                shades.tint(green, 0.38, 1.85),
            )
        } else {
            (
                shades.tint(green, 0.5, 2.0),
                shades.tint(green, 0.38, 1.7),
                shades.tint(green, 0.3, 1.5),
            )
        };
        let wood = mix(theme.muted, theme.warning, 0.25);
        let (fence, fence_lit, fence_shade) = if theme.dark {
            (
                shades.tint(wood, 0.45, 2.1),
                shades.tint(shades.lighter(wood, 0.4), 0.6, 2.6),
                shades.tint(wood, 0.3, 1.6),
            )
        } else {
            (
                shades.tint(wood, 0.35, 1.5),
                shades.tint(wood, 0.12, 1.12),
                shades.tint(wood, 0.6, 2.2),
            )
        };
        let earth = mix(theme.yellow(), theme.red(), 0.45);
        let soil = shades.tint(earth, 0.5, 2.1);
        let petal = |c: Rgb, centre: Rgb| (stronger(theme, c, 0.3), c, centre);
        let dark_centre = solid(
            theme,
            mix(theme.yellow(), theme.red(), 0.6),
            shades.darkest(),
            0.5,
        );
        Self {
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            halo: if theme.dark {
                shades.tint(theme.yellow(), 0.3, 1.6)
            } else {
                mix(theme.background, sunlit(theme.yellow()), 0.3)
            },
            hedge,
            hedge_lit,
            fence,
            fence_lit,
            fence_shade,
            lawn,
            soil,
            soil_dark: shades.tint(earth, 0.7, 2.6),
            soil_lit: shades.tint(earth, 0.32, 1.6),
            seed: shades.held(earth, theme.foreground, 0.2, IMAGERY_CEILING + 0.2),
            // a light theme's yellow is dark enough for text, which would paint a brown sun, so
            // its sun is the warmest gold the sky still holds as imagery
            sun: if theme.dark {
                theme.bright_yellow()
            } else {
                sunlit(theme.yellow())
            },
            stem: green,
            leaf: stronger(theme, theme.bright_green(), 0.15),
            leaf_dark: solid(theme, green, shades.darkest(), 0.3),
            petals: [
                petal(theme.red(), theme.yellow()),
                petal(theme.bright_magenta(), theme.yellow()),
                // a light theme's own yellow is close to brown, so a sunflower there is the
                // sun's gold darkened only as far as a solid shape needs
                if theme.dark {
                    petal(theme.yellow(), dark_centre)
                } else {
                    let gold = super::paint::strong(theme, sunlit(theme.yellow()));
                    (gold, stronger(theme, gold, 0.25), dark_centre)
                },
                petal(theme.blue(), theme.blue()),
                petal(theme.magenta(), theme.magenta()),
                petal(theme.bright_red(), theme.red()),
            ],
            wings: [theme.magenta(), theme.bright_cyan(), theme.yellow()],
            body: strong_body(theme),
            sky: Ramp::new(sky_top, sky_low, 8),
        }
    }
}

/// A colour's own hue lifted to a bright, clear gold: the light of a sun in a pale sky.
fn sunlit(colour: Rgb) -> Rgb {
    let lab = crate::pixels::Lab::from_rgb(colour);
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt().max(0.001);
    let lift = 0.13_f32.max(chroma) / chroma;
    crate::pixels::Lab {
        l: 0.88,
        a: lab.a * lift,
        b: lab.b * lift,
    }
    .to_rgb()
}

/// A butterfly's body: the theme's strongest text colour, which every wing colour stands apart
/// from.
fn strong_body(theme: &ThemeData) -> Rgb {
    theme.bright()
}

fn kind_index(kind: Kind) -> usize {
    KINDS.iter().position(|&k| k == kind).unwrap_or(0)
}

pub struct Garden {
    width: usize,
    height: usize,
    motion: Motion,
    /// The season clock, in seconds of the standard pace.
    season: f64,
    t: f64,
    plants: Vec<Plant>,
    butterflies: Vec<Butterfly>,
    clouds: Vec<Cloud>,
    hedge: Option<Wave>,
}

impl Garden {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut garden = Self {
            width: 1,
            height: 1,
            motion,
            // the first season opens with the earliest flowers already out, so a passing build
            // is greeted by colour while the rest are still growing
            season: f64::from(SEASON) * 0.5,
            t: 0.0,
            plants: Vec::new(),
            butterflies: Vec::new(),
            clouds: Vec::new(),
            hedge: None,
        };
        garden.resize(width, height, rng);
        garden
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The first pixel row of the soil.
    fn ground(&self) -> i32 {
        let tall = self.tall();
        let depth = ((tall as f32 * 0.14).round() as i32).clamp(3, 22);
        (tall - depth).max(1)
    }

    fn has_fence(&self) -> bool {
        self.height >= 12
    }

    /// The fence's top and its foot, in pixel rows.
    fn fence_rows(&self) -> (i32, i32) {
        let tall = self.tall();
        let foot = self.ground() - 2;
        let high = ((tall as f32 * 0.2).round() as i32).clamp(8, 30);
        (foot - high, foot)
    }

    /// How far through the season the garden is, from 0 to 1.
    fn progress(&self) -> f32 {
        (self.season / f64::from(SEASON)).rem_euclid(1.0) as f32
    }

    fn stage(&self, plant: &Plant) -> Stage {
        let p = self.progress();
        if p < plant.start {
            return Stage::Seed;
        }
        if p >= SETTLE_START {
            let left = 1.0 - (p - SETTLE_START) / (1.0 - SETTLE_START);
            return Stage::Settling((plant.full as f32 * left).floor() as i32);
        }
        if p >= FALL_START {
            return Stage::Falling((p - FALL_START) * SEASON);
        }
        let grown = (p - plant.start) / plant.grow;
        if grown < 1.0 {
            Stage::Growing((plant.full as f32 * grown).floor() as i32)
        } else if grown < 1.2 {
            Stage::Bud
        } else {
            Stage::Bloom
        }
    }

    fn flutter_period(&self) -> f32 {
        FLUTTER_PERIOD / self.motion.pace()
    }

    fn paint_backdrop(&self, canvas: &mut Canvas, c: &GardenColours) {
        let ground = self.ground();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        for y in 0..ground {
            canvas.fill_row(y, c.sky.at(y as f32 / ground.max(1) as f32));
        }
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }
        if self.height >= 10 && wide >= 30 {
            // a still sun in a soft halo, drawn over the clouds so it always reads as the sun;
            // it never changes, so it can never flash
            let r = (tall as f32 * 0.06).clamp(2.0, 8.0);
            let (sx, sy) = (wide as f32 - (wide as f32 * 0.1).max(r + 4.0), r + 3.0);
            super::paint::disc(canvas, sx, sy, r * 1.9, c.halo);
            super::paint::disc(canvas, sx, sy, r, c.sun);
        }
        if self.has_fence() {
            let (top, foot) = self.fence_rows();
            // a rounded hedge behind the fence gives the garden a back wall
            if let Some(wave) = &self.hedge {
                let hedge_top = top - (tall as f32 * 0.07).round() as i32;
                for x in 0..wide {
                    let h = hedge_top + (wave.at(x as f32) * 2.5).round() as i32;
                    let bump = ((x as f32 / 5.0).sin().abs() * 2.0).round() as i32;
                    let crown = h - bump;
                    canvas.fill_rect(x, crown, 1, foot - crown, c.hedge);
                    canvas.fill_rect(x, crown, 1, 2, c.hedge_lit);
                }
            }
            // the lawn between the fence and the bed
            canvas.fill_rect(0, foot, wide, ground - foot, c.lawn);
            // a picket fence: pointed pickets on two rails, lit from the left
            let high = foot - top;
            let rails = [top + high / 3, top + high * 3 / 4];
            for &rail in &rails {
                canvas.fill_rect(0, rail, wide, 2, c.fence_shade);
            }
            let mut x = 1;
            while x < wide {
                canvas.set(x + 1, top, c.fence);
                canvas.fill_rect(x, top + 1, 3, high - 1, c.fence);
                canvas.fill_rect(x, top + 1, 1, high - 1, c.fence_lit);
                canvas.set(x + 1, top, c.fence_lit);
                x += 5;
            }
        } else {
            canvas.fill_rect(0, ground - 1, wide, 1, c.lawn);
        }
        // the soil: a lit crumbly top edge, then soft darker patches of turned earth
        for y in ground..tall {
            for x in 0..wide {
                let n = noise(x as f32 / 5.0, y as f32 / 2.5, 41);
                let colour = if y == ground {
                    c.soil_lit
                } else if n > 0.6 {
                    c.soil_dark
                } else {
                    c.soil
                };
                canvas.set(x, y, colour);
            }
        }
    }

    /// A leaf off the stem at `(x, y)`, pointing to one side: a short blade that widens then
    /// tapers, with its underside in shadow.
    fn leaf(canvas: &mut Canvas, c: &GardenColours, x: i32, y: i32, side: i32, size: i32) {
        for i in 1..=size {
            let rise = (i - 1) / 2;
            let px = x + side * i;
            canvas.set(px, y - rise, c.leaf);
            if i > 1 && i < size {
                canvas.set(px, y - rise + 1, c.leaf_dark);
            }
        }
    }

    fn paint_stem(&self, canvas: &mut Canvas, c: &GardenColours, plant: &Plant, height: i32) {
        let base = plant.base;
        let size = (plant.full / 6).clamp(2, 5);
        for k in 0..height {
            canvas.set(plant.x, base - k, c.stem);
        }
        // leaves come out along the lower part of the stem, as on a real flower, so the heads
        // stand clear above them
        let gap = (plant.full / 4).clamp(4, 9);
        let mut k = gap / 2 + 1;
        let mut left = plant.leaf;
        while k + 2 < height && k * 10 < plant.full * 6 {
            Self::leaf(
                canvas,
                c,
                plant.x,
                base - k,
                if left { -1 } else { 1 },
                size,
            );
            left = !left;
            k += gap;
        }
        if height > 0 && height < 4 {
            // a seedling's two round seed leaves
            let top = base - height + 1;
            canvas.set(plant.x - 1, top, c.leaf);
            canvas.set(plant.x + 1, top, c.leaf);
            canvas.set(plant.x - 2, top - 1, c.leaf);
            canvas.set(plant.x + 2, top - 1, c.leaf);
        }
    }

    fn head_rows(kind: Kind) -> &'static [&'static str] {
        match kind {
            Kind::Tulip => &TULIP,
            Kind::Daisy => &DAISY,
            Kind::Sunflower => &SUNFLOWER,
            Kind::Bell => &BELL,
            Kind::Rose => &ROSE,
            Kind::Lavender => &[],
        }
    }

    fn paint_head(&self, canvas: &mut Canvas, c: &GardenColours, plant: &Plant, open: bool) {
        let top = plant.base - plant.full + 1;
        let (petal, deep, centre) = c.petals[kind_index(plant.kind)];
        if !open {
            // a swelling bud: green with the petal colour just showing at its tip
            canvas.sprite(
                plant.x - 1,
                top - 3,
                &[".p.", "pPp", "ggg", ".g."],
                &[('p', petal), ('P', deep), ('g', c.leaf_dark)],
            );
            return;
        }
        if plant.kind == Kind::Lavender {
            // a spike of tiny florets up the top of the stem
            let spike = (plant.full / 3).clamp(3, 10);
            for k in 0..spike {
                let y = top + k;
                let colour = if k % 2 == 0 { petal } else { deep };
                canvas.set(plant.x, y - 1, colour);
                if k % 2 == 1 && k + 1 < spike {
                    canvas.set(plant.x - 1, y, petal);
                    canvas.set(plant.x + 1, y, deep);
                }
            }
            return;
        }
        let rows = Self::head_rows(plant.kind);
        let w = rows[0].len() as i32;
        let h = rows.len() as i32;
        let y = if plant.kind == Kind::Bell {
            top - 1
        } else {
            top - h / 2 - 1
        };
        let x = if plant.kind == Kind::Bell {
            plant.x - 2
        } else {
            plant.x - w / 2
        };
        canvas.sprite(
            x,
            y,
            rows,
            &[('p', petal), ('P', deep), ('c', centre), ('g', c.stem)],
        );
    }

    fn paint_plants(&self, canvas: &mut Canvas, c: &GardenColours) {
        for plant in &self.plants {
            match self.stage(plant) {
                Stage::Seed => {
                    // a seed just under the surface, waiting
                    canvas.set(plant.x, plant.base + 1, c.seed);
                }
                Stage::Growing(0) | Stage::Settling(0) => {
                    canvas.set(plant.x, plant.base, c.leaf);
                    canvas.set(plant.x, plant.base + 1, c.seed);
                }
                Stage::Growing(height) | Stage::Settling(height) => {
                    self.paint_stem(canvas, c, plant, height);
                }
                Stage::Bud => {
                    self.paint_stem(canvas, c, plant, plant.full);
                    self.paint_head(canvas, c, plant, false);
                }
                Stage::Bloom => {
                    self.paint_stem(canvas, c, plant, plant.full);
                    self.paint_head(canvas, c, plant, true);
                }
                Stage::Falling(since) => {
                    self.paint_stem(canvas, c, plant, plant.full);
                    // a few petals drift down from where the flower was, swaying as they go
                    let (petal, deep, _) = c.petals[kind_index(plant.kind)];
                    let top = (plant.base - plant.full) as f32;
                    for j in 0..4 {
                        let fall = since * self.motion.speed() * 2.6 - j as f32 * 1.6;
                        if fall < 0.0 {
                            continue;
                        }
                        let y = (top + fall).min(plant.base as f32 + 1.0);
                        let x = plant.x as f32 + (fall * 0.45 + j as f32 * 2.1).sin() * 2.5;
                        let colour = if j % 2 == 0 { petal } else { deep };
                        canvas.set(x.round() as i32, y.round() as i32, colour);
                    }
                }
            }
        }
    }

    fn paint_butterflies(&self, canvas: &mut Canvas, c: &GardenColours) {
        let period = f64::from(self.flutter_period());
        for (i, fly) in self.butterflies.iter().enumerate() {
            let a = cycle(self.t, fly.rate * TAU * self.motion.speed()) + fly.phase;
            let x = fly.centre + fly.reach * a.sin();
            let y = fly.row + 3.0 * (a * 2.3).sin();
            let beat = ((self.t / period) + i as f64 * 0.37).rem_euclid(1.0);
            let rows: &[&str] = match (beat * 4.0) as u32 {
                0 => &WINGS_OPEN,
                2 => &WINGS_SHUT,
                _ => &WINGS_HALF,
            };
            let wing = c.wings[fly.colour % c.wings.len()];
            let edge = solid_edge(wing, c.body);
            canvas.sprite(
                x.round() as i32 - 3,
                y.round() as i32 - 2,
                rows,
                &[('w', wing), ('W', edge), ('b', c.body)],
            );
        }
    }
}

/// A wing's edge, a step towards the body colour so the wing has a rim.
fn solid_edge(wing: Rgb, body: Rgb) -> Rgb {
    mix(wing, body, 0.3)
}

impl Scene for Garden {
    fn name(&self) -> &'static str {
        "garden"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ground = self.ground();
        let room = ((ground as f32 * 0.62) as i32).max(2);
        // the bed is planted in two staggered rows, the back row a little higher and taller,
        // so the flowers overlap and fill it
        let gap = ((7.0 / self.motion.busy()).round() as i32).max(5);
        self.plants.clear();
        for (row, base) in [(0, ground - 1), (1, ground + 1)] {
            let mut x = 2 + rng.below(3) as i32 + row * gap / 2;
            while x < self.width as i32 - 2 {
                let kind = *rng.pick(&KINDS);
                let reach = match (kind, row) {
                    (Kind::Sunflower, _) => rng.range(0.8, 1.0),
                    (_, 0) => rng.range(0.5, 0.8),
                    _ => rng.range(0.28, 0.5),
                };
                let full = ((room as f32 * reach) as i32).clamp(2, TALLEST).min(room);
                // a tall plant grows for longer, so no stem ever rises faster than a pixel a
                // second
                let grow = rng.range(0.25, 0.4).max(full as f32 * 1.05 / SEASON);
                self.plants.push(Plant {
                    x,
                    base,
                    kind,
                    full,
                    start: rng.range(0.0, 0.3).min(FALL_START - 1.25 * grow),
                    grow,
                    leaf: rng.chance(0.5),
                });
                x += gap + rng.below(3) as i32 - 1;
            }
        }
        let wanted = if self.height >= 10 {
            self.motion.scaled((self.width / 40).max(1)).min(6)
        } else {
            0
        };
        let tall = self.tall() as f32;
        let (low, high) = (tall * 0.2, (ground - room) as f32);
        self.butterflies = (0..wanted)
            .map(|i| Butterfly {
                centre: rng.range(0.15, 0.85) * self.width as f32,
                reach: rng.range(0.1, 0.25) * self.width as f32,
                rate: rng.range(0.03, 0.06),
                phase: rng.range(0.0, TAU),
                row: rng.range(low, high.max(low + 1.0)),
                colour: i,
            })
            .collect();
        let wide = self.width as f32;
        self.clouds = if self.height >= 10 {
            let count = (self.width / 45).clamp(1, 5);
            (0..count)
                .map(|_| {
                    Cloud::random(
                        rng,
                        wide,
                        (tall * 0.12, tall * 0.26),
                        (wide * 0.08, (wide * 0.16).max(8.0)),
                        self.motion.speed(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        self.hedge = Some(Wave::new(rng, 14.0));
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.season += f64::from(dt * self.motion.pace());
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, width);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = GardenColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint_backdrop(&mut canvas, &colours);
        self.paint_plants(&mut canvas, &colours);
        self.paint_butterflies(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels,
    };
    use crate::theme;

    fn drawn(garden: &Garden, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        garden.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("garden");
        check_determinism("garden");
        check_pixel_scene("garden", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in [
            &TULIP[..],
            &DAISY[..],
            &SUNFLOWER[..],
            &ROSE[..],
            &BELL[..],
            &WINGS_OPEN[..],
            &WINGS_HALF[..],
            &WINGS_SHUT[..],
        ] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
    }

    #[test]
    fn a_season_grows_blooms_and_starts_again() {
        let mut rng = Rng::new(8);
        let mut garden = Garden::new(80, 24, Motion::FULL, &mut rng);
        let mut bloomed = false;
        let mut fell = false;
        let mut restarted = false;
        let mut was_late = false;
        for _ in 0..((SEASON as usize + 10) * 20) {
            garden.step(0.05, &mut rng);
            let stages: Vec<Stage> = garden.plants.iter().map(|p| garden.stage(p)).collect();
            bloomed |= stages.contains(&Stage::Bloom);
            fell |= stages.iter().any(|s| matches!(s, Stage::Falling(_)));
            if was_late && garden.progress() < 0.1 {
                restarted = true;
            }
            was_late = garden.progress() > 0.9;
        }
        assert!(bloomed && fell && restarted);
    }

    #[test]
    fn every_plant_blooms_before_the_petals_fall() {
        for &(w, h) in &[(20, 8), (80, 24), (400, 120)] {
            let mut rng = Rng::new(8);
            let garden = Garden::new(w, h, Motion::FULL, &mut rng);
            for plant in &garden.plants {
                assert!(plant.start + plant.grow * 1.2 < FALL_START, "{plant:?}");
            }
        }
    }

    #[test]
    fn stems_grow_slowly_and_wings_beat_slowly() {
        let mut rng = Rng::new(8);
        for motion in Motion::ALL {
            for &(w, h) in &[(200, 60), (400, 120)] {
                let garden = Garden::new(w, h, motion, &mut rng);
                assert!(garden.flutter_period() >= FLUTTER_PERIOD);
                for plant in &garden.plants {
                    // seconds a pixel of stem takes to grow, at the fastest pace
                    let per_pixel = plant.grow * SEASON / plant.full as f32;
                    assert!(per_pixel >= 1.0, "{per_pixel}");
                }
            }
        }
    }

    #[test]
    fn a_bloom_shows_its_petals_in_every_theme() {
        for theme in theme::all() {
            let c = GardenColours::new(theme);
            let mut rng = Rng::new(8);
            let mut garden = Garden::new(80, 24, Motion::FULL, &mut rng);
            // most of the way through the season, every plant has bloomed
            garden.season = f64::from(SEASON) * 0.8;
            let frame = drawn(&garden, 80, 24, theme);
            let all = pixels(&frame);
            assert!(
                all.contains(&c.stem) && all.contains(&c.sun),
                "{}",
                theme.name
            );
            let blooms = garden
                .plants
                .iter()
                .filter(|p| all.contains(&c.petals[kind_index(p.kind)].0))
                .count();
            assert!(blooms >= 5, "{} shows {blooms}", theme.name);
        }
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = GardenColours::new(theme);
            let name = theme.name;
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("cloud", c.cloud),
                ("hedge", c.hedge),
                ("fence", c.fence),
                ("fence edge", c.fence_lit),
                ("lawn", c.lawn),
                ("soil", c.soil),
                ("halo", c.halo),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            apart(theme, c.sky_top, c.sky_low, 0.03, "the sky's gradient");
            apart(theme, c.fence, c.hedge, 0.03, "the fence and the hedge");
            apart(theme, c.soil, c.lawn, 0.04, "the soil and the lawn");
            apart(theme, c.stem, c.soil, 0.1, "a stem and the soil");
            apart(theme, c.stem, c.lawn, 0.08, "a stem and the lawn");
            for (petal, _, _) in &c.petals {
                apart(theme, *petal, c.sky_low, 0.1, "a petal and the sky");
            }
            for wing in c.wings {
                assert!(theme.contrast_of(wing) >= theme.floor(), "{name}");
                apart(theme, wing, c.sky_top, 0.1, "a wing and the sky");
            }
            assert!(theme.contrast_of(c.stem) >= theme.floor(), "{name}");
            apart(theme, c.sun, c.halo, 0.05, "the sun and its halo");
            apart(theme, c.halo, c.sky_top, 0.03, "the halo and the sky");
        }
    }
}
