---
name: castcodes-rebrand-surface
description: Guard CastCodes public rebrand surfaces. Use when changing user-visible naming, app IDs, binary/package slugs, URL schemes, public config paths, bundle metadata, docs, release assets, installer metadata, or when deciding whether an inherited Warp name should remain internal compatibility state.
---

# castcodes-rebrand-surface

## Workflow

Start from the CastCodes contract before editing public surfaces:

- Public product name: `CastCodes`
- Public binary/package slug: `cast-codes`
- Public Rust identifier prefix for new public code: `cast_codes`
- Public app ID: `dev.castcodes.CastCodes`
- Public URL scheme: `castcodes`
- Public config directory: `.cast-codes`

Preserve inherited internal crate and module names such as `warp_core`, `warpui`, `warp_terminal`, `warpify`, and upstream dependency names unless the user explicitly asks for a full internal rename. Do not run blind repo-wide replacements.

## Check Public Surfaces

For public naming or packaging changes, read `CASTCODES.md` and `references/repo-map.md`, then inspect the relevant surface:

- Docs and contributor surfaces: `README.md`, `FAQ.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`
- Bundle and app metadata: `app/Cargo.toml`, `app/src/bin/oss.rs`, `script/update_plist`, platform bundle scripts, Linux package resources, Windows installer files
- Core identity code: `crates/warp_core/src/brand.rs`, `crates/warp_core/src/paths.rs`, `crates/warp_core/src/channel/state.rs`

When a Warp reference is found, classify it before changing it:

- Public surface: rename or explain why it is intentionally allowed.
- Compatibility boundary: keep it and prefer a narrow allowlist.
- Internal crate/module/dependency: keep it unless the task is a full internal rename.

## Verification

For any public-surface change, run:

```bash
./script/check_rebrand
cargo test -p warp_core --features local_fs castcodes
```

For app or bundle identity changes, also run:

```bash
cargo check -p warp --bin cast-codes --features gui
./script/bundle --channel oss --check-only
```

If a check is too expensive for the current turn, say exactly which check was skipped and why.

## Related Skills

- Use `castcodes-dev-loop` to choose the broader local build/check command set.
- Use `castcodes-fork-local-boundary` when a rebrand change touches hosted services or upstream URLs.
