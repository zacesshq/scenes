//! Short tones for the watch mode, synthesised at run time into WAV files in a cache folder and
//! played by the system's own player in the background. Nothing is bundled and no audio library
//! is linked: a missing player only means no sound.
//!
//! Every set plays the same short phrase for each event, a rising triad for a pass and a falling
//! fourth for a failure, so the meaning carries over when the set changes. Only the timbre
//! differs. Every tone is levelled to the same loudness, measured as the loudest 50 millisecond
//! stretch and kept under a peak ceiling, so no set or event is much louder than another.

use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use crate::rng::Rng;

/// Samples a second. Plenty for short soft tones and small enough to write in an instant.
pub const SAMPLE_RATE: u32 = 22_050;
/// The default volume, out of 100.
pub const DEFAULT_VOLUME: u8 = 50;
/// How far `+` and `-` move the volume.
pub const VOLUME_STEP: u8 = 10;
/// The loudest sample at volume 100, as a share of full scale. The headroom keeps every player's
/// own resampling from clipping.
const PEAK: f32 = 0.7;
/// The loudness every full level tone is set to at volume 100: the root mean square of its
/// loudest 50 milliseconds, as a share of full scale.
const TARGET_RMS: f32 = 0.2;
/// The stretch loudness is measured over, in seconds.
const LOUDNESS_WINDOW: f32 = 0.05;
/// The shortest fade at either end of a whole tone, in seconds. A tone that starts or stops on a
/// non-zero sample clicks.
const EDGE_FADE: f32 = 0.006;
const TAIL_FADE: f32 = 0.03;
/// No partial above this is synthesised. Higher partials make even a soft square wave harsh and
/// near half the sample rate they would alias.
const HIGHEST_PARTIAL: f32 = 6000.0;
/// Bumped whenever the synthesis changes, so a cached file from an older version is not reused.
const SYNTH_VERSION: u32 = 3;

/// The environment variable that turns sound off: `0`, `false`, `no` or `off`.
pub const SOUND_ENV: &str = "ZACESS_SOUND";
/// The environment variable that sets the volume, 0 to 100.
pub const VOLUME_ENV: &str = "ZACESS_VOLUME";
/// The environment variable that picks the sound set.
pub const SET_ENV: &str = "ZACESS_SOUNDS";

/// The events the watch mode can sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    /// A run passed.
    Pass,
    /// A run failed after passing or on the first result.
    Fail,
    /// A rebuild started.
    Tick,
    /// A run passed after failing.
    Recovered,
    /// A run failed again. Very quiet and off unless turned on.
    StillFailing,
    /// A passing run has warnings it did not have before.
    Warnings,
    /// A command in a followed shell was not found, which is almost always a typo.
    Typo,
}

impl Tone {
    pub const ALL: [Tone; 7] = [
        Tone::Pass,
        Tone::Fail,
        Tone::Tick,
        Tone::Recovered,
        Tone::StillFailing,
        Tone::Warnings,
        Tone::Typo,
    ];

    /// The name used in the config file and by `sounds play`.
    pub fn name(self) -> &'static str {
        match self {
            Tone::Pass => "pass",
            Tone::Fail => "fail",
            Tone::Tick => "tick",
            Tone::Recovered => "recovered",
            Tone::StillFailing => "still_failing",
            Tone::Warnings => "warnings",
            Tone::Typo => "typo",
        }
    }

    pub fn parse(text: &str) -> Option<Tone> {
        let key = text.trim().to_ascii_lowercase().replace('-', "_");
        Tone::ALL.into_iter().find(|t| t.name() == key)
    }

    /// How loud the event is beside a pass: the tick and the still failing reminder are kept in
    /// the background.
    fn level(self) -> f32 {
        match self {
            Tone::Tick => 0.4,
            Tone::StillFailing => 0.3,
            // a mistyped command is common and small, so it is a nudge rather than an alarm
            Tone::Typo => 0.6,
            _ => 1.0,
        }
    }

    /// Whether the event sounds unless the config says otherwise.
    pub fn on_by_default(self) -> bool {
        self != Tone::StillFailing
    }
}

/// A family of tones with one timbre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundSet {
    Bells,
    Marimba,
    Kalimba,
    Pad,
    Chip,
    Glass,
    Piano,
    Rain,
}

