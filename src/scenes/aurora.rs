//! Aurora: slow waves of colour across the top of a starry sky over a mountain ridge.

use std::f32::consts::TAU;

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

#[derive(Clone, Debug)]
struct Band {
    /// The band's centre as a fraction of the height.
    base: f32,
    amplitude: f32,
    wavelength: f32,
    phase: f32,
    speed: f32,
    thickness: f32,
    colour: usize,
}

pub struct Aurora {
    width: usize,
    height: usize,
    motion: Motion,
    t: f32,
    bands: Vec<Band>,
    stars: Vec<(i32, i32)>,
    ridge: Vec<i32>,
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
            ridge: Vec::new(),
        };
        aurora.resize(width, height, rng);
        aurora
    }

    fn centre(&self, band: &Band, x: f32) -> f32 {
        let height = self.height as f32;
        let main = (x / band.wavelength * TAU + band.phase).sin();
        let detail = (x / (band.wavelength * 0.37) + band.phase * 1.7).sin();
        band.base * height + band.amplitude * (main + 0.5 * detail)
    }
}

impl Scene for Aurora {
    fn name(&self) -> &'static str {
        "aurora"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let height = self.height as f32;
        let width = self.width as f32;
        let count = if self.height < 10 { 2 } else { 3 };
        let bases = [0.17, 0.32, 0.46];
        self.bands = (0..count)
            .map(|i| Band {
                base: bases[i],
                amplitude: (height * 0.08).max(0.6),
                wavelength: rng.range(width / 3.0, width / 1.5).max(4.0),
                phase: rng.range(0.0, TAU),
                speed: rng.range(0.45, 0.75) * self.motion.speed(),
                thickness: (height * 0.045).clamp(0.6, 2.2),
                colour: i,
            })
            .collect();
        let stars = count_for(self.width, self.height, 40, 3, 200);
        self.stars = (0..stars)
            .map(|_| (rng.below(self.width) as i32, rng.below(self.height) as i32))
            .collect();
        let peak = ((self.height / 5) as f32).max(1.0);
        let (l1, l2) = (rng.range(9.0, 18.0), rng.range(3.0, 6.0));
        let (p1, p2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        self.ridge = (0..self.width)
            .map(|x| {
                let x = x as f32;
                let h = 1.0
                    + peak * 0.6 * (1.0 + (x / l1 + p1).sin()) / 2.0
                    + peak * 0.4 * (1.0 + (x / l2 + p2).sin()) / 2.0;
                h.round() as i32
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += dt;
        for band in &mut self.bands {
            band.phase += band.speed * dt;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        for &(x, y) in &self.stars {
            frame.put(x, y, '.', Some(theme.muted));
        }
        let colours = [theme.green(), theme.cyan(), theme.magenta(), theme.blue()];
        for band in &self.bands {
            let colour = Some(colours[band.colour % colours.len()]);
            // the thickness breathes a little over about twenty seconds
            let thickness = band.thickness * (1.0 + 0.25 * (self.t * 0.3 + band.phase).sin());
            for x in 0..self.width {
                let centre = self.centre(band, x as f32);
                let top = (centre - thickness).floor() as i32;
                let bottom = (centre + thickness).ceil() as i32;
                for y in top..=bottom {
                    let distance = ((y as f32 + 0.5 - centre) / thickness).abs();
                    if distance > 1.0 {
                        continue;
                    }
                    let ch = if distance < 0.35 {
                        '\u{2593}'
                    } else if distance < 0.7 {
                        '\u{2592}'
                    } else {
                        '\u{2591}'
                    };
                    frame.put(x as i32, y, ch, colour);
                }
            }
        }
        let bottom = self.height as i32 - 1;
        for (x, &h) in self.ridge.iter().enumerate() {
            let next = self.ridge.get(x + 1).copied().unwrap_or(h);
            let ch = if next > h {
                '/'
            } else if next < h {
                '\\'
            } else {
                '^'
            };
            frame.put(x as i32, bottom - h + 1, ch, Some(theme.grey()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("aurora");
        check_determinism("aurora");
    }

    #[test]
    fn bands_sit_in_the_upper_half_and_move_slowly() {
        let mut rng = Rng::new(13);
        let mut aurora = Aurora::new(80, 24, Motion::FULL, &mut rng);
        let first = render(&mut aurora, 80, 24, 1);
        let shaded = |frame: &Frame, y: usize| {
            frame
                .row_text(y)
                .chars()
                .filter(|c| matches!(c, '\u{2591}' | '\u{2592}' | '\u{2593}'))
                .count()
        };
        let upper: usize = (0..12).map(|y| shaded(&first, y)).sum();
        let lower: usize = (14..24).map(|y| shaded(&first, y)).sum();
        assert!(upper > 60, "{upper}");
        assert_eq!(lower, 0);
        let later = render(&mut aurora, 80, 24, 40);
        assert_ne!(first.to_text(), later.to_text());
        assert!(first.row_text(23).contains('^') || first.row_text(22).contains('^'));
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
