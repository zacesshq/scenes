//! Dead LED: the bench from `pcb-bring-up`, the same board, lamp, scope and supply, where the
//! chain lights up to one LED that stays dark. The LEDs past it light, the status LED and the
//! "POWER GOOD" legend never do and the probe is lifted off the mat and moved slowly over to
//! the dark LED, where it rests on its pad. The scope beside it sweeps along a flat trace.
//! Once the probe is there nothing changes but the slow sweep of the beam.
//!
//! The drawing and its colours are shared with `pcb-bring-up` through `Bench`, so the pass and
//! the fail scene read as the same bench on a good day and a bad one.

use super::pcb_bring_up::{Bench, Story};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "The same bench with one LED in the chain left dark, a probe moving to it and a flat trace on the scope, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "One light stayed dark. Probe the first error.",
    "Something in the chain is open.",
    "The trace is flat. Follow it back.",
];

pub struct DeadLed {
    bench: Bench,
}

impl DeadLed {
    pub fn new(width: usize, height: usize, motion: Motion, _rng: &mut Rng) -> Self {
        Self {
            bench: Bench::new(width, height, motion, Story::DeadLed),
        }
    }
}

impl Scene for DeadLed {
    fn name(&self) -> &'static str {
        "dead-led"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.bench.resize(width, height);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.bench.step(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        self.bench.draw(frame, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::pcb_bring_up::{Colours, Led, PROBE_MOVE};
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene};
    use crate::scenes::testing::{pixels, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("dead-led");
        check_determinism("dead-led");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("dead-led", 30.0);
    }

    #[test]
    fn the_dead_led_never_lights_and_the_probe_reaches_it() {
        for motion in Motion::ALL {
            let mut bench = Bench::new(80, 24, motion, Story::DeadLed);
            let (dead, n) = (bench.dead(), bench.chain());
            let start = bench.probe().expect("probe").0;
            let mut longest_step = 0.0_f32;
            let mut last = start;
            for _ in 0..(90 * 20) {
                bench.step(0.05);
                assert_eq!(bench.led(dead), Led::Dark, "{motion:?}");
                assert!(!bench.good(), "{motion:?}");
                let tip = bench.probe().expect("probe").0;
                longest_step = longest_step.max((tip.0 - last.0).hypot(tip.1 - last.1));
                last = tip;
            }
            // every other LED in the chain has lit
            for i in (0..n).filter(|&i| i != dead) {
                assert_eq!(bench.led(i), Led::Lit, "{motion:?} {i}");
            }
            // the tip rests on the dark LED, having moved there slowly
            let (lx, ly) = bench.leds()[dead];
            let tip = bench.probe().expect("probe").0;
            assert!(
                (tip.0 - lx as f32).abs() <= 4.0 && (tip.1 - ly as f32).abs() <= 3.0,
                "{motion:?}: {tip:?} against {lx},{ly}"
            );
            let distance = (tip.0 - start.0).hypot(tip.1 - start.1);
            assert!(longest_step <= distance / (PROBE_MOVE * 20.0) * 2.0 + 0.01);
        }
    }

    #[test]
    fn the_scope_trace_stays_flat_and_the_legend_stays_dim() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(1);
        let mut scene = DeadLed::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 100, 30, 40 * 20);
        let all = pixels(&frame);
        assert!(!all.contains(&c.green.0), "the status LED lit");
        assert!(!all.contains(&c.green_glow.0), "the legend glows");
        assert!(all.contains(&c.lens), "no dark LED");
        // the trace pixels on the scope sit on one row
        let rows: Vec<usize> = (0..60)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let pair = frame.get(x, y / 2).and_then(|cell| cell.pixel_pair());
                pair.is_some_and(|(top, bottom)| (if y % 2 == 0 { top } else { bottom }) == c.trace)
            })
            .map(|(_, y)| y as usize)
            .collect();
        assert!(!rows.is_empty(), "no trace");
        assert!(rows.iter().all(|&y| y == rows[0]), "{rows:?}");
        let text = frame.to_text();
        assert!(text.contains("POWER GOOD"));
    }
}