impl SoundSet {
    pub const ALL: [SoundSet; 8] = [
        SoundSet::Bells,
        SoundSet::Marimba,
        SoundSet::Kalimba,
        SoundSet::Pad,
        SoundSet::Chip,
        SoundSet::Glass,
        SoundSet::Piano,
        SoundSet::Rain,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SoundSet::Bells => "bells",
            SoundSet::Marimba => "marimba",
            SoundSet::Kalimba => "kalimba",
            SoundSet::Pad => "pad",
            SoundSet::Chip => "chip",
            SoundSet::Glass => "glass",
            SoundSet::Piano => "piano",
            SoundSet::Rain => "rain",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            SoundSet::Bells => "Soft bells with a little shimmer, the original tones",
            SoundSet::Marimba => "Warm wooden bars, short and round",
            SoundSet::Kalimba => "Plucked metal tines with a gentle ring",
            SoundSet::Pad => "A soft synth pad that swells in and fades out",
            SoundSet::Chip => "Gentle 8-bit square waves, rounded so they never buzz",
            SoundSet::Glass => "Slow glassy tones like a wet finger on a rim",
            SoundSet::Piano => "A soft grand piano, felt hammers and a warm low ring",
            SoundSet::Rain => "Water droplets over a quiet bed of rain",
        }
    }

    pub fn parse(text: &str) -> Option<SoundSet> {
        let key = text.trim().to_ascii_lowercase();
        SoundSet::ALL.into_iter().find(|s| s.name() == key)
    }

    /// The next set, wrapping round, for the `m` key.
    pub fn next(self) -> SoundSet {
        let i = SoundSet::ALL.iter().position(|&s| s == self).unwrap_or(0);
        SoundSet::ALL[(i + 1) % SoundSet::ALL.len()]
    }

    fn timbre(self) -> Timbre {
        match self {
            SoundSet::Bells => Timbre {
                partials: &[(1.0, 1.0, 1.0), (2.0, 0.25, 1.0), (3.0, 0.08, 1.0)],
                attack: 0.006,
                decay: 5.0,
                length: 0.75,
                octave: 1.0,
                glide: 0.0,
                rain: 0.0,
            },
            // the fourth partial of a marimba bar dies away much faster than the fundamental,
            // which gives the wooden knock at the start
            SoundSet::Marimba => Timbre {
                partials: &[(1.0, 1.0, 1.0), (3.93, 0.18, 3.5), (9.8, 0.04, 6.0)],
                attack: 0.003,
                decay: 7.0,
                length: 0.5,
                octave: 0.5,
                glide: 0.0,
                rain: 0.0,
            },
            // a tine's overtone is not a whole multiple, which is what makes it sound plucked
            // metal rather than a string
            SoundSet::Kalimba => Timbre {
                partials: &[(1.0, 1.0, 1.0), (5.4, 0.22, 4.0), (2.0, 0.08, 1.5)],
                attack: 0.004,
                decay: 4.0,
                length: 0.7,
                octave: 1.0,
                glide: 0.0,
                rain: 0.0,
            },
            SoundSet::Pad => Timbre {
                partials: &[
                    (1.0, 1.0, 1.0),
                    (1.004, 0.7, 1.0),
                    (0.996, 0.7, 1.0),
                    (2.0, 0.25, 1.0),
                    (3.0, 0.1, 1.0),
                ],
                attack: 0.12,
                decay: 1.4,
                length: 0.95,
                octave: 0.5,
                glide: 0.0,
                rain: 0.0,
            },
            // odd harmonics at one over their number make a square wave; stopping at the ninth
            // keeps it soft and every partial well under half the sample rate
            SoundSet::Chip => Timbre {
                partials: &[
                    (1.0, 1.0, 1.0),
                    (3.0, 1.0 / 3.0, 1.0),
                    (5.0, 1.0 / 5.0, 1.0),
                    (7.0, 1.0 / 7.0, 1.0),
                    (9.0, 1.0 / 9.0, 1.0),
                ],
                attack: 0.006,
                decay: 3.0,
                length: 0.3,
                octave: 1.0,
                glide: 0.0,
                rain: 0.0,
            },
            SoundSet::Glass => Timbre {
                partials: &[(1.0, 1.0, 1.0), (2.0, 0.06, 1.0), (4.0, 0.02, 1.0)],
                attack: 0.05,
                decay: 2.2,
                length: 0.9,
                octave: 1.0,
                glide: 0.0,
                rain: 0.0,
            },
            // a piano string is stiff, so each overtone sits a little above its whole multiple
            // (n times the square root of 1 + 0.0004 n squared); the upper ones die away first,
            // which gives the bright strike and the warm ring after it
            SoundSet::Piano => Timbre {
                partials: &[
                    (1.0, 1.0, 1.0),
                    (1.0015, 0.45, 1.0),
                    (2.0016, 0.5, 1.6),
                    (3.0054, 0.22, 2.3),
                    (4.0128, 0.12, 3.0),
                    (5.025, 0.06, 3.8),
                ],
                attack: 0.003,
                decay: 3.2,
                length: 0.9,
                octave: 0.5,
                glide: 0.0,
                rain: 0.0,
            },
            // a falling drop rings as a bubble whose pitch rises as it closes, so each note
            // glides up a little and dies fast over a soft bed of rain
            SoundSet::Rain => Timbre {
                partials: &[(1.0, 1.0, 1.0), (2.01, 0.1, 2.0)],
                attack: 0.004,
                decay: 9.0,
                length: 0.6,
                octave: 1.0,
                glide: 0.6,
                rain: 0.35,
            },
        }
    }
}

