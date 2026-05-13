# CastCodes

CastCodes is an open-source terminal and code workspace fork. This repository is being rebranded externally as **CastCodes** while preserving upstream internal crate and module names such as `warp_core`, `warpui`, and `warp_terminal` for now.

## Current Scope

- Public app name: **CastCodes**
- Public binary/package slug: `cast-codes`
- Public app ID: `dev.castcodes.CastCodes`
- Public URL scheme: `castcodes://`
- Local terminal and code workflows are in scope.
- Upstream hosted services are not provided by this fork.

The public CastCodes build is fork-local by default. Sign-in, upstream cloud flows, hosted telemetry, hosted crash reporting, upstream release feeds, billing, shared sessions, and upstream feedback flows are disabled or unavailable unless CastCodes-owned infrastructure is added later. Feedback links route to this fork's GitHub issue tracker.

## Build

```bash
./script/run
```

For a static packaging check:

```bash
./script/check_rebrand
cargo check -p warp --bin cast-codes --features gui
./script/bundle --channel oss --check-only
```

## Repository Notes

This is a staged external rebrand. Do not run blind repository-wide replacements; many remaining internal names are intentional compatibility boundaries.

Use the rebrand guard before shipping public-surface changes:

```bash
./script/check_rebrand
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security reporting guidance is in [SECURITY.md](SECURITY.md), and community expectations are in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

CastCodes preserves the upstream license structure. See [LICENSE-AGPL](LICENSE-AGPL), [LICENSE-MIT](LICENSE-MIT), and related license files in this repository.
