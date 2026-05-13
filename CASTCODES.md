# CASTCODES.md

This repository is the CastCodes fork-local external rebrand of the upstream codebase.

## Naming Rules

- Product display name: `CastCodes`
- Binary/package slug: `cast-codes`
- New public Rust identifiers: `cast_codes`
- Public app ID: `dev.castcodes.CastCodes`
- Public URL scheme: `castcodes`
- Public config directory: `.cast-codes`

Keep internal crate/module names such as `warp_core`, `warpui`, `warp_terminal`, `warpify`, and inherited upstream dependency names unless a later full internal rename explicitly changes them.

## Cloud Boundary

The public CastCodes build must not call upstream hosted services by default. This includes upstream auth, upstream cloud APIs, upstream release feeds, Sentry, RudderStack, billing, hosted crash reporting, hosted telemetry, and shared-session services.

When cloud services are unavailable, UI and commands should be hidden, disabled, or explicitly marked unavailable rather than pretending CastCodes owns those services.

## Verification

Use the rebrand guard for public-surface changes:

```bash
./script/check_rebrand
```

Core identity checks:

```bash
cargo test -p warp_core --features local_fs
cargo check -p warp --bin cast-codes --features gui
```
