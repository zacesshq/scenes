# Sound sets

The watch mode plays a short tone for each change in a build. Every set plays the same short
phrase for each event, so the meaning carries over when the set changes: a rising triad for a pass, a falling fourth for a failure, the triad landing an octave up
for a fix and a rising minor third for new warnings. Only the timbre differs.

| Set | Sounds like |
| --- | --- |
| `bells` (the default) | Soft bells with a little shimmer, the original tones |
| `marimba` | Warm wooden bars, short and round |
| `kalimba` | Plucked metal tines with a gentle ring |
| `pad` | A soft synth pad that swells in and fades out |
| `chip` | Gentle 8-bit square waves, rounded so they never buzz |
| `glass` | Slow glassy tones like a wet finger on a rim |
| `piano` | A soft grand piano, felt hammers and a warm low ring |
| `rain` | Water droplets over a quiet bed of rain |

| Event | Plays when | On by default |
| --- | --- | --- |
| `pass` | A run passes after waiting or a key asked for the run | Yes |
| `fail` | A run fails after passing, on the first result or when a key asked for the run | Yes |
| `tick` | A rebuild starts. Quieter than the others | Yes |
| `recovered` | A run passes after failing | Yes |
| `warnings` | A passing run has more warnings than the last | Yes |
| `still_failing` | A run fails again. Very quiet | No |

```sh
zacess-scenes sounds play marimba            # the pass tone
zacess-scenes sounds play kalimba recovered  # one event
zacess-scenes sounds play rain               # droplets over rain
```

The tones are synthesised by the program, with soft envelopes and short fades so they never click
and levelled so no set or event is much louder than another. The piano's overtones sit a little
sharp of whole multiples, as a stiff string's do. The rain set's droplets rise in pitch over a soft
bed of filtered noise that is seeded, so a tone is always written the same way.

The tones are cached as small WAV files in `$XDG_CACHE_HOME/zacess-scenes` when that is set, otherwise `~/Library/Caches/zacess-scenes` on
macOS, `~/.cache/zacess-scenes` on Linux and `%LOCALAPPDATA%\zacess-scenes` on Windows. The
system's own player plays them in the background: `afplay` on macOS, the first of `pw-play`,
`paplay` or `aplay` on Linux and PowerShell's `System.Media.SoundPlayer` on Windows. Without a
player there is no sound and the status bar says `no sound player`.

| Setting | Flag | Environment variable | Config |
| --- | --- | --- | --- |
| Sound off | `--no-sound` | `ZACESS_SOUND=0` (also `false`, `no` or `off`) | `enabled = false` under `[sound]` |
| The sound set | `--sounds <set>` | `ZACESS_SOUNDS=<set>` | `set` |
| Volume from 0 to 100, 50 by default | `--volume <n>` | `ZACESS_VOLUME=<n>` | `volume` |
| Quiet hours | | | `quiet_hours = "22:00-07:00"` |
| One event on or off | | | `[sound.events]` |

Every tone has the same state on screen, so sound is never the only signal.
