//! Ambient sound: a quiet bed of water, rain, wind, fire or night sounds that plays the whole
//! time a scene runs, under the short event tones. Like the tones it is synthesised at run time
//! into a WAV file in the cache folder and played by the system's own player, so nothing is
//! bundled and no audio library is linked.
//!
//! Each ambience is one seamless loop of about twenty seconds, built from seeded noise through
//! simple filters. The seed is fixed per ambience, so the same loop is always written the same
//! way and the cached file stays valid. The loop is levelled well below the tones, so an event
//! always stands out over it.
//!
//! The players have no loop option, so the file is played again each time the player exits.
//! That leaves a gap of a few milliseconds at the join, which the soft edges of the loop hide.

use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::rng::Rng;
use crate::sound::{self, Clock, Player, QuietHours, SAMPLE_RATE, VOLUME_STEP};

/// The default ambient volume, out of 100. Lower than the tones' default, since it plays all the
/// time.
pub const DEFAULT_VOLUME: u8 = 20;
/// How long one loop lasts, in seconds. The slow swells and gusts repeat a whole number of times
/// in it, so the join never falls mid-swell at a different level.
pub const LOOP_SECONDS: f32 = 20.0;
/// How much of the end is faded into the start, in seconds, so the join is seamless.
const CROSSFADE: f32 = 1.0;
/// The loudness of a loop at volume 100, as a share of full scale. The tones are set to 0.2 in
/// `sound` (private there), so this is about a third of their level.
const TARGET_RMS: f32 = 0.07;
/// The loudest sample at volume 100, the same ceiling the tones keep in `sound` (private there).
const PEAK: f32 = 0.7;
/// The highest pitch synthesised. The tones stop at 6 kHz for the same reason: higher is harsh
/// and nears half the sample rate.
const HIGHEST_PITCH: f32 = 5000.0;
/// Bumped whenever the synthesis changes, so a cached file from an older version is not reused.
const AMBIENT_VERSION: u32 = 1;

/// The kinds of ambient sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ambience {
    /// Slow swells of low surf.
    Water,
    /// A steady soft hiss with the odd drop.
    Rain,
    /// Low noise rising and falling in gusts.
    Wind,
    /// A low rumble with sparse soft crackles.
    Fire,
    /// A very quiet bed with the occasional cricket.
    Night,
}

/// Which ambience each built-in scene plays. A scene not in the table, such as a user's Lua
/// scene, plays the night ambience, the quietest.
const SCENE_AMBIENCE: [(&str, Ambience); 39] = [
    ("sea", Ambience::Water),
    ("rain", Ambience::Rain),
    ("snow", Ambience::Wind),
    ("starfield", Ambience::Night),
    ("code-rain", Ambience::Night),
    ("fireflies", Ambience::Night),
    ("aurora", Ambience::Wind),
    ("campfire", Ambience::Fire),
    ("waves", Ambience::Water),
    ("zacess", Ambience::Night),
    ("koi-pond", Ambience::Water),
    ("harbour", Ambience::Water),
    ("lanterns", Ambience::Water),
    ("balloons", Ambience::Wind),
    ("countryside", Ambience::Wind),
    // the lit window and chimney smoke make the stove the sound of the cabin, not the snow
    ("winter-cabin", Ambience::Fire),
    ("autumn", Ambience::Wind),
    ("blossom", Ambience::Wind),
    ("city-night", Ambience::Night),
    ("orbit", Ambience::Night),
    ("constellations", Ambience::Night),
    // the weather at the window is the loudest thing in a quiet room
    ("tea", Ambience::Rain),
    ("cat-nap", Ambience::Night),
    ("garden", Ambience::Night),
    ("tiles", Ambience::Night),
    ("ribbons", Ambience::Night),
    ("storm", Ambience::Rain),
    ("flood", Ambience::Water),
    ("listing-ship", Ambience::Water),
    ("corrosion", Ambience::Night),
    ("tangle", Ambience::Night),
    ("wilt", Ambience::Wind),
    ("fog", Ambience::Wind),
    ("broken-bridge", Ambience::Water),
    ("leak", Ambience::Water),
    ("blackout", Ambience::Night),
    ("traffic-jam", Ambience::Night),
    ("erosion", Ambience::Water),
    ("drought", Ambience::Wind),
];

impl Ambience {
    pub const ALL: [Ambience; 5] = [
        Ambience::Water,
        Ambience::Rain,
        Ambience::Wind,
        Ambience::Fire,
        Ambience::Night,
    ];

