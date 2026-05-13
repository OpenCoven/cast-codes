# CastCodes Plugin Host Map

## Main Code Areas

- `app/src/plugin/**`: app plugin host, native/WASM host, service implementation, plugin app model.
- `app/src/settings_view/mcp_servers/**`: MCP server settings surfaces.
- `app/src/terminal/cli_agent_sessions/plugin_manager/**`: CLI-agent plugin managers for Codex, Claude, Gemini, and OpenCode.
- `app/src/terminal/input/skills/**`: skill-related terminal input surfaces.
- `app/src/search/ai_context_menu/skills/**`, `app/src/search/ai_context_menu/commands/**`: plugin-facing context menu surfaces.
- `app/Cargo.toml`: `plugin_host`, `mcp_server`, `mcp_oauth`, and related feature definitions.
- `crates/warp_cli/**`: CLI flags and plugin-host integration points.
- `crates/warp_js/**`, `crates/node_runtime/**`: JavaScript/runtime support used by plugins.

## Search Patterns

```bash
rg -n "plugin_host|mcp_oauth|mcp_server|plugin manager|PluginManager|plugins|skills|MCP|oauth" app/src crates app/Cargo.toml
```

For command exposure:

```bash
rg -n "slash|command palette|CommandPalette|commands|skills" app/src/terminal app/src/search
```

## Boundary Checks

- Plugin install/discovery should not require upstream hosted services for the public OSS app.
- MCP OAuth must not silently point OSS users at upstream auth unless explicitly configured.
- Plugin command UI must not advertise commands that are unavailable in the current channel.
- CLI-agent plugin managers should preserve provider-specific behavior and tests.

## Test Hints

Look for nearby `*_tests.rs` modules in the edited directory first. For compile coverage:

```bash
cargo check -p warp --bin cast-codes --features gui,plugin_host
```

Use a narrower package test when the edited crate has direct unit tests.
