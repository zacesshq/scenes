# Packaging

How Zacess Scenes reaches crates.io, Homebrew and winget after a release. Each step is run by a
maintainer on their own machine with their own credentials. No token is ever stored in this
repository or in its workflows.

Before any of these, the release must exist: the tag `vX.Y.Z` pushed, the GitHub release made from
it and [`release.yml`](../.github/workflows/release.yml) finished, so the five archives and their
`.sha256` files are attached.

## crates.io

```sh
# check what would be packaged, then publish with the token from the environment
cargo publish --dry-run
CARGO_REGISTRY_TOKEN="$(cat /path/to/token)" cargo publish
```

The token comes from a file or a credential helper, never from the shell history. The crate
excludes `.github`, `packaging`, `scripts` and `palette`, so what is published is the code, the
docs and the licence.

## Homebrew

The formula template is [`homebrew/zacess-scenes.rb`](homebrew/zacess-scenes.rb). It installs the
release archive for the person's platform rather than building from source, so it needs the SHA-256
of each archive.

1. Create the tap repository `zacesshq/homebrew-tap` once, with a `Formula` folder.
2. Copy the template to `Formula/zacess-scenes.rb` in the tap and replace every `VERSION` with the
   version (without the `v`) and each `SHA256_` placeholder with the sum from the matching
   `.sha256` file on the release.
3. Check it locally, then commit it to the tap:

   ```sh
   brew install --build-from-source ./Formula/zacess-scenes.rb
   brew test zacess-scenes
   brew audit --strict --online zacess-scenes
   ```

Once the tap exists, the install line is `brew install zacesshq/tap/zacess-scenes`.

## winget

The manifest templates are in [`winget`](winget): the version, installer and default locale files
winget needs. The installer is the Windows zip from the release, installed as a portable program
with the alias `zacess-scenes`.

1. Copy the three files into a fork of
   [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) under
   `manifests/z/Zacess/Scenes/X.Y.Z/`, replacing every `VERSION` with the version and
   `SHA256_WINDOWS` with the sum from the zip's `.sha256` file.
2. Validate and test them on a Windows machine:

   ```powershell
   winget validate --manifest manifests\z\Zacess\Scenes\X.Y.Z
   winget install --manifest manifests\z\Zacess\Scenes\X.Y.Z
   zacess-scenes --version
   ```

3. Open a pull request to microsoft/winget-pkgs. Their checks take a day or two.

Once accepted, the install line is `winget install Zacess.Scenes`.

> [!NOTE]
> The README says Homebrew and winget are coming until both are live. Update it and remove this
> note in the pull request that follows the first accepted package.