    /// The name used in file names and settings.
    pub fn name(self) -> &'static str {
        match self {
            Ambience::Water => "water",
            Ambience::Rain => "rain",
            Ambience::Wind => "wind",
            Ambience::Fire => "fire",
            Ambience::Night => "night",
        }
    }

    pub fn parse(text: &str) -> Option<Ambience> {
        let key = text.trim().to_ascii_lowercase();
        Ambience::ALL.into_iter().find(|a| a.name() == key)
    }

    /// The ambience a scene plays, the night ambience for any scene not in the table.
    pub fn for_scene(name: &str) -> Ambience {
        SCENE_AMBIENCE
            .iter()
            .find(|(scene, _)| *scene == name)
            .map_or(Ambience::Night, |&(_, ambience)| ambience)
    }

    /// The seed of the ambience's noise, fixed so the loop is the same every time.
    fn seed(self) -> u64 {
        0xa4b1_0000 + self as u64
    }
}

fn samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32).round() as usize
}

/// The coefficient of a one pole low-pass filter with this cut-off.
fn pole(cutoff: f32) -> f32 {
    1.0 - (-TAU * cutoff / SAMPLE_RATE as f32).exp()
}

/// A one pole low-pass filter.
#[derive(Clone, Copy)]
struct LowPass {
    k: f32,
    y: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> Self {
        Self {
            k: pole(cutoff),
            y: 0.0,
        }
    }

    fn step(&mut self, x: f32) -> f32 {
        self.y += self.k * (x - self.y);
        self.y
    }
}

/// A slow swell from 0 to 1 that repeats `count` times a loop, so it lines up at the join.
fn swell(t: f32, count: f32) -> f32 {
    0.5 - 0.5 * (TAU * count * t / LOOP_SECONDS).cos()
}

/// A short soft burst: a few milliseconds up, then an exponential fall, so it never clicks.
fn burst(age: f32, attack: f32, decay: f32) -> f32 {
    if age < 0.0 {
        0.0
    } else if age < attack {
        age / attack
    } else {
        (-(age - attack) / decay).exp()
    }
}

/// Events at random times: their start times over `count` samples, about `per_second` of them.
fn event_times(rng: &mut Rng, count: usize, per_second: f32) -> Vec<f32> {
    let total = count as f32 / SAMPLE_RATE as f32;
    let mut times = Vec::new();
    let mut t = rng.range(0.0, 2.0 / per_second);
    while t < total {
        times.push(t);
        t += rng.range(0.3, 1.7) / per_second;
    }
    times
}

/// Adds short decaying sine bursts at the given times.
fn add_chimes(mix: &mut [f32], times: &[(f32, f32, f32)], attack: f32, decay: f32) {
    let length = attack + decay * 6.0;
    for &(start, freq, level) in times {
        let freq = freq.min(HIGHEST_PITCH);
        let first = samples(start);
        for i in 0..samples(length) {
            let Some(slot) = mix.get_mut(first + i) else {
                break;
            };
            let age = i as f32 / SAMPLE_RATE as f32;
            *slot += level * burst(age, attack, decay) * (TAU * freq * age).sin();
        }
    }
}

