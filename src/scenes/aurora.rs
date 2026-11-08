//! Aurora: slow curtains of colour across the top of a starry sky over a mountain ridge, painted
//! in solid pixels.
//!
//! A light theme shows the same sky at the blue hour, deep blue overhead, with the curtains as
//! pale glowing light over dark mountains capped with white snow.
//!
//! Each curtain is brightest along its lower edge and fades upwards in soft rays, the way a real
//! aurora hangs. The sky, the faint parts of the curtains and the ridge are tints of the
//! background held below the non-text floor, so they read as background. The brightest edge of
//! each curtain reaches the theme's own colour, so the curtains keep their outline when colour is
//! off.

use std::f32::consts::TAU;

use super::{count_for, cycle, paint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Radians a second of the bands' slow breathing in thickness: one breath every 21 seconds.
const BREATH_RATE: f32 = 0.3;
const SKY_STEPS: usize = 8;
const GLOW_STEPS: usize = 7;

#[derive(Clone, Debug)]
struct Band {
    /// The band's centre as a fraction of the height.
    base: f32,
    /// How far the band swings, in pixels.
    amplitude: f32,
    wavelength: f32,
    phase: f32,
    speed: f32,
    /// Half the band's bright edge, in pixels.
    thickness: f32,
    /// Where in its breathing cycle the band starts.
    breath: f32,
    colour: usize,
}

/// The colours of the aurora in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct AuroraColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub star: Rgb,
    pub ridge: Rgb,
    pub ridge_shade: Rgb,
    pub snow: Rgb,
    /// The far range's tones, then the near range's.
    ranges: [RangeTones; 2],
    pub bands: [Rgb; 4],
    /// Whether this is a night sky, where every star shows.
    pub dark: bool,
    sky: Ramp,
    /// For each band colour and each sky step, the glow from the sky to the band's colour.
    glow: Vec<Vec<Ramp>>,
}

impl AuroraColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let (sky_top, sky_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.08, 1.15),
                shades.tint(theme.info, 0.36, 1.7),
            )
        } else {
            (
                shades.tint(theme.info, 0.9, 2.6),
                shades.tint(mix(theme.info, theme.magenta(), 0.35), 0.35, 1.55),
            )
        };
        // the mountains stand dark against the sky, their snow bright on the crests
        let (ridge, snow) = if theme.dark {
            (
                mix(theme.background, theme.muted, 0.04),
                shades.tint(theme.bright(), 0.6, 2.7),
            )
        } else {
            (
                paint::strong(
                    theme,
                    shades.tint(mix(theme.info, theme.muted, 0.5), 1.0, 2.9),
                ),
                WHITE,
            )
        };
        // by day the curtains are pale light, brighter than the deep sky they hang in
        let bands = if theme.dark {
            [theme.green(), theme.cyan(), theme.magenta(), theme.blue()]
        } else {
            [theme.green(), theme.cyan(), theme.magenta(), theme.blue()]
                .map(|hue| mix(WHITE, hue, 0.34))
        };
        let sky = Ramp::new(sky_top, sky_low, SKY_STEPS);
        let glow = bands
            .iter()
            .map(|&band| {
                sky.steps()
                    .iter()
                    .map(|&step| Ramp::new(step, band, GLOW_STEPS))
                    .collect()
            })
            .collect();
        let ridge_shade = if theme.dark {
            shades.tint(theme.muted, 0.1, 1.15)
        } else {
            paint::stronger(theme, ridge, 0.12)
        };
        // at night the far range catches the light of the aurora on its lit faces while the near
        // one stands dark in front; by day the near range is the darker one and the far range
        // pales into the haze
        let ranges = if theme.dark {
            let rock = mix(theme.muted, theme.info, 0.3);
            let snow_shade = mix(snow, rock, 0.45);
            [
                RangeTones {
                    lit: shades.tint(rock, 1.0, 2.1),
                    shade: shades.tint(rock, 1.0, 1.3),
                    snow,
                    snow_shade: mix(snow_shade, sky_low, 0.2),
                },
                RangeTones {
                    lit: ridge_shade,
                    shade: ridge,
                    snow,
                    snow_shade,
                },
            ]
        } else {
            let snow_shade = mix(snow, ridge_shade, 0.3);
            [
                RangeTones {
                    lit: mix(ridge, sky_low, 0.55),
                    shade: mix(ridge, sky_low, 0.25),
                    snow: mix(snow, sky_low, 0.3),
                    snow_shade: mix(snow_shade, sky_low, 0.4),
                },
                RangeTones {
                    lit: ridge,
                    shade: ridge_shade,
                    snow,
                    snow_shade,
                },
            ]
        };
        Self {
            sky_top,
            sky_low,
            ranges,
            star: if theme.dark {
                shades.held(theme.muted, sky_top, 0.4, IMAGERY_CEILING)
            } else {
                mix(WHITE, sky_top, 0.3)
            },
            ridge,
            ridge_shade,
            snow,
            bands,
            dark: theme.dark,
            sky,
            glow,
        }
    }

    fn sky_step(&self, t: f32) -> usize {
        ((t.clamp(0.0, 1.0) * (SKY_STEPS - 1) as f32).round() as usize).min(SKY_STEPS - 1)
    }
}

