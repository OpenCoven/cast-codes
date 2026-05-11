---
name: castcodes-dev-loop
description: Choose and run CastCodes local development, build, rebrand, and packaging checks. Use when building or verifying the app, selecting focused Rust checks, running the public OSS binary, validating release or bundle behavior, or interpreting CastCodes-specific local development failures.
---

# castcodes-dev-loop

## Workflow

Choose the smallest command set that proves the change. Read `references/commands.md` when the right command is not obvious.

Default local run:

```bash
./script/run
```

Public-surface and identity changes:

```bash
./script/check_rebrand
cargo test -p warp_core --features local_fs castcodes
```

Core app compile check:

```bash
cargo check -p warp --bin cast-codes --features gui
```

Static bundle check:

```bash
./script/bundle --channel oss --check-only
```

## Command Selection

- Rebrand/doc/bundle metadata: run `./script/check_rebrand` first.
- Core identity/path/channel behavior: run `cargo test -p warp_core --features local_fs castcodes`.
- App code or UI behavior: run `cargo check -p warp --bin cast-codes --features gui`, then focused tests.
- Packaging scripts: run the relevant `script/*/bundle*` check path for the target platform when available.
- Generic Rust failures: use existing `fix-errors` and `rust-unit-tests` skills.

Do not default to full presubmit when a focused command proves the touched behavior. Do report when full presubmit was not run.

## Verification Notes

`./script/run` selects the public `cast-codes` binary for OSS worktrees without internal `warp-channel-config`. It may delegate to platform-specific scripts and can launch the app on macOS.

For headless or CI-style checks, prefer `cargo check` and package-specific tests before running the app.

## Related Skills

- Use `fix-errors` for compiler, clippy, formatting, and test failure repair.
- Use `rust-unit-tests` for writing focused Rust test coverage.
- Use `castcodes-rebrand-surface` and `castcodes-fork-local-boundary` for policy-specific checks.