/// The raw mix of an ambience over `count` samples, before the loop is joined and levelled.
fn raw(ambience: Ambience, count: usize) -> Vec<f32> {
    let mut rng = Rng::new(ambience.seed());
    let mut noise = || rng.unit() * 2.0 - 1.0;
    let mut mix = vec![0.0_f32; count];
    let time = |i: usize| i as f32 / SAMPLE_RATE as f32;
    match ambience {
        Ambience::Water => {
            let (mut low, mut lower, mut foam) = (
                LowPass::new(350.0),
                LowPass::new(350.0),
                LowPass::new(1400.0),
            );
            for (i, slot) in mix.iter_mut().enumerate() {
                let t = time(i);
                let white = noise();
                let body = lower.step(low.step(white));
                // three waves a loop, with a slower fourth beneath so they are not all alike
                let wave = 0.65 * swell(t, 3.0).powi(2) + 0.35 * swell(t, 2.0);
                let hiss = foam.step(white) - body;
                *slot = (0.15 + 0.85 * wave) * body * 4.0 + 0.25 * wave.powi(3) * hiss;
            }
        }
        Ambience::Rain => {
            let (mut high, mut low) = (LowPass::new(3500.0), LowPass::new(400.0));
            for slot in mix.iter_mut() {
                let white = noise();
                let bright = high.step(white);
                // the band between the two filters: a hiss with the rumble taken out
                *slot = bright - low.step(bright);
            }
            let mut drops = Rng::new(ambience.seed() ^ 0xd209);
            let times: Vec<_> = event_times(&mut drops, count, 5.0)
                .into_iter()
                .map(|t| (t, drops.range(1200.0, 2600.0), drops.range(0.08, 0.2)))
                .collect();
            add_chimes(&mut mix, &times, 0.003, 0.012);
        }
        Ambience::Wind => {
            let (mut low, mut lower, mut whistle) = (
                LowPass::new(260.0),
                LowPass::new(260.0),
                LowPass::new(900.0),
            );
            for (i, slot) in mix.iter_mut().enumerate() {
                let t = time(i);
                let white = noise();
                let body = lower.step(low.step(white));
                let gust = 0.6 * swell(t, 2.0) + 0.4 * swell(t, 5.0);
                let edge = whistle.step(white) - body;
                *slot = (0.25 + 0.75 * gust) * body * 4.0 + 0.12 * gust.powi(2) * edge;
            }
        }
        Ambience::Fire => {
            let (mut low, mut lower) = (LowPass::new(140.0), LowPass::new(140.0));
            for (i, slot) in mix.iter_mut().enumerate() {
                let flicker = 0.8 + 0.2 * swell(time(i), 4.0);
                *slot = flicker * lower.step(low.step(noise())) * 5.0;
            }
            // crackles: short bursts of filtered noise, each a few milliseconds up and a few tens
            // down, so they pop softly rather than click
            let mut sparks = Rng::new(ambience.seed() ^ 0xc4ac);
            let mut grain = LowPass::new(2500.0);
            for start in event_times(&mut sparks, count, 4.0) {
                let level = sparks.range(0.15, 0.45);
                let decay = sparks.range(0.008, 0.03);
                let first = samples(start);
                for i in 0..samples(0.004 + decay * 6.0) {
                    let Some(slot) = mix.get_mut(first + i) else {
                        break;
                    };
                    let age = i as f32 / SAMPLE_RATE as f32;
                    *slot +=
                        level * burst(age, 0.004, decay) * grain.step(sparks.unit() * 2.0 - 1.0);
                }
            }
        }
        Ambience::Night => {
            let (mut low, mut lower) = (LowPass::new(180.0), LowPass::new(180.0));
            for slot in mix.iter_mut() {
                *slot = lower.step(low.step(noise())) * 1.5;
            }
            // a cricket: three to five soft pulses close together, every couple of seconds
            let mut crickets = Rng::new(ambience.seed() ^ 0xc41c);
            let mut times = Vec::new();
            for start in event_times(&mut crickets, count, 0.45) {
                let freq = crickets.range(4200.0, 4800.0);
                let level = crickets.range(0.05, 0.1);
                for pulse in 0..3 + crickets.below(3) {
                    times.push((start + pulse as f32 * 0.045, freq, level));
                }
            }
            add_chimes(&mut mix, &times, 0.005, 0.006);
        }
    }
    mix
}

/// The root mean square of the whole loop.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// The 16 bit samples of an ambience's loop at a volume from 0 to 100. The last second is faded
/// into the first, so playing the loop end to end has no join.
pub fn synthesise(ambience: Ambience, volume: u8) -> Vec<i16> {
    let length = samples(LOOP_SECONDS);
    let fade = samples(CROSSFADE);
    let extra = raw(ambience, length + fade);
    let mut mix: Vec<f32> = extra[..length].to_vec();
    // the samples past the end carry on from the last one, so fading them into the start makes
    // the last sample run straight on into the first
    for i in 0..fade {
        let t = i as f32 / fade as f32;
        mix[i] = extra[i] * t + extra[length + i] * (1.0 - t);
    }
    let volume = f32::from(volume.min(100)) / 100.0;
    let loud = rms(&mix);
    let peak = mix.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let gain = if loud > 0.0 && peak > 0.0 {
        (TARGET_RMS / loud).min(PEAK / peak) * volume
    } else {
        0.0
    };
    mix.iter()
        .map(|s| {
            (s * gain * f32::from(i16::MAX))
                .round()
                .clamp(-f32::from(i16::MAX), f32::from(i16::MAX)) as i16
        })
        .collect()
}

