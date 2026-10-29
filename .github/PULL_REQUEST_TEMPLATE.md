## What changed

<!-- What changed and why. -->

## Closes

<!-- Closes #N -->

## How it was checked

- [ ] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass
- [ ] A changed scene was looked at headless at 20 by 8 and at full size, in a dark theme and a light theme
- [ ] A changed scene was run in a real terminal and resized while it ran

## Accessibility impact

- [ ] Nothing flashes and nothing that changes in place changes a cell more than three times a second (tick this too when no motion changed)
- [ ] Reduced motion still slows the scene and removes lightning and flicker
- [ ] Every colour drawn comes from the theme and the contrast test still passes
- [ ] Any new key or flag is in `docs/usage.md` or `docs/watch-mode.md`, the help text and ACCESSIBILITY.md

## Before submitting

- [ ] I have read the [Code of Conduct](https://github.com/zacesshq/scenes/blob/main/CODE_OF_CONDUCT.md) and [CONTRIBUTING.md](https://github.com/zacesshq/scenes/blob/main/CONTRIBUTING.md)
