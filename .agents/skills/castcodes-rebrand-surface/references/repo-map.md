# CastCodes Rebrand Map

## Source Of Truth

- `CASTCODES.md`: canonical naming and cloud-boundary policy.
- `README.md`: public scope and build commands.
- `script/check_rebrand`: guarded public-surface path list and intentional Warp allowances.

## Public Identity Paths

- `crates/warp_core/src/brand.rs`: product constants.
- `crates/warp_core/src/brand_tests.rs`: identity tests.
- `crates/warp_core/src/paths.rs`: config/data/cache path naming.
- `crates/warp_core/src/paths_tests.rs`: path expectations.
- `crates/warp_core/src/channel/state.rs`: default OSS channel identity.
- `app/src/bin/oss.rs`: public CastCodes binary wrapper and embedded plist.
- `app/Cargo.toml`: bin names, bundle metadata, feature comments.
- `script/run`, `script/macos/run`, `script/update_plist`, `script/*/bundle*`: platform launch and bundle surfaces.
- `resources/linux/**`, `script/windows/windows-installer.iss`: installer/package naming.

## Search Patterns

Use targeted searches rather than global replacement:

```bash
rg -n "Warp|warp\\.dev|warpdotdev|dev\\.warp|\\.warp|warp-terminal|warp-oss|warp://" \
  README.md FAQ.md CONTRIBUTING.md SECURITY.md CODE_OF_CONDUCT.md CASTCODES.md \
  Cargo.toml app/Cargo.toml app/src/bin/oss.rs app/channels script resources .github
```

Use `script/check_rebrand` as the real guard because it includes intentional compatibility allowances.

## Decision Rules

- Rename public-facing copy, package metadata, app IDs, URL schemes, and config paths to CastCodes.
- Keep internal crate names and inherited module names while this remains a staged external rebrand.
- Keep upstream dependency repository URLs if the dependency still comes from upstream.
- Prefer adding a narrow `script/check_rebrand` allowlist entry over weakening the guard.