/// The size of a loop's WAV file, known without synthesising it.
fn wav_size() -> u64 {
    44 + 2 * samples(LOOP_SECONDS) as u64
}

/// Writes an ambience's WAV file into `dir` unless one is already there. Returns its path. The
/// size is checked without synthesising, so a cached loop costs nothing to find again.
pub fn ensure_wav(dir: &Path, ambience: Ambience, volume: u8) -> std::io::Result<PathBuf> {
    let path = dir.join(format!(
        "ambient-{}-v{volume}-s{AMBIENT_VERSION}.wav",
        ambience.name()
    ));
    let current = std::fs::metadata(&path).is_ok_and(|m| m.len() == wav_size());
    if !current {
        std::fs::create_dir_all(dir)?;
        // written beside the target and renamed, so a player never reads half a file
        let partial = path.with_extension(format!("wav.{}", std::process::id()));
        std::fs::write(&partial, sound::wav_bytes(&synthesise(ambience, volume)))?;
        std::fs::rename(&partial, &path)?;
    }
    Ok(path)
}

/// The command that plays one file once. `sound` keeps its own copy private, so this matches it.
fn play_command(player: &Player, file: &Path) -> Command {
    match player {
        Player::Command(program) => {
            let mut command = Command::new(program);
            // aplay prints a line about the format for every file unless asked not to
            if program.file_stem().is_some_and(|s| s == "aplay") {
                command.arg("-q");
            }
            command.arg(file);
            command
        }
        Player::PowerShell(program) => {
            // single quotes inside a PowerShell single quoted string are doubled
            let quoted = file.display().to_string().replace('\'', "''");
            let mut command = Command::new(program);
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("(New-Object System.Media.SoundPlayer '{quoted}').PlaySync()"),
            ]);
            command
        }
    }
}

/// The ambient sound of a running scene: whether it is on, its volume, quiet hours, the scene's
/// ambience and the player playing the loop.
pub struct Ambient {
    enabled: bool,
    volume: u8,
    ambience: Ambience,
    quiet: Option<(QuietHours, Clock)>,
    player: Option<Player>,
    dir: PathBuf,
    child: Option<Child>,
    paused: bool,
    ducked_until: Option<Instant>,
    /// The file last written, so a respawn at the join never synthesises again.
    file: Option<(Ambience, u8, PathBuf)>,
    /// When the player was last started, to tell a loop that ended from one that failed.
    started_at: Option<Instant>,
    /// No new attempt before this, after the player failed to play or the file to be written.
    retry_at: Option<Instant>,
}

/// A player that exits sooner than this after starting failed rather than finished the loop.
const QUICK_EXIT: Duration = Duration::from_secs(2);
/// How long to wait before trying again after a failure, so a system with no audio device never
/// starts a player on every frame.
const RETRY_AFTER: Duration = Duration::from_secs(30);

impl Ambient {
    /// Sets ambient sound up, finding the player once. Ambient sound that is off at the start
    /// can still be turned on later, so the player is looked for either way.
    pub fn new(enabled: bool, volume: u8) -> Self {
        Self {
            enabled,
            volume: volume.min(100),
            ambience: Ambience::Night,
            quiet: None,
            player: sound::find_player(),
            dir: sound::cache_dir(),
            child: None,
            paused: false,
            ducked_until: None,
            file: None,
            started_at: None,
            retry_at: None,
        }
    }

