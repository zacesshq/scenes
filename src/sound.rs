//! Short tones for the watch mode, synthesised at run time into WAV files in a cache folder and
//! played by the system's own player in the background. Nothing is bundled and no audio library
//! is linked: a missing player only means no sound.

use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Samples a second. Plenty for short soft tones and small enough to write in an instant.
pub const SAMPLE_RATE: u32 = 22_050;
/// The default volume, out of 100.
pub const DEFAULT_VOLUME: u8 = 50;
/// The loudest sample at volume 100, as a share of full scale. The headroom keeps every player's
/// own resampling from clipping.
const PEAK: f32 = 0.7;
/// Fade in and out lengths in seconds. A tone that starts or stops on a non-zero sample clicks.
const FADE_IN: f32 = 0.006;
const FADE_OUT: f32 = 0.03;
/// Bumped whenever the synthesis changes, so a cached file from an older version is not reused.
const SYNTH_VERSION: u32 = 1;

/// The environment variable that turns sound off: `0`, `false`, `no` or `off`.
pub const SOUND_ENV: &str = "ZACESS_SOUND";
/// The environment variable that sets the volume, 0 to 100.
pub const VOLUME_ENV: &str = "ZACESS_VOLUME";

/// The tones the watch mode plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// A gentle rising chime when a run passes.
    Pass,
    /// A lower two-note tone when a run fails.
    Fail,
    /// A soft tick when a rebuild starts.
    Tick,
}

impl Tone {
    pub const ALL: [Tone; 3] = [Tone::Pass, Tone::Fail, Tone::Tick];

    fn name(self) -> &'static str {
        match self {
            Tone::Pass => "pass",
            Tone::Fail => "fail",
            Tone::Tick => "tick",
        }
    }
}

/// One note: its pitch, when it starts, how long it rings, how quickly it dies away and how
/// much of the second and third partials it has, which is what makes it sound like a bell.
struct Note {
    freq: f32,
    start: f32,
    length: f32,
    decay: f32,
    level: f32,
    partials: (f32, f32),
}

/// The notes of a tone and its total length in seconds.
fn notes(tone: Tone) -> (Vec<Note>, f32) {
    let bell = |freq, start| Note {
        freq,
        start,
        length: 0.75,
        decay: 5.0,
        level: 1.0,
        partials: (0.25, 0.08),
    };
    match tone {
        // three steps up a major triad starting on D5
        Tone::Pass => (
            vec![bell(587.33, 0.0), bell(739.99, 0.13), bell(880.0, 0.26)],
            1.05,
        ),
        // a falling fourth low in the range, rounder and shorter than the chime
        Tone::Fail => {
            let low = |freq, start| Note {
                freq,
                start,
                length: 0.42,
                decay: 4.0,
                level: 1.0,
                partials: (0.12, 0.0),
            };
            (vec![low(311.13, 0.0), low(233.08, 0.24)], 0.7)
        }
        Tone::Tick => (
            vec![Note {
                freq: 1320.0,
                start: 0.0,
                length: 0.05,
                decay: 60.0,
                level: 0.35,
                partials: (0.0, 0.0),
            }],
            0.06,
        ),
    }
}