/// How a set sounds: its partials as a multiple of the pitch, a level and how much faster than
/// the note they die away, plus the note's attack, decay rate, length and an octave shift. `glide`
/// raises each note's pitch by that share of itself every second and `rain` is the level of a soft
/// noise bed under the whole tone.
struct Timbre {
    partials: &'static [(f32, f32, f32)],
    attack: f32,
    decay: f32,
    length: f32,
    octave: f32,
    glide: f32,
    rain: f32,
}

/// One note of a phrase: its pitch, when it starts and its own level.
#[derive(Clone, Copy, Debug)]
struct Note {
    freq: f32,
    start: f32,
    level: f32,
}

/// The phrase an event plays (the same in every set) and how long a note lasts as a share of
/// the set's note length.
fn phrase(tone: Tone) -> (Vec<Note>, f32) {
    let note = |freq, start| Note {
        freq,
        start,
        level: 1.0,
    };
    match tone {
        // three steps up a major triad on D5
        Tone::Pass => (
            vec![note(587.33, 0.0), note(739.99, 0.13), note(880.0, 0.26)],
            1.0,
        ),
        // a falling fourth low in the range, shorter than the chime
        Tone::Fail => (vec![note(311.13, 0.0), note(233.08, 0.24)], 0.6),
        Tone::Tick => (vec![note(1320.0, 0.0)], 0.08),
        // the triad again, quicker, landing an octave above where it started
        Tone::Recovered => (
            vec![
                note(587.33, 0.0),
                note(739.99, 0.1),
                note(880.0, 0.2),
                Note {
                    freq: 1174.66,
                    start: 0.3,
                    level: 1.2,
                },
            ],
            1.0,
        ),
        Tone::StillFailing => (vec![note(233.08, 0.0)], 0.45),
        // a rising minor third, a question rather than an alarm
        Tone::Warnings => (vec![note(440.0, 0.0), note(523.25, 0.16)], 0.55),
        // two quick low notes a step apart, a soft shrug that is clearly not the fail phrase
        Tone::Typo => (vec![note(392.0, 0.0), note(349.23, 0.09)], 0.35),
    }
}

fn seconds(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}

fn samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32).round() as usize
}

/// The mix of a tone before it is levelled, as floats.
fn raw(set: SoundSet, tone: Tone) -> Vec<f32> {
    let timbre = set.timbre();
    let (notes, share) = phrase(tone);
    let length = (timbre.length * share).max(0.05);
    let last_start = notes.iter().fold(0.0_f32, |m, n| m.max(n.start));
    let mut mix = vec![0.0_f32; samples(last_start + length) + 1];
    for note in &notes {
        let freq = note.freq * timbre.octave;
        let first = samples(note.start);
        let count = samples(length);
        for i in 0..count {
            let Some(slot) = mix.get_mut(first + i) else {
                break;
            };
            let t = seconds(i);
            // each note fades in and out on its own as well, so overlapping notes never click
            let attack = (t / timbre.attack).min(1.0);
            let release = ((length - t) / TAIL_FADE).clamp(0.0, 1.0);
            let mut wave = 0.0;
            for &(multiple, level, fade) in timbre.partials {
                let f = freq * multiple;
                if f > HIGHEST_PARTIAL {
                    continue;
                }
                // the phase of a pitch rising at a steady rate is the integral of the frequency
                let phase = TAU * f * (t + timbre.glide * t * t / 2.0);
                wave += level * (-timbre.decay * fade * t).exp() * phase.sin();
            }
            *slot += note.level * attack * release * wave;
        }
    }
    if timbre.rain > 0.0 {
        add_rain(&mut mix, timbre.rain, tone);
    }
    mix
}