    /// No ambient sound inside these hours. Starts the clock only when there are quiet hours.
    pub fn with_quiet_hours(mut self, quiet: Option<QuietHours>) -> Self {
        self.quiet = quiet.map(|q| (q, Clock::start()));
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        // a key press is a fresh request, so it tries again straight away after a failure
        self.retry_at = None;
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn ambience(&self) -> Ambience {
        self.ambience
    }

    /// Whether a player was found.
    pub fn available(&self) -> bool {
        self.player.is_some()
    }

    /// Raises or lowers the volume by one step, within 0 to 100. The loop starts again at the
    /// new level on the next tick.
    pub fn change_volume(&mut self, up: bool) -> u8 {
        self.volume = if up {
            self.volume.saturating_add(VOLUME_STEP).min(100)
        } else {
            self.volume.saturating_sub(VOLUME_STEP)
        };
        self.retry_at = None;
        self.kill();
        self.volume
    }

    /// Follows the scene on screen, switching the loop only when its ambience differs.
    pub fn set_scene(&mut self, name: &str) {
        let ambience = Ambience::for_scene(name);
        if ambience != self.ambience {
            self.ambience = ambience;
            self.kill();
        }
    }

    /// Stops the loop while the scene is paused and starts it again on resume.
    pub fn pause(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Stops the loop for a moment so an event tone is heard clearly, then lets it start again.
    /// The players are separate programs with no volume control while playing, so this is a
    /// short pause rather than a fade.
    pub fn duck(&mut self, length: Duration) {
        self.ducked_until = Some(Instant::now() + length);
        self.kill();
    }

    /// Whether the loop should be playing at this minute of the day and this moment, apart from
    /// whether a player exists.
    pub fn should_play(&self, now_minute: Option<u16>, now: Instant) -> bool {
        let quiet = self
            .quiet
            .as_ref()
            .is_some_and(|(hours, _)| now_minute.is_some_and(|m| hours.contains(m)));
        self.enabled
            && self.volume > 0
            && !self.paused
            && self.ducked_until.is_none_or(|until| now >= until)
            && !quiet
    }

    /// Starts, restarts or stops the player to match the settings. Called every frame or so:
    /// when the loop has played to its end the player has exited and is started again.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let minute = self.quiet.as_ref().and_then(|(_, clock)| clock.now());
        if !self.should_play(minute, now) {
            self.kill();
            return;
        }
        if self.ducked_until.is_some_and(|until| now >= until) {
            self.ducked_until = None;
        }
        let running = self
            .child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if running {
            return;
        }
        if self.child.is_some()
            && self
                .started_at
                .is_some_and(|at| now.duration_since(at) < QUICK_EXIT)
        {
            self.child = None;
            self.retry_at = Some(now + RETRY_AFTER);
        }
        if self.retry_at.is_some_and(|at| now < at) {
            return;
        }
        self.retry_at = None;
        self.spawn();
        if self.child.is_none() {
            self.retry_at = Some(now + RETRY_AFTER);
        }
    }

    /// Stops the loop. It starts again on the next tick when it should be playing.
    pub fn stop(&mut self) {
        self.kill();
    }

    fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Plays the loop once in the background. Any failure, from a missing player to a full
    /// disk, is silent: ambient sound only ever adds to what is on screen.
    fn spawn(&mut self) {
        self.kill();
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let key = (self.ambience, self.volume);
        let file = match &self.file {
            Some((ambience, volume, path)) if (*ambience, *volume) == key => path.clone(),
            _ => match ensure_wav(&self.dir, self.ambience, self.volume) {
                Ok(path) => {
                    self.file = Some((key.0, key.1, path.clone()));
                    path
                }
                Err(_) => return,
            },
        };
        self.child = play_command(player, &file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        self.started_at = Some(Instant::now());
    }
}

impl Drop for Ambient {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn as_float(samples: &[i16]) -> Vec<f32> {
        samples
            .iter()
            .map(|&s| f32::from(s) / f32::from(i16::MAX))
            .collect()
    }

    /// An ambient player with no player program, so nothing is ever spawned.
    fn silent(enabled: bool, volume: u8) -> Ambient {
        let mut ambient = Ambient::new(enabled, volume);
        ambient.player = None;
        ambient
    }

    #[test]
    fn every_ambience_is_a_loop_of_the_right_length() {
        for ambience in Ambience::ALL {
            let loop_ = synthesise(ambience, 100);
            assert_eq!(loop_.len(), samples(LOOP_SECONDS), "{}", ambience.name());
            assert_eq!(
                sound::wav_bytes(&loop_).len() as u64,
                wav_size(),
                "{}",
                ambience.name()
            );
        }
    }

    #[test]
    fn no_sample_reaches_full_scale_and_it_stays_under_the_tones() {
        for ambience in Ambience::ALL {
            let mix = as_float(&synthesise(ambience, 100));
            let peak = mix.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            assert!(peak <= PEAK + 0.001, "{} peaks at {peak}", ambience.name());
            // the tones are levelled to 0.2 over their loudest 50 milliseconds
            let loud = sound::loudness(&mix);
            assert!(
                loud < 0.2,
                "{} is as loud as a tone: {loud}",
                ambience.name()
            );
            assert!(rms(&mix) > 0.01, "{} is silent", ambience.name());
        }
    }

    #[test]
    fn the_loop_joins_without_a_jump() {
        let window = samples(0.05);
        for ambience in Ambience::ALL {
            let mix = as_float(&synthesise(ambience, 100));
            let (start, end) = (&mix[..window], &mix[mix.len() - window..]);
            let (a, b) = (rms(start), rms(end));
            assert!(
                (a - b).abs() < 0.5 * a.max(b) + 0.01,
                "{}: {a} at the start, {b} at the end",
                ambience.name()
            );
            // the step from the last sample to the first is no bigger than the largest step
            // anywhere else in the loop
            let join = (mix[0] - mix[mix.len() - 1]).abs();
            let largest = mix
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                join <= largest,
                "{}: the join steps {join}, the most elsewhere is {largest}",
                ambience.name()
            );
        }
    }

    #[test]
    fn the_volume_scales_the_level() {
        let full = rms(&as_float(&synthesise(Ambience::Water, 100)));
        let half = rms(&as_float(&synthesise(Ambience::Water, 50)));
        assert!((half / full - 0.5).abs() < 0.02, "{half} against {full}");
        assert!(synthesise(Ambience::Water, 0).iter().all(|&s| s == 0));
    }

    #[test]
    fn the_same_loop_is_written_every_time() {
        assert_eq!(
            synthesise(Ambience::Fire, 40),
            synthesise(Ambience::Fire, 40)
        );
    }

    #[test]
    fn every_built_in_scene_has_an_ambience() {
        for info in crate::scenes::SCENES.iter() {
            assert!(
                SCENE_AMBIENCE.iter().any(|(name, _)| *name == info.name),
                "{} has no ambience",
                info.name
            );
        }
        assert_eq!(Ambience::for_scene("campfire"), Ambience::Fire);
        assert_eq!(Ambience::for_scene("my-own-scene"), Ambience::Night);
    }

    #[test]
    fn names_parse_back() {
        for ambience in Ambience::ALL {
            assert_eq!(Ambience::parse(ambience.name()), Some(ambience));
        }
        assert_eq!(Ambience::parse(" Rain "), Some(Ambience::Rain));
        assert_eq!(Ambience::parse("thunder"), None);
    }

    #[test]
    fn off_paused_ducked_silent_and_quiet_hours_stop_the_loop() {
        let now = Instant::now();
        assert!(silent(true, 20).should_play(None, now));
        assert!(!silent(false, 20).should_play(None, now));
        assert!(!silent(true, 0).should_play(None, now));

        let mut paused = silent(true, 20);
        paused.pause(true);
        assert!(!paused.should_play(None, now));
        paused.pause(false);
        assert!(paused.should_play(None, now));

        let mut ducked = silent(true, 20);
        ducked.duck(Duration::from_secs(2));
        assert!(!ducked.should_play(None, Instant::now()));
        assert!(ducked.should_play(None, Instant::now() + Duration::from_secs(3)));

        let mut quiet = silent(true, 20);
        quiet.quiet = Some((QuietHours::parse("23:00-05:00").unwrap(), Clock::start()));
        assert!(!quiet.should_play(Some(23 * 60 + 30), now));
        assert!(!quiet.should_play(Some(4 * 60), now));
        assert!(quiet.should_play(Some(5 * 60), now));
        // an unknown time is not taken as quiet
        assert!(quiet.should_play(None, now));
    }

    #[test]
    fn volume_steps_stay_in_range_and_scenes_switch_the_ambience() {
        let mut ambient = silent(true, 95);
        assert_eq!(ambient.change_volume(true), 100);
        assert_eq!(ambient.change_volume(false), 90);
        let mut low = silent(true, 5);
        assert_eq!(low.change_volume(false), 0);

        ambient.set_scene("storm");
        assert_eq!(ambient.ambience(), Ambience::Rain);
        ambient.set_scene("rain");
        assert_eq!(ambient.ambience(), Ambience::Rain);
        ambient.set_scene("sea");
        assert_eq!(ambient.ambience(), Ambience::Water);
        // with no player, a tick never spawns anything
        ambient.tick();
        assert!(ambient.child.is_none());
    }

    #[test]
    fn the_cached_loop_is_written_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("zacess-ambient-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = ensure_wav(&dir, Ambience::Night, 20).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), wav_size());
        let written = std::fs::metadata(&path).unwrap().modified().unwrap();
        let again = ensure_wav(&dir, Ambience::Night, 20).unwrap();
        assert_eq!(path, again);
        assert_eq!(
            std::fs::metadata(&again).unwrap().modified().unwrap(),
            written
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
