# Releasing

Releases are automated by two tools that hand off to each other:

- **[release-please]** watches `main`, maintains a release pull request, bumps
  the version in `Cargo.toml`, updates `CHANGELOG.md`, creates the tag and
  creates the GitHub Release.
- **[dist]** (cargo-dist) builds the binaries, archives, checksums and the shell
  installer, uploads them to that Release, and publishes to crates.io.

## The handoff

The two tools have to agree on exactly one thing: who owns the GitHub Release.

`release-please-config.json` sets:

- `"draft": true` — release-please creates the Release, but as a draft, so
  nothing is announced before the binaries exist.
- `"force-tag-creation": true` — the tag is pushed immediately rather than when
  the draft is published. Without this, the draft never publishes, because the
  workflow that publishes it is triggered by the tag.
- `"include-v-in-tag": false` — tags are `1.2.0`, not `v1.2.0`, matching every
  existing tag in this repository. dist derives the installer's download URLs
  from the tag it is invoked with, so this must not drift.

`dist-workspace.toml` sets `create-release = false`, so dist uploads to the
existing draft and undrafts it instead of creating a second, competing Release.

Do **not** add release-please's `skip-github-release` option. That would leave
no Release for dist to upload into.

## Targets

`dist-workspace.toml` pins the same six targets the previous hand-built workflow
published:

| Target | Notes |
| --- | --- |
| `x86_64-unknown-linux-musl` | static, the safe default on older clusters |
| `x86_64-unknown-linux-gnu` | |
| `aarch64-unknown-linux-musl` | static |
| `i686-unknown-linux-musl` | static |
| `x86_64-apple-darwin` | Intel macOS |
| `aarch64-apple-darwin` | Apple silicon |

There are deliberately no Windows targets, no PowerShell installer and no
Homebrew tap: `ssubmit` drives Slurm, which is a Unix/macOS concern.

## Publishing channels

- **GitHub Releases** — archives, `ssubmit-installer.sh`, `sha256.sum`, per-file
  `.sha256` checksums and `dist-manifest.json`. Handled by dist.
- **crates.io** — `.github/workflows/publish-crates.yml`, wired in as a dist
  custom publish job because dist has no builtin crates.io publisher.
- **conda-forge** — unchanged and external. The [feedstock] bot watches for new
  releases and opens its own PR; nothing in this repository drives it.

## Changing the release pipeline

`.github/workflows/release.yml` is generated. Its name and `.yml` extension are
dictated by dist (the generated caller resolves
`uses: ./.github/workflows/publish-crates.yml`), which is why these two files
don't match the `.yaml` extension the rest of the workflows use. Don't "fix" it.

Never edit the generated workflow by hand — change `dist-workspace.toml` and
regenerate:

```shell
dist generate
```

CI runs `just dist-check` (`dist generate --check`), which fails if the
committed workflow has drifted from the configuration.

Pull requests also run the release workflow in `upload` mode
(`pr-run-mode = "upload"`): every target and the installer are built, but the
host, publish and announce jobs are skipped, so nothing reaches a GitHub Release
or crates.io from a PR.

To see what a release would contain without building it:

```shell
just dist-plan
```

## Archive format

`unix-archive = ".tar.gz"` overrides dist's `.tar.xz` default. Two reasons, both
load-bearing:

- It matches what the old workflow published, so `install/install.sh` keeps
  working through the transition.
- `xz` is not always installed on older cluster login nodes. `gzip` always is.

Note that dist names archives `ssubmit-<target>.tar.gz` — **no version
component**, unlike the old `ssubmit-<version>-<target>.tar.gz`. dist cannot put
the version in the archive name, so `install/install.sh` was updated to drop it.
Anyone pinning the old asset URLs needs to know this.

## Installer transition

`install/install.sh`, served from GitHub raw, is the hand-written installer that
predates dist. It is still the documented install method and still works — it
downloads from `releases/latest/download`, which dist keeps populated.

dist generates its own `ssubmit-installer.sh` and attaches it to each Release.
Once the first dist release is published and its installer has been verified
end-to-end on a real cluster, the documented command becomes:

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/mbhall88/ssubmit/releases/latest/download/ssubmit-installer.sh | sh
```

and `install/install.sh` is deleted. Until then both exist, and the README
documents the GitHub-raw one. See issue #18.

[release-please]: https://github.com/googleapis/release-please
[dist]: https://opensource.axo.dev/cargo-dist/
[feedstock]: https://github.com/conda-forge/ssubmit-feedstock