/// Adds a soft bed of rain: noise through two gentle low-pass filters, so it hisses quietly
/// rather than crackles, faded in and out over the whole tone. The noise is seeded by the event,
/// so the same tone is always written the same way and the cache stays valid.
fn add_rain(mix: &mut [f32], level: f32, tone: Tone) {
    let mut rng = Rng::new(0x7a1d_0000 + tone as u64);
    let count = mix.len();
    let fade_in = samples(0.04).max(1) as f32;
    let fade_out = samples(0.15).max(1) as f32;
    let (mut low, mut lower) = (0.0_f32, 0.0_f32);
    for (i, slot) in mix.iter_mut().enumerate() {
        let white = rng.unit() * 2.0 - 1.0;
        low += 0.12 * (white - low);
        lower += 0.12 * (low - lower);
        let from_end = (count - 1 - i) as f32;
        let fade = (i as f32 / fade_in).min(from_end / fade_out).min(1.0);
        // the filters take the noise down to about a tenth of its level, so it is scaled back up
        *slot += level * 8.0 * lower * fade;
    }
}

/// The root mean square of the loudest stretch of samples.
pub fn loudness(samples: &[f32]) -> f32 {
    let window = self::samples(LOUDNESS_WINDOW).max(1);
    if samples.len() <= window {
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        return (sum / window as f32).sqrt();
    }
    let mut sum: f32 = samples[..window].iter().map(|s| s * s).sum();
    let mut most = sum;
    for i in window..samples.len() {
        sum += samples[i] * samples[i] - samples[i - window] * samples[i - window];
        most = most.max(sum);
    }
    (most.max(0.0) / window as f32).sqrt()
}

/// The 16 bit samples of a tone at a volume from 0 to 100.
pub fn synthesise(set: SoundSet, tone: Tone, volume: u8) -> Vec<i16> {
    let mix = raw(set, tone);
    let count = mix.len();
    let volume = f32::from(volume.min(100)) / 100.0;
    let loud = loudness(&mix);
    let peak = mix.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    // the tone is levelled to the target loudness, unless that would push its peak past the
    // ceiling, so no mix of notes can ever clip
    let gain = if loud > 0.0 && peak > 0.0 {
        (TARGET_RMS * tone.level() / loud).min(PEAK / peak) * volume
    } else {
        0.0
    };
    let edge = samples(EDGE_FADE).max(1);
    let tail = samples(TAIL_FADE).max(1);
    mix.iter()
        .enumerate()
        .map(|(i, s)| {
            let from_end = count - 1 - i;
            let fade = (i.min(from_end) as f32 / edge as f32)
                .min(from_end as f32 / tail as f32)
                .min(1.0);
            (s * gain * fade * f32::from(i16::MAX))
                .round()
                .clamp(-f32::from(i16::MAX), f32::from(i16::MAX)) as i16
        })
        .collect()
}

/// A mono 16 bit PCM WAV file holding `samples`.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1_u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes a second
    out.extend_from_slice(&2_u16.to_le_bytes()); // bytes a frame
    out.extend_from_slice(&16_u16.to_le_bytes()); // bits a sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// Whether the sound variable's value turns sound off.
pub fn env_turns_off(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("0" | "false" | "no" | "off")
    )
}

/// Parses a volume from 0 to 100.
pub fn parse_volume(text: &str) -> Option<u8> {
    text.trim().parse::<u8>().ok().filter(|v| *v <= 100)
}