/// Pure white, the snow and the brightest light on a light theme: a touch above an off-white
/// background, so it stays imagery.
const WHITE: Rgb = Rgb::new(255, 255, 255);

pub struct Aurora {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    bands: Vec<Band>,
    /// Stars as (column, pixel row).
    stars: Vec<(i32, i32)>,
    /// The far range's peaks, then the near range's.
    far: Vec<Peak>,
    near: Vec<Peak>,
}

/// One mountain: its summit's column and height above the bottom in pixels and how steeply its
/// left and right flanks fall, in pixels of height a column.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Peak {
    x: f32,
    height: f32,
    left: f32,
    right: f32,
}

impl Peak {
    /// The height of this mountain's outline at a column, roughened a little by noise so its
    /// flanks are not ruled lines.
    fn top_at(&self, x: f32, salt: u32) -> f32 {
        let slope = if x < self.x { self.left } else { self.right };
        let rough = (paint::noise(x * 0.45, self.x, salt) - 0.5) * 1.6;
        self.height - (x - self.x).abs() * slope + rough
    }
}

/// The tones of one range of mountains: lit face, shaded face, lit snow and shaded snow.
#[derive(Clone, Copy, Debug)]
struct RangeTones {
    lit: Rgb,
    shade: Rgb,
    snow: Rgb,
    snow_shade: Rgb,
}

