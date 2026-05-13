# CastCodes Command Map

## High Signal Commands

```bash
./script/run
./script/check_rebrand
cargo test -p warp_core --features local_fs castcodes
cargo check -p warp --bin cast-codes --features gui
./script/bundle --channel oss --check-only
```

## Existing Repo Checks

Use existing repo skills for broader Rust hygiene:

```bash
cargo fmt -- --check
cargo clippy --workspace --exclude warp_completer --all-targets --all-features --tests -- -D warnings
cargo clippy -p warp_completer --all-targets --tests -- -D warnings
cargo nextest run -p <package_name>
cargo nextest run -E 'test(<substring>)'
```

## Platform Scripts

- Cross-platform run entrypoint: `script/run`
- macOS app launch/bundle behavior: `script/macos/run`, `script/macos/bundle`
- Linux package behavior: `script/linux/bundle*`
- Windows bundle behavior: `script/windows/bundle.ps1`, `script/windows/windows-installer.iss`
- Shared bundle dispatcher: `script/bundle`

## Feature Notes

- `gui`: grouped graphical app feature for CastCodes.
- `plugin_host`: enables plugin support and is usually checked directly early in app lifecycle.
- `skip_login`: used by tests and fast local paths; do not use it to justify public hosted-service exposure.
- `release_bundle`: bundle-only feature for release packaging behavior.

## Reporting

When completing work, list the exact commands run and whether any planned command was skipped. If a long-running app launch was not needed, say that compile or focused tests covered the change instead.