/// The folder the tones are cached in: `$XDG_CACHE_HOME/zacess-scenes`, `~/Library/Caches` on
/// macOS, `%LOCALAPPDATA%` on Windows, otherwise `~/.cache`. The temporary folder is used when
/// there is no home.
pub fn cache_dir() -> PathBuf {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if let Some(dir) = var("XDG_CACHE_HOME") {
        PathBuf::from(dir)
    } else if cfg!(windows) {
        var("LOCALAPPDATA").map_or_else(std::env::temp_dir, PathBuf::from)
    } else if let Some(home) = var("HOME") {
        let home = PathBuf::from(home);
        if cfg!(target_os = "macos") {
            home.join("Library").join("Caches")
        } else {
            home.join(".cache")
        }
    } else {
        std::env::temp_dir()
    };
    base.join("zacess-scenes")
}

/// Writes a tone's WAV file into `dir` unless an identical one is already there. Returns its
/// path.
pub fn ensure_wav(dir: &Path, set: SoundSet, tone: Tone, volume: u8) -> std::io::Result<PathBuf> {
    let path = dir.join(format!(
        "{}-{}-v{volume}-s{SYNTH_VERSION}.wav",
        set.name(),
        tone.name()
    ));
    let bytes = wav_bytes(&synthesise(set, tone, volume));
    let current = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64);
    if !current {
        std::fs::create_dir_all(dir)?;
        // written beside the target and renamed, so a player never reads half a file
        let partial = path.with_extension(format!("wav.{}", std::process::id()));
        std::fs::write(&partial, &bytes)?;
        std::fs::rename(&partial, &path)?;
    }
    Ok(path)
}

/// Quiet hours: a span of the day, in minutes from midnight, with no sound. The span may cross
/// midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuietHours {
    pub start: u16,
    pub end: u16,
}

impl QuietHours {
    /// Parses `22:00-07:00`. Equal ends make an empty span.
    pub fn parse(text: &str) -> Option<QuietHours> {
        let time = |part: &str| {
            let (h, m) = part.trim().split_once(':')?;
            let h: u16 = h.trim().parse().ok()?;
            let m: u16 = m.trim().parse().ok()?;
            (h < 24 && m < 60).then_some(h * 60 + m)
        };
        let (start, end) = text.split_once('-')?;
        Some(QuietHours {
            start: time(start)?,
            end: time(end)?,
        })
    }

    pub fn contains(self, minute: u16) -> bool {
        if self.start <= self.end {
            (self.start..self.end).contains(&minute)
        } else {
            minute >= self.start || minute < self.end
        }
    }

    pub fn text(self) -> String {
        format!(
            "{:02}:{:02}-{:02}:{:02}",
            self.start / 60,
            self.start % 60,
            self.end / 60,
            self.end % 60
        )
    }
}

/// The local time as minutes from midnight, asked of the system. The standard library has no
/// time zones, so the system's own clock command is used, as for the other system settings.
pub fn local_minutes() -> Option<u16> {
    let text = if cfg!(windows) {
        crate::motion::command_output(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-Date -Format HH:mm",
            ],
        )?
    } else {
        crate::motion::command_output("date", &["+%H:%M"])?
    };
    let (h, m) = text.trim().split_once(':')?;
    Some(h.parse::<u16>().ok()? * 60 + m.parse::<u16>().ok()?)
}

/// The local time, kept up to date by a background thread that asks once every half minute, so
/// a state change never waits on a command.
#[derive(Clone, Debug)]
pub struct Clock {
    minutes: Arc<AtomicI32>,
}

impl Clock {
    pub fn start() -> Self {
        let minutes = Arc::new(AtomicI32::new(-1));
        let shared = Arc::clone(&minutes);
        std::thread::spawn(move || {
            loop {
                let now = local_minutes().map_or(-1, i32::from);
                shared.store(now, Ordering::Relaxed);
                std::thread::sleep(Duration::from_secs(30));
            }
        });
        Self { minutes }
    }

    #[cfg(test)]
    fn fixed(minute: u16) -> Self {
        Self {
            minutes: Arc::new(AtomicI32::new(i32::from(minute))),
        }
    }

    /// Minutes from midnight, `None` until known.
    pub fn now(&self) -> Option<u16> {
        u16::try_from(self.minutes.load(Ordering::Relaxed)).ok()
    }
}

/// A program that can play a WAV file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Player {
    /// A command that takes the file as its last argument: afplay, pw-play, paplay or aplay.
    Command(PathBuf),
    /// PowerShell's `System.Media.SoundPlayer`, on Windows.
    PowerShell(PathBuf),
}

