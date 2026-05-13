---
name: castcodes-plugin-host
description: Work on CastCodes plugin host, skill, MCP, CLI-agent plugin manager, native or WASM plugin host, app plugin, and plugin-related command behavior. Use when editing plugin support or adding plugin-facing capability while preserving the public OSS hosted-service boundary.
---

# castcodes-plugin-host

## Workflow

Use this skill for plugin-host and plugin-facing work in the app. Read `references/plugin-map.md` when ownership is unclear.

Before editing:

1. Identify whether the change is in the native/WASM plugin host, app plugin service, terminal CLI-agent plugin manager, MCP settings, bundled skills, or command exposure.
2. Confirm whether the change requires the `plugin_host` Cargo feature.
3. Check whether the plugin path can reach hosted auth, MCP OAuth, cloud agents, shared sessions, or upstream service URLs. If yes, use `castcodes-fork-local-boundary`.
4. Keep plugin behavior local and explicit in public OSS builds unless CastCodes-owned infrastructure is configured.

## Implementation Rules

- Preserve existing plugin host module boundaries under `app/src/plugin/**`.
- Keep CLI-agent plugin manager behavior scoped to the provider manager being edited.
- Do not add marketplace or install metadata for repo-local skills unless the user explicitly asks for plugin packaging.
- Do not make plugin discovery depend on upstream hosted services in the public OSS path.
- For plugin commands that appear in UI, verify command palette and slash-command availability matches actual runtime support.
- For MCP OAuth or cloud-backed plugin behavior, require explicit unavailable or disabled OSS behavior.

## Verification

Run focused tests for the edited module when present. At minimum for app plugin-host code:

```bash
cargo check -p warp --bin cast-codes --features gui,plugin_host
```

If the change touches public identity or hosted services, also run:

```bash
./script/check_rebrand
cargo test -p warp_core --features local_fs castcodes
```

If `plugin_host` is not needed for the touched path, explain why the normal `gui` check is sufficient.

## Related Skills

- Use `castcodes-fork-local-boundary` for hosted service or MCP OAuth exposure.
- Use `castcodes-ui-work` for plugin commands, settings, or command-palette surfaces.
- Use `castcodes-dev-loop` for broader verification command selection.