/// The 16 bit samples of a tone at a volume from 0 to 100.
pub fn synthesise(tone: Tone, volume: u8) -> Vec<i16> {
    let (notes, total) = notes(tone);
    let count = (total * SAMPLE_RATE as f32).round() as usize;
    let mut mix = vec![0.0_f32; count];
    for note in &notes {
        let first = (note.start * SAMPLE_RATE as f32) as usize;
        let length = (note.length * SAMPLE_RATE as f32) as usize;
        for i in 0..length {
            let Some(slot) = mix.get_mut(first + i) else {
                break;
            };
            let t = i as f32 / SAMPLE_RATE as f32;
            // each note fades in and out on its own as well, so overlapping notes never click
            let attack = (t / FADE_IN).min(1.0);
            let release = ((note.length - t) / FADE_OUT).clamp(0.0, 1.0);
            let envelope = attack * release * (-note.decay * t).exp();
            let phase = TAU * note.freq * t;
            let wave = phase.sin()
                + note.partials.0 * (2.0 * phase).sin()
                + note.partials.1 * (3.0 * phase).sin();
            *slot += note.level * envelope * wave;
        }
    }
    // the whole tone is scaled so its loudest sample sits exactly at the chosen level, so no
    // mix of notes can ever clip
    let loudest = mix.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let target = PEAK * f32::from(volume.min(100)) / 100.0;
    let tone_level = notes.iter().fold(0.0_f32, |m, n| m.max(n.level));
    let gain = if loudest > 0.0 {
        target * tone_level / loudest
    } else {
        0.0
    };
    let fade_out = (FADE_OUT * SAMPLE_RATE as f32) as usize;
    let fade_in = (FADE_IN * SAMPLE_RATE as f32) as usize;
    mix.iter()
        .enumerate()
        .map(|(i, s)| {
            let edge = (i.min(count - 1 - i) as f32 / fade_in.min(fade_out) as f32).min(1.0);
            let tail = ((count - 1 - i) as f32 / fade_out as f32).min(1.0);
            let value = s * gain * edge.min(tail) * f32::from(i16::MAX);
            value
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
pub fn ensure_wav(dir: &Path, tone: Tone, volume: u8) -> std::io::Result<PathBuf> {
    let path = dir.join(format!("{}-v{volume}-s{SYNTH_VERSION}.wav", tone.name()));
    let bytes = wav_bytes(&synthesise(tone, volume));
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

/// Sound for the watch mode: whether it is on, its volume, the player and the cached tones.
pub struct Sound {
    enabled: bool,
    volume: u8,
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
            player: find_player(),
            dir: cache_dir(),
        }
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

    /// Whether a player was found.
    pub fn available(&self) -> bool {
        self.player.is_some()
    }

    /// Plays a tone in the background and returns at once. Any failure, from a missing player
    /// to a full disk, is silent: the screen always shows the same state.
    pub fn play(&self, tone: Tone) {
        if !self.enabled || self.volume == 0 {
            return;
        }
        let Some(player) = &self.player else {
            return;
        };
        let Ok(file) = ensure_wav(&self.dir, tone, self.volume) else {
            return;
        };
        let child = play_command(player, &file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = child {
            // waited for on its own thread so the finished player never lingers as a zombie
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
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

    #[test]
    fn the_wav_header_describes_mono_16_bit_pcm() {
        let samples = synthesise(Tone::Pass, 80);
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
    fn each_tone_has_its_length() {
        let seconds = |tone| synthesise(tone, 50).len() as f32 / SAMPLE_RATE as f32;
        assert!((seconds(Tone::Pass) - 1.05).abs() < 0.001);
        assert!((seconds(Tone::Fail) - 0.7).abs() < 0.001);
        assert!((seconds(Tone::Tick) - 0.06).abs() < 0.001);
    }

    #[test]
    fn no_sample_reaches_full_scale_at_any_volume() {
        for tone in Tone::ALL {
            for volume in [0, 1, 50, 100, 255] {
                let samples = synthesise(tone, volume);
                let loudest = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
                let ceiling = (PEAK * f32::from(i16::MAX)).ceil() as u16;
                assert!(loudest <= ceiling, "{tone:?} at {volume}: {loudest}");
                assert!(samples.iter().all(|&s| s != i16::MIN));
            }
        }
    }

    #[test]
    fn the_volume_scales_the_level() {
        let peak = |volume| {
            synthesise(Tone::Pass, volume)
                .iter()
                .map(|s| s.unsigned_abs())
                .max()
                .unwrap()
        };
        assert_eq!(peak(0), 0);
        assert!(peak(25) < peak(50) && peak(50) < peak(100));
        let full = PEAK * f32::from(i16::MAX);
        assert!((f32::from(peak(100)) - full).abs() < 2.0);
    }

    #[test]
    fn tones_start_and_end_in_silence_so_they_never_click() {
        for tone in Tone::ALL {
            let samples = synthesise(tone, 100);
            assert_eq!(samples[0], 0, "{tone:?}");
            assert_eq!(*samples.last().unwrap(), 0, "{tone:?}");
            // no jump between two neighbouring samples larger than a smooth tone makes
            let biggest_step = samples
                .windows(2)
                .map(|w| (i32::from(w[1]) - i32::from(w[0])).abs())
                .max()
                .unwrap();
            assert!(biggest_step < 12_000, "{tone:?}: {biggest_step}");
        }
    }

    #[test]
    fn the_pass_chime_rises_and_the_fail_tone_is_lower() {
        let (pass, _) = notes(Tone::Pass);
        assert!(pass.windows(2).all(|w| w[1].freq > w[0].freq));
        let (fail, _) = notes(Tone::Fail);
        assert_eq!(fail.len(), 2);
        assert!(fail.iter().all(|n| n.freq < pass[0].freq));
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
    fn the_cached_file_is_written_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("zacess-scenes-test-{}", std::process::id()));
        let path = ensure_wav(&dir, Tone::Tick, 30).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes, wav_bytes(&synthesise(Tone::Tick, 30)));
        assert_eq!(ensure_wav(&dir, Tone::Tick, 30).unwrap(), path);
        assert_ne!(ensure_wav(&dir, Tone::Tick, 31).unwrap(), path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn muted_sound_plays_nothing_and_never_fails() {
        let sound = Sound {
            enabled: false,
            volume: 50,
            player: None,
            dir: std::env::temp_dir(),
        };
        sound.play(Tone::Pass);
        let silent = Sound {
            enabled: true,
            volume: 50,
            player: None,
            dir: std::env::temp_dir(),
        };
        silent.play(Tone::Fail);
        assert!(!silent.available());
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