impl Aurora {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut aurora = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            bands: Vec::new(),
            stars: Vec::new(),
            far: Vec::new(),
            near: Vec::new(),
        };
        aurora.resize(width, height, rng);
        aurora
    }

    fn tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// The pixel row of a band's centre at a column.
    fn centre(&self, band: &Band, x: f32) -> f32 {
        let main = (x / band.wavelength * TAU + band.phase).sin();
        let detail = (x / (band.wavelength * 0.37) + band.phase * 1.7).sin();
        band.base * self.tall() + band.amplitude * (main + 0.5 * detail)
    }

    /// How brightly a band glows at a pixel, from 0 to 1: brightest along its lower edge,
    /// fading in rays above it and quickly below it.
    fn glow(&self, band: &Band, thickness: f32, x: i32, y: i32) -> f32 {
        let edge = self.centre(band, x as f32) + thickness;
        let y = y as f32 + 0.5;
        if y > edge {
            return (1.0 - (y - edge) / 1.5).max(0.0);
        }
        // rays: streaks a few columns wide that drift slowly with the band, some reaching much
        // higher than others, as the folds of a real curtain do
        let ray = paint::noise(
            x as f32 * 0.32 + band.phase * 1.3,
            band.colour as f32 * 7.0,
            19,
        );
        let curtain = (thickness * 4.0 + 2.0) * (0.7 + 0.9 * ray);
        let rise = (edge - y) / curtain;
        if rise >= 1.0 {
            return 0.0;
        }
        // the lower edge burns brightest all along; above it the folds fade at their own rate
        if rise < 0.1 {
            return 1.0;
        }
        (1.0 - rise).powf(1.4) * (0.6 + 0.4 * ray)
    }

    fn paint(&self, canvas: &mut Canvas, c: &AuroraColours) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / tall.max(1) as f32));
        }
        for &(x, y) in &self.stars {
            if c.dark || y < tall / 3 {
                canvas.set(x, y, c.star);
            }
        }

        let breaths: Vec<f32> = self
            .bands
            .iter()
            .map(|band| {
                // the thickness breathes a little, once every 21 seconds or 42 under reduced
                // motion
                let breath = cycle(self.t, BREATH_RATE * self.motion.pace()) + band.breath;
                band.thickness * (1.0 + 0.25 * breath.sin())
            })
            .collect();
        for y in 0..tall {
            let step = c.sky_step(y as f32 / tall.max(1) as f32);
            for x in 0..wide {
                let mut best = (0.0_f32, 0_usize);
                for (band, &thickness) in self.bands.iter().zip(&breaths) {
                    let g = self.glow(band, thickness, x, y);
                    if g > best.0 {
                        best = (g, band.colour);
                    }
                }
                if best.0 > 0.08 {
                    let ramp = &c.glow[best.1 % c.glow.len()][step];
                    canvas.set(x, y, ramp.at(best.0));
                }
            }
        }

        // two ranges of mountains, the far one paler, each peak with a lit face and a shaded one
        // split along a ridge line that runs down from its summit with snow on the high peaks
        let [far, near] = c.ranges;
        self.paint_range(canvas, &self.far, far, 0.55, 31);
        self.paint_range(canvas, &self.near, near, 0.72, 37);
    }

    fn paint_range(
        &self,
        canvas: &mut Canvas,
        peaks: &[Peak],
        tones: RangeTones,
        snow_from: f32,
        salt: u32,
    ) {
        let tall = canvas.height() as i32;
        let highest = peaks.iter().map(|p| p.height).fold(0.0_f32, f32::max);
        // snow lies only on the peaks that rise well up the range, deeper the higher they are
        let snowline = highest * snow_from;
        for x in 0..canvas.width() as i32 {
            let xf = x as f32 + 0.5;
            let tops: Vec<f32> = peaks.iter().map(|p| p.top_at(xf, salt)).collect();
            let top = tops.iter().copied().fold(0.0_f32, f32::max);
            for y in (tall - top.ceil() as i32).max(0)..tall {
                let above = (tall - y) as f32 - 0.5;
                // the lowest peak that covers a pixel stands in front of the taller ones
                let Some(i) = (0..peaks.len())
                    .filter(|&i| above <= tops[i])
                    .min_by(|&a, &b| peaks[a].height.total_cmp(&peaks[b].height))
                else {
                    continue;
                };
                let p = peaks[i];
                let down = p.height - above;
                let wander = (paint::noise(down * 0.3, p.x, salt + 1) - 0.5) * (1.0 + down * 0.3);
                // the ridge line runs down and out from the summit, so each face is a slope
                let shaded = xf - p.x > down * 0.6 + wander;
                let cap = (p.height - snowline) * 0.8;
                // the snow's lower edge is ragged, with tongues reaching down the gullies, so the
                // snowline never runs straight across a peak
                let ragged = (paint::noise(xf * 0.2, p.x, salt + 2) - 0.5) * cap * 0.7;
                let tongue = paint::noise(xf * 0.3, p.x, salt + 3) > 0.65;
                let reach = cap + ragged + if tongue { cap * 0.35 } else { 0.0 };
                let snowy = cap > 0.0 && down < reach;
                let colour = match (snowy, shaded) {
                    (true, false) => tones.snow,
                    (true, true) => tones.snow_shade,
                    (false, false) => tones.lit,
                    (false, true) => tones.shade,
                };
                canvas.set(x, y, colour);
            }
        }
    }
}