/// Finds a program on `PATH`, adding `.exe` on Windows.
pub fn find_program(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

/// The players to look for on this system, in order of preference.
pub fn player_names() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["afplay"]
    } else if cfg!(windows) {
        &["powershell", "pwsh"]
    } else {
        &["pw-play", "paplay", "aplay"]
    }
}

/// The first player this system has, if any.
pub fn find_player() -> Option<Player> {
    player_names().iter().find_map(|name| {
        let path = find_program(name)?;
        Some(if cfg!(windows) {
            Player::PowerShell(path)
        } else {
            Player::Command(path)
        })
    })
}

/// The command that plays one file.
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

/// Sound for the watch mode: whether it is on, its volume and set, which events sound, quiet
/// hours, the player and the cached tones.
pub struct Sound {
    enabled: bool,
    volume: u8,
    set: SoundSet,
    events: Vec<Tone>,
    quiet: Option<(QuietHours, Clock)>,
    player: Option<Player>,
    dir: PathBuf,
}

impl Sound {
    /// Sets sound up, finding the player once. Sound that is off at the start can still be
    /// turned on later, so the player is looked for either way.
    pub fn new(enabled: bool, volume: u8) -> Self {
        Self {
            enabled,
            volume: volume.min(100),
            set: SoundSet::Bells,
            events: Tone::ALL
                .into_iter()
                .filter(|t| t.on_by_default())
                .collect(),
            quiet: None,
            player: find_player(),
            dir: cache_dir(),
        }
    }

    pub fn with_set(mut self, set: SoundSet) -> Self {
        self.set = set;
        self
    }

    /// Which events sound.
    pub fn with_events(mut self, events: Vec<Tone>) -> Self {
        self.events = events;
        self
    }

