# CastCodes

CastCodes is an open-source terminal and code workspace fork. This repository is being rebranded externally as **CastCodes** while preserving upstream internal crate and module names such as `warp_core`, `warpui`, and `warp_terminal` for now.

## Current Scope

- Public app name: **CastCodes**
- Public binary/package slug: `cast-codes`
- Public app ID: `dev.castcodes.CastCodes`
- Public URL scheme: `castcodes://`
- Local terminal and code workflows are in scope.

The public CastCodes build is local-only by default. It does not include sign-in, hosted telemetry, hosted crash reporting, billing, shared sessions, upstream release feeds, or upstream feedback flows. Feedback links route to this fork's GitHub issue tracker.

## Install on Linux

Prebuilt CastCodes packages for Linux are published as GitHub release assets at
[github.com/OpenCoven/cast-codes/releases/latest](https://github.com/OpenCoven/cast-codes/releases/latest).
The OSS channel currently publishes **x86_64** builds in four formats; each
asset ships with a matching `.sha256` checksum file.

| Distribution | Asset |
| --- | --- |
| Debian, Ubuntu, derivatives | `cast-codes_<version>_amd64.deb` |
| Fedora, RHEL, derivatives | `cast-codes-v<version>-1.x86_64.rpm` |
| Arch Linux, derivatives | `cast-codes-v<version>-1-x86_64.pkg.tar.zst` |
| Any glibc-based distro | `CastCodes-x86_64.AppImage` |

A CLI-only variant of each package (`cast-codes-cli-...`) is also published for
headless installs.

### Verify the download

Download both the package and its `.sha256` file, then compare the checksum:

```bash
expected="$(awk '{print $1}' cast-codes_<version>_amd64.deb.sha256)"
actual="$(sha256sum cast-codes_<version>_amd64.deb | awk '{print $1}')"
test "$actual" = "$expected"
```

### Debian / Ubuntu

```bash
sudo apt install ./cast-codes_<version>_amd64.deb
```

### Fedora / RHEL

```bash
sudo dnf install ./cast-codes-v<version>-1.x86_64.rpm
```

### Arch Linux

```bash
sudo pacman -U cast-codes-v<version>-1-x86_64.pkg.tar.zst
```

Packages install the binary to `/opt/castcodes/cast-codes/` and register a
`cast-codes` launcher and `dev.castcodes.CastCodes.desktop` entry.

### AppImage

The AppImage runs without installation on most modern glibc-based distros:

```bash
chmod +x CastCodes-x86_64.AppImage
./CastCodes-x86_64.AppImage
```

If the AppImage fails to start, install runtime dependencies first — see
[Build from source on Linux](#build-from-source-on-linux) below for the
relevant packages, or run `./script/linux/install_runtime_deps` from a
repository checkout.

### Build from source on Linux

From a checkout, install build and runtime dependencies, then run the app:

```bash
./script/linux/install_runtime_deps
./script/run
```

The dependency installer currently provisions Debian-family systems
(`apt-get`); on other distros, install the equivalent packages by hand — see
`script/linux/install_build_deps` and `script/linux/install_runtime_deps` for
the full lists.

## Build

```bash
./script/run
```

For a static packaging check:

```bash
./script/check_ai_attribution
./script/check_rebrand
cargo check -p warp-app --bin cast-codes --features gui
./script/bundle --channel oss --check-only
```

## Repository Notes

This is a staged external rebrand. Do not run blind repository-wide replacements; many remaining internal names are intentional compatibility boundaries.

Use the rebrand guard before shipping public-surface changes:

```bash
./script/check_ai_attribution
./script/check_rebrand
```

`./script/check_ai_attribution` prevents generated-by/model-credit footers
from entering specs, docs, prompts, and public artifacts. Mentions of supported
AI tools are still allowed when they describe real product behavior.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security reporting guidance is in [SECURITY.md](SECURITY.md), and community expectations are in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

CastCodes preserves the upstream license structure. See [LICENSE-AGPL](LICENSE-AGPL), [LICENSE-MIT](LICENSE-MIT), and related license files in this repository.