/// A range of peaks across a canvas `width` wide, each between `low` and `high` pixels tall,
/// spaced `gap` apart, with flanks falling between `steep.0` and `steep.1` a column.
fn range(
    rng: &mut Rng,
    width: f32,
    low: f32,
    high: f32,
    gap: (f32, f32),
    steep: (f32, f32),
) -> Vec<Peak> {
    let mut peaks = Vec::new();
    let mut x = -rng.range(0.0, gap.0);
    while x < width + gap.0 {
        peaks.push(Peak {
            x,
            height: rng.range(low, high),
            left: rng.range(steep.0, steep.1),
            right: rng.range(steep.0, steep.1),
        });
        x += rng.range(gap.0, gap.1);
    }
    peaks
}

impl Scene for Aurora {
    fn name(&self) -> &'static str {
        "aurora"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let tall = self.tall();
        let width = self.width as f32;
        let count = if self.height < 10 { 2 } else { 3 };
        let bases = [0.17, 0.3, 0.42];
        self.bands = (0..count)
            .map(|i| Band {
                base: bases[i],
                amplitude: (tall * 0.06).max(1.0),
                wavelength: rng.range(width / 3.0, width / 1.5).max(4.0),
                phase: rng.range(0.0, TAU),
                speed: rng.range(0.45, 0.75) * self.motion.speed(),
                thickness: (tall * 0.03).clamp(1.0, 3.5),
                breath: rng.range(0.0, TAU),
                colour: i,
            })
            .collect();
        let stars = count_for(self.width, self.height, 40, 3, 200);
        self.stars = (0..stars)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(self.height * 2) as i32,
                )
            })
            .collect();
        // a far range of tall peaks behind a nearer, lower one; the flanks are scaled with the
        // picture so the mountains keep their shape at every size
        let peak = (tall / 2.6).max(3.0);
        let spread = (width / 80.0).clamp(0.6, 3.0);
        self.far = range(
            rng,
            width,
            peak * 0.55,
            peak,
            (12.0 * spread, 26.0 * spread),
            (0.5, 0.85),
        );
        self.near = range(
            rng,
            width,
            peak * 0.28,
            peak * 0.62,
            (9.0 * spread, 18.0 * spread),
            (0.45, 0.8),
        );
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        for band in &mut self.bands {
            // ten turns is a whole number of turns for the detail wave too, which runs 1.7
            // times as fast, so the wrap never shows
            band.phase = (band.phase + band.speed * dt) % (10.0 * TAU);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = AuroraColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::distance;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("aurora");
        check_determinism("aurora");
        check_pixel_scene("aurora", 6.0);
    }

    #[test]
    fn the_sky_ridge_and_curtains_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = AuroraColours::new(theme);
            let name = theme.name;
            assert!(distance(c.sky_top, c.sky_low) >= 0.03, "{name}");
            assert!(distance(c.ridge, c.sky_low) >= 0.03, "{name}");
            assert!(distance(c.snow, c.ridge) >= 0.03, "{name}");
            for colour in [c.sky_top, c.sky_low, c.snow] {
                assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
            }
            if theme.dark {
                assert!(theme.contrast_of(c.ridge) < IMAGERY_CEILING, "{name}");
                assert!(theme.contrast_of(c.star) >= IMAGERY_CEILING, "{name}");
                for band in c.bands {
                    assert!(theme.contrast_of(band) >= theme.floor(), "{name}");
                    assert!(distance(band, c.sky_top) >= 0.1, "{name}");
                }
            } else {
                // at the blue hour the curtains are pale light and the mountains keep the
                // outline
                assert!(theme.contrast_of(c.ridge) >= IMAGERY_CEILING, "{name}");
                assert!(
                    theme.contrast_of(c.ridge_shade) >= IMAGERY_CEILING,
                    "{name}"
                );
                for band in c.bands {
                    assert!(theme.contrast_of(band) < IMAGERY_CEILING, "{name}");
                }
                // the three curtains drawn stay clear of the deep sky they hang in
                for band in &c.bands[..3] {
                    let gap = distance(*band, c.sky_top);
                    assert!(gap >= 0.06, "{name}: {gap:.3}");
                }
            }
            // each curtain fades from the sky to its own colour, faint at first
            for ramps in &c.glow {
                for ramp in ramps {
                    assert!(theme.contrast_of(ramp.at(0.2)) < IMAGERY_CEILING, "{name}");
                }
            }
        }
    }

    #[test]
    fn bands_sit_in_the_upper_half_and_move_slowly() {
        let theme = theme::default_theme();
        let c = AuroraColours::new(theme);
        let mut rng = Rng::new(13);
        let mut aurora = Aurora::new(80, 24, Motion::FULL, &mut rng);
        let first = render(&mut aurora, 80, 24, 1);
        let lit = |frame: &Frame, y: usize| {
            (0..80)
                .filter_map(|x| frame.get(x, y as i32).and_then(|cell| cell.pixel_pair()))
                .flat_map(|(a, b)| [a, b])
                .filter(|p| c.bands.contains(p))
                .count()
        };
        let upper: usize = (0..12).map(|y| lit(&first, y)).sum();
        let lower: usize = (16..24).map(|y| lit(&first, y)).sum();
        assert!(upper > 30, "{upper}");
        assert_eq!(lower, 0);
        let later = render(&mut aurora, 80, 24, 40);
        assert_ne!(first, later);
        // the ridge stands along the bottom
        let (_, bottom) = first.get(5, 23).unwrap().pixel_pair().unwrap();
        assert!(bottom == c.ridge || bottom == c.ridge_shade);
    }

    #[test]
    fn both_ranges_stand_out_from_the_sky_and_each_other() {
        for theme in theme::all() {
            let c = AuroraColours::new(theme);
            let [far, near] = c.ranges;
            for (a, b, what) in [
                (far.lit, far.shade, "the far faces"),
                (far.lit, c.sky_low, "the far range and the sky"),
                (far.shade, c.sky_low, "the far shade and the sky"),
                (near.lit, far.shade, "the near range and the far one"),
                (far.snow, far.lit, "the far snow"),
                (near.snow, near.lit, "the near snow"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn the_mountains_slope_with_no_cliffs() {
        for (w, h) in [(40, 12), (80, 24), (200, 50), (400, 120)] {
            let mut rng = Rng::new(21);
            let aurora = Aurora::new(w, h, Motion::FULL, &mut rng);
            for (peaks, salt) in [(&aurora.far, 31), (&aurora.near, 37)] {
                let outline: Vec<f32> = (0..w)
                    .map(|x| {
                        peaks
                            .iter()
                            .map(|p| p.top_at(x as f32 + 0.5, salt))
                            .fold(0.0_f32, f32::max)
                    })
                    .collect();
                for pair in outline.windows(2) {
                    assert!((pair[0] - pair[1]).abs() < 2.5, "a cliff at {w}x{h}");
                }
            }
        }
    }

    #[test]
    fn a_calm_aurora_moves_at_half_speed() {
        let mut rng = Rng::new(13);
        let full = Aurora::new(40, 12, Motion::FULL, &mut rng);
        let mut rng = Rng::new(13);
        let calm = Aurora::new(40, 12, Motion::CALM, &mut rng);
        for (a, b) in full.bands.iter().zip(&calm.bands) {
            assert!((a.speed - 2.0 * b.speed).abs() < 1e-5);
        }
    }
}