    /// No sound inside these hours. Starts the clock only when there are quiet hours.
    pub fn with_quiet_hours(mut self, quiet: Option<QuietHours>) -> Self {
        self.quiet = quiet.map(|q| (q, Clock::start()));
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set(&self) -> SoundSet {
        self.set
    }

    /// Moves to the next set and returns it.
    pub fn next_set(&mut self) -> SoundSet {
        self.set = self.set.next();
        self.set
    }

    /// Raises or lowers the volume by one step, within 0 to 100.
    pub fn change_volume(&mut self, up: bool) -> u8 {
        self.volume = if up {
            self.volume.saturating_add(VOLUME_STEP).min(100)
        } else {
            self.volume.saturating_sub(VOLUME_STEP)
        };
        self.volume
    }

    /// Whether a player was found.
    pub fn available(&self) -> bool {
        self.player.is_some()
    }

    /// Whether it is quiet hours now.
    pub fn quiet_now(&self) -> bool {
        self.quiet
            .as_ref()
            .is_some_and(|(hours, clock)| clock.now().is_some_and(|m| hours.contains(m)))
    }

    /// Whether a tone would sound now.
    pub fn would_play(&self, tone: Tone) -> bool {
        self.enabled && self.volume > 0 && self.events.contains(&tone) && !self.quiet_now()
    }

    /// Plays a tone in the background and returns at once. Any failure, from a missing player
    /// to a full disk, is silent: the screen always shows the same state.
    pub fn play(&self, tone: Tone) {
        if self.would_play(tone) {
            self.play_now(self.set, tone);
        }
    }

    /// Plays a tone of a set whatever the settings say, for a preview.
    pub fn play_now(&self, set: SoundSet, tone: Tone) -> Option<std::process::Child> {
        let player = self.player.as_ref()?;
        let file = ensure_wav(&self.dir, set, tone, self.volume).ok()?;
        let child = play_command(player, &file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn u16_at(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
    }

    fn as_float(samples: &[i16]) -> Vec<f32> {
        samples
            .iter()
            .map(|&s| f32::from(s) / f32::from(i16::MAX))
            .collect()
    }

    fn decibels(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    #[test]
    fn the_wav_header_describes_mono_16_bit_pcm() {
        let samples = synthesise(SoundSet::Bells, Tone::Pass, 80);
        let bytes = wav_bytes(&samples);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32_at(&bytes, 4) as usize, bytes.len() - 8);
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(u32_at(&bytes, 16), 16);
        assert_eq!(u16_at(&bytes, 20), 1);
        assert_eq!(u16_at(&bytes, 22), 1);
        assert_eq!(u32_at(&bytes, 24), SAMPLE_RATE);
        assert_eq!(u32_at(&bytes, 28), SAMPLE_RATE * 2);
        assert_eq!(u16_at(&bytes, 32), 2);
        assert_eq!(u16_at(&bytes, 34), 16);
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32_at(&bytes, 40) as usize, samples.len() * 2);
        assert_eq!(bytes.len(), 44 + samples.len() * 2);
    }

    #[test]
    fn there_are_eight_sets_each_with_every_event() {
        assert_eq!(SoundSet::ALL.len(), 8);
        for set in SoundSet::ALL {
            assert_eq!(SoundSet::parse(set.name()), Some(set));
            for tone in Tone::ALL {
                let samples = synthesise(set, tone, 50);
                let length = seconds(samples.len());
                assert!((0.05..1.6).contains(&length), "{set:?} {tone:?}: {length}");
                assert!(
                    samples.iter().any(|&s| s != 0),
                    "{set:?} {tone:?} is silent"
                );
            }
        }
        assert_eq!(SoundSet::Glass.next(), SoundSet::Piano);
        assert_eq!(SoundSet::Rain.next(), SoundSet::Bells);
        assert_eq!(Tone::parse("still-failing"), Some(Tone::StillFailing));
        assert!(!Tone::StillFailing.on_by_default());
    }

    #[test]
    fn no_sample_reaches_full_scale_at_any_volume() {
        let ceiling = (PEAK * f32::from(i16::MAX)).ceil() as u16;
        for set in SoundSet::ALL {
            for tone in Tone::ALL {
                for volume in [0, 1, 50, 100, 255] {
                    let samples = synthesise(set, tone, volume);
                    let loudest = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
                    assert!(
                        loudest <= ceiling,
                        "{set:?} {tone:?} at {volume}: {loudest}"
                    );
                    assert!(samples.iter().all(|&s| s != i16::MIN));
                }
            }
        }
    }

    #[test]
    fn every_set_and_event_is_levelled_to_the_same_loudness() {
        // each tone reaches its target loudness to within a decibel, so no set or event stands
        // out: only the tick and the still failing reminder sit lower, on purpose
        for set in SoundSet::ALL {
            for tone in Tone::ALL {
                let loud = loudness(&as_float(&synthesise(set, tone, 100)));
                let wanted = TARGET_RMS * tone.level();
                let off = decibels(loud / wanted).abs();
                assert!(off < 1.0, "{set:?} {tone:?} is {off:.2} dB from its level");
            }
        }
        let pass: Vec<f32> = SoundSet::ALL
            .iter()
            .map(|&s| loudness(&as_float(&synthesise(s, Tone::Pass, 100))))
            .collect();
        let most = pass.iter().cloned().fold(0.0, f32::max);
        let least = pass.iter().cloned().fold(1.0, f32::min);
        assert!(decibels(most / least) < 1.5, "{pass:?}");
    }

    #[test]
    fn the_volume_scales_the_level() {
        let peak = |volume| {
            synthesise(SoundSet::Bells, Tone::Pass, volume)
                .iter()
                .map(|s| s.unsigned_abs())
                .max()
                .unwrap()
        };
        assert_eq!(peak(0), 0);
        assert!(peak(25) < peak(50) && peak(50) < peak(100));
    }

    #[test]
    fn tones_start_and_end_in_silence_so_they_never_click() {
        for set in SoundSet::ALL {
            for tone in Tone::ALL {
                let samples = synthesise(set, tone, 100);
                assert_eq!(samples[0], 0, "{set:?} {tone:?}");
                assert_eq!(*samples.last().unwrap(), 0, "{set:?} {tone:?}");
                // no jump between two neighbouring samples larger than a smooth tone makes
                let biggest_step = samples
                    .windows(2)
                    .map(|w| (i32::from(w[1]) - i32::from(w[0])).abs())
                    .max()
                    .unwrap();
                assert!(biggest_step < 12_000, "{set:?} {tone:?}: {biggest_step}");
            }
        }
    }

    #[test]
    fn the_pass_phrase_rises_and_the_fail_phrase_is_lower() {
        let (pass, _) = phrase(Tone::Pass);
        assert!(pass.windows(2).all(|w| w[1].freq > w[0].freq));
        let (fail, _) = phrase(Tone::Fail);
        assert_eq!(fail.len(), 2);
        assert!(fail.iter().all(|n| n.freq < pass[0].freq));
        let (recovered, _) = phrase(Tone::Recovered);
        assert!(recovered.last().unwrap().freq > pass.last().unwrap().freq);
    }

    #[test]
    fn the_variables_and_volume_parse() {
        for value in ["0", "off", "OFF", "false", "no", " no "] {
            assert!(env_turns_off(Some(value)), "{value}");
        }
        for value in ["1", "on", "", "yes"] {
            assert!(!env_turns_off(Some(value)), "{value}");
        }
        assert!(!env_turns_off(None));
        assert_eq!(parse_volume("0"), Some(0));
        assert_eq!(parse_volume(" 40 "), Some(40));
        assert_eq!(parse_volume("100"), Some(100));
        assert_eq!(parse_volume("101"), None);
        assert_eq!(parse_volume("-1"), None);
        assert_eq!(parse_volume("loud"), None);
    }

    #[test]
    fn quiet_hours_parse_and_cross_midnight() {
        let night = QuietHours::parse("22:00-07:30").unwrap();
        assert!(night.contains(23 * 60) && night.contains(0) && night.contains(7 * 60 + 29));
        assert!(!night.contains(7 * 60 + 30) && !night.contains(12 * 60));
        assert_eq!(night.text(), "22:00-07:30");
        let lunch = QuietHours::parse(" 12:00 - 13:00 ").unwrap();
        assert!(lunch.contains(12 * 60 + 30) && !lunch.contains(13 * 60));
        for bad in ["", "22:00", "25:00-07:00", "22:60-07:00", "ten-eleven"] {
            assert_eq!(QuietHours::parse(bad), None, "{bad}");
        }
    }

    fn silent_sound() -> Sound {
        Sound {
            enabled: true,
            volume: 50,
            set: SoundSet::Bells,
            events: Tone::ALL
                .into_iter()
                .filter(|t| t.on_by_default())
                .collect(),
            quiet: None,
            player: None,
            dir: std::env::temp_dir(),
        }
    }

    #[test]
    fn events_quiet_hours_and_volume_decide_what_plays() {
        let mut sound = silent_sound();
        assert!(sound.would_play(Tone::Pass));
        assert!(!sound.would_play(Tone::StillFailing));
        sound.quiet = Some((
            QuietHours::parse("22:00-07:00").unwrap(),
            Clock::fixed(23 * 60),
        ));
        assert!(sound.quiet_now() && !sound.would_play(Tone::Pass));
        sound.quiet = Some((
            QuietHours::parse("22:00-07:00").unwrap(),
            Clock::fixed(12 * 60),
        ));
        assert!(sound.would_play(Tone::Pass));
        assert_eq!(sound.change_volume(true), 60);
        for _ in 0..10 {
            sound.change_volume(true);
        }
        assert_eq!(sound.volume(), 100);
        for _ in 0..12 {
            sound.change_volume(false);
        }
        assert_eq!(sound.volume(), 0);
        assert!(!sound.would_play(Tone::Pass));
        assert_eq!(sound.next_set(), SoundSet::Marimba);
        sound.toggle();
        assert!(!sound.enabled());
        sound.play(Tone::Fail);
        assert!(!sound.available());
    }

    #[test]
    fn the_cached_file_is_written_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("zacess-scenes-test-{}", std::process::id()));
        let path = ensure_wav(&dir, SoundSet::Chip, Tone::Tick, 30).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            bytes,
            wav_bytes(&synthesise(SoundSet::Chip, Tone::Tick, 30))
        );
        assert_eq!(
            ensure_wav(&dir, SoundSet::Chip, Tone::Tick, 30).unwrap(),
            path
        );
        assert_ne!(
            ensure_wav(&dir, SoundSet::Chip, Tone::Tick, 31).unwrap(),
            path
        );
        assert_ne!(
            ensure_wav(&dir, SoundSet::Pad, Tone::Tick, 30).unwrap(),
            path
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_powershell_path_is_quoted() {
        let command = play_command(
            &Player::PowerShell(PathBuf::from("powershell")),
            Path::new("C:\\Users\\o'neil\\pass.wav"),
        );
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.last()
                .unwrap()
                .contains("'C:\\Users\\o''neil\\pass.wav'")
        );
    }
}
