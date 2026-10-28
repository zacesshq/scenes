# Contributing to Zacess Scenes

Thank you for helping make a calm terminal. This guide covers how a change is made and checked.

## How a change happens

1. Every change starts from an issue that says what "done" means. A scene idea uses the
   [scene idea form](https://github.com/zacesshq/scenes/issues/new?template=scene_idea.yml) and an
   accessibility problem the
   [accessibility problem form](https://github.com/zacesshq/scenes/issues/new?template=accessibility_problem.yml).
2. Work on a branch named after its type and topic, such as `feat/lighthouse-scene` or
   `fix/snow-bank-on-light-themes`.
3. Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`,
   `fix:`, `docs:`, `test:`, `refactor:` and `chore:`.
4. Open a pull request that closes its issue. It is squash-merged once the checks pass and it is
   reviewed.

## Checks

With a stable Rust toolchain of 1.85 or later:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same three on Linux, macOS and Windows, then draws every scene in every theme without
a terminal. To look at a scene yourself:

```sh
# the last of 60 frames, as plain text, repeatable with the same seed
cargo run -- campfire --headless --frames 60 --size 20x8 --seed 1 --theme deep-dark
cargo run -- campfire --headless --frames 60 --size 120x40 --seed 1 --theme deep-light

# in a real terminal, then resize the window while it runs
cargo run -- campfire
```

## Adding a scene

A scene is one file in `src/scenes` that implements the `Scene` trait in `src/scene.rs`:

- `resize` fits the scene to a size. It is called before the first frame and on every resize, so
  everything that depends on the size is worked out there. Treat 20 columns by 8 rows as the
  smallest size and leave out what does not fit rather than squeezing it in.
- `step` advances the simulation by a number of seconds. Multiply every speed by
  `Motion::speed()` and read `motion.calm` for anything that should be left out or slowed further
  under reduced motion.
- `draw` draws into a cleared `Frame` using only colours from the `ThemeData` it is given, so the
  contrast test can prove every colour. Draw with the role methods (`theme.cyan()`,
  `theme.bright_yellow()`, `theme.muted`) rather than a fixed colour.

Then:

1. Add it to `SCENES` and `create` in `src/scenes/mod.rs`, with a one-line summary.
2. Give it a `passes_the_shared_checks` test and a `check_determinism` test, like the others, plus
   a test for the thing that makes the scene itself, such as the bolt's timing or the bank's depth.
3. Describe it in `docs/scenes.md` and add a row to the scene table in the README, including what it
   does under reduced motion.
4. Add it to the scene lists in the two issue forms under `.github/ISSUE_TEMPLATE`.

> [!IMPORTANT]
> Nothing may flash and nothing may change more than three times a second. Anything periodic
> keeps its period in a named constant with a test that holds it, the way `MIN_LIGHTNING_GAP`,
> `FLICKER_PERIOD` and `MIN_PULSE_PERIOD` do. A change that lowers one of those cannot merge.

## Colours

Every colour lives in `palette/zacess.json`, a copy of the palette in
[zacesshq/themes](https://github.com/zacesshq/themes). `src/palette.rs` is written from it by
`scripts/build-palette.py` and never edited by hand; CI fails when the two disagree. To take a new
palette version, replace the JSON file, run the script and commit both.

## Project conventions

- **Accessibility is a requirement, not a feature.** See [ACCESSIBILITY.md](ACCESSIBILITY.md) for
  what every scene has to keep to.
- **Every scene runs headless.** A scene never reads the clock, the terminal or the environment
  itself; it is given time, a size, a random number generator and a theme.
- **No heavy dependencies.** crossterm is the only runtime dependency. Reading a system setting
  uses the command the system already has.
- Documentation uses UK English.

## Releasing

1. Bump `version` in `Cargo.toml` (and `Cargo.lock` by running `cargo build`), move the
   changelog's `Unreleased` entries into a section for the version and merge that through a pull
   request.
2. A maintainer tags the merged commit with a signed annotated tag, pushes it and creates the
   GitHub release from the tag with that version's changelog section as the notes.
3. The tag runs [`release.yml`](.github/workflows/release.yml), which builds the binaries for every
   platform and attaches them to that release. It waits for the release to exist and never creates
   one.
4. The crates.io, Homebrew and winget steps in [packaging/README.md](packaging/README.md) follow,
   each run by a maintainer.
