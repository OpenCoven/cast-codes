# Contributing to CastCodes

CastCodes accepts small, focused patches that preserve the existing architecture and keep public fork behavior honest.

## Rebrand Boundaries

- Use `CastCodes` for public UI, docs, app metadata, installers, packages, and release surfaces.
- Use `cast-codes` for public binaries, packages, and slugs.
- Use `castcodes` for public URL schemes and reverse-DNS organization segments.
- Do not blindly rename internal crates, modules, inherited dependency names, protocol identifiers, or historical tests.
- Do not add upstream service endpoints to the public CastCodes build.

## Development

```bash
./script/run
```

For rebrand-sensitive work:

```bash
./script/check_rebrand
cargo check -p warp --bin cast-codes --features gui
```

Add focused tests when changing path behavior, channel identity, app IDs, URL schemes, or cloud-gating logic.

## Pull Requests

Keep PRs narrow and squashable. Separate required fixes from optional cleanup. Mention any intentional remaining legacy upstream names references so reviewers can distinguish compatibility from missed rebrand work.

## Conduct

All contributors are expected to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
