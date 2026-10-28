# Security Policy

Zacess Scenes is a small program that takes over a terminal, reads a few settings and draws. It
makes no network connection and writes no file. A flaw that breaks any of that is taken seriously.

## Scope

In scope:

- The program reading anything beyond what it documents: the `ZACESS_THEME`,
  `ZACESS_REDUCED_MOTION`, `NO_COLOR`, `XDG_STATE_HOME` and home directory variables, the theme
  switcher's state file and the system's reduced motion and dark mode settings through `defaults`,
  `gsettings` or `reg`
- The program writing a file, opening a network connection or running any other program
- A crafted theme state file or environment variable that makes the program misbehave
- The terminal left in raw mode or on the alternate screen after a quit, a panic or Ctrl+C
- The release workflow publishing something other than what the tagged commit builds or exposing
  a token

## Out of scope

- How a terminal itself handles colours, the alternate screen or raw mode (report those to its
  makers)
- Vulnerabilities in crossterm, GitHub Actions or the Rust toolchain with no demonstrated impact on
  this program (report those upstream)

## Supported versions

Only the latest release and the latest commit on `main` receive security fixes.

## Reporting a vulnerability

> [!IMPORTANT]
> Report privately, never in a public issue or discussion. Use
> [GitHub private vulnerability reporting](https://github.com/zacesshq/scenes/security/advisories/new)
> or email [contact@zacess.com](mailto:contact@zacess.com) with details and reproduction steps.
> Expect an acknowledgement within a few days.
