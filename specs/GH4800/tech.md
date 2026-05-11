# TECH.md — Wayland-compatible Warp toggle command

Issue: https://github.com/warpdotdev/warp/issues/4800
Product spec: `specs/GH4800/product.md`

## Context
Warp currently treats native Wayland as unsupported for its in-app global hotkey feature, while X11/Xwayland continues to use global key grabs.

Relevant code:

- `app/src/terminal/keys_settings.rs:192` — `KeysSettings::global_hotkey_mode` returns `Disabled` immediately when `app.is_wayland()` is true, so the rest of the global-hotkey path is suppressed on native Wayland.
- `app/src/settings_view/features_page.rs:5360` — `GlobalHotkeyWidget` renders "Not supported on Wayland" plus a docs link instead of the dropdown/keybinding editor.
- `app/src/settings_view/features_page.rs:7138` — the Linux "Use Wayland for window management" setting warns that enabling Wayland disables global hotkey support.
- `crates/warpui/src/windowing/winit/delegate/global_hotkey.rs:14` — `GlobalHotKeyHandler` wraps the `global_hotkey` crate and is designed around platform-managed global shortcut registration.
- `crates/warpui/src/windowing/winit/delegate/global_hotkey.rs:83` — registered global hotkeys are forwarded into the UI event loop as `CustomEvent::GlobalShortcutTriggered`.
- `app/src/root_view.rs:329` — RootView registers the two existing global actions, `root_view:toggle_quake_mode_window` and `root_view:show_or_hide_non_quake_mode_windows`.
- `app/src/root_view.rs:1361` — `toggle_quake_mode_window` creates, shows, focuses, or hides the dedicated hotkey window.
- `app/src/root_view.rs:1442` — `show_or_hide_non_quake_mode_windows` implements the "Show/hide all windows" behavior for normal windows.
- `crates/warpui/src/windowing/winit/window.rs:1182` — the current winit window wrapper notes that setting visibility is unsupported on Wayland; `hide_app` and `hide_window` rely on `set_visible(false)`.
- `crates/warpui_core/src/windowing/system.rs:16` — `System::allows_programmatic_window_activation` already encodes that Wayland does not generally allow programmatic activation.
- `app/src/app_services/linux/mod.rs:28` — Linux release bundles already forward app startup arguments to a running Warp instance over the session bus.
- `app/src/app_services/linux/mod.rs:110` — Warp hosts an `org.freedesktop.Application` DBus service per channel/app id.
- `app/src/app_services/linux/mod.rs:126` and `app/src/app_services/linux/mod.rs:134` — `Activate` and `ActivateAction` are currently no-ops.
- `app/src/app_services/linux/mod.rs:142` — `Open` is implemented and forwards URIs into `crate::uri::handle_incoming_uri`.
- `crates/warp_cli/src/lib.rs:122` — `AppArgs` is the right parser surface for GUI-app flags that are not Oz CLI subcommands.
- `app/channels/stable/dev.warp.Warp.desktop:10` — Linux desktop entries launch the channel wrapper command, e.g. `warp-terminal %U`.
- `resources/linux/arch/app/warp.sh.template:6` — Linux package wrappers already support channel-specific launcher names and user flags files.

The product direction is not to resurrect native global key capture on Wayland. Instead, Warp should expose a per-channel toggle command that a compositor can run from its own shortcut system. The command should use existing single-instance DBus IPC, not external X11 tools.

## Proposed changes
1. Add a GUI app toggle flag and helper command surface.
   - Add `toggle_visibility: bool` to `warp_cli::AppArgs` in `crates/warp_cli/src/lib.rs`, exposed as `--toggle`.
   - Keep this in `AppArgs`, not `CliCommand`, so `warp-terminal --toggle` is treated as a GUI-app request and can participate in the existing Linux single-instance forwarding path.
   - Add a packaged helper executable or wrapper script named `warp-terminal-toggle` for Stable and equivalent channel-specific names for Preview/Dev/Local/Oss. The helper should exec the channel's normal launcher with `--toggle`, preserving user flag-file behavior where package wrappers already provide it.
   - Do not add `wmctrl`, `xdotool`, or compositor-specific dependencies to package metadata.

2. Forward the toggle action through the existing Linux DBus service.
   - In `app/src/app_services/linux/mod.rs`, update `pass_startup_args_to_existing_instance` so `args.toggle_visibility` calls `ExistingApplicationProxy::activate_action` instead of `open`.
   - Use a stable action name such as `toggle-visibility`. Keep it channel-scoped by relying on `DBusServiceHost::well_known_name()` and `ChannelState::app_id()`, as the current `Open` path already does.
   - Continue to pass normal URLs to `Open`; `--toggle` and URL opening should be mutually exclusive at the clap layer or resolved deterministically by prioritizing explicit URLs and returning a clear CLI error for invalid combinations.
   - Capture and forward freedesktop `platform_data` if the launching environment provides activation metadata such as an activation token. Initially this can be a small helper that reads known environment variables into the DBus platform-data map; if winit cannot consume the token yet, retain it in the action event type for a follow-up rather than discarding it at the DBus boundary.

3. Implement `Activate` and `ActivateAction` in Warp's DBus host.
   - Extend `ApplicationServiceEvent` with `Activate { platform_data }` and `ActivateAction { action_name, platform_data }`.
   - `ApplicationService::activate` should forward to the UI thread and activate the most recent normal window, opening a new one only if no normal windows exist.
   - `ApplicationService::activate_action("toggle-visibility", ...)` should dispatch a new root action, for example `root_view:toggle_normal_windows_from_external_shortcut`, rather than overloading URI parsing.
   - Unknown action names should return a DBus `Failed` or `InvalidArgs` error and log the action name without panicking.
   - Ensure the DBus task remains non-blocking and keeps the current teardown behavior in `DBusServiceHost::terminate`.

4. Add a root action specialized for external shortcut toggles.
   - Keep `root_view:show_or_hide_non_quake_mode_windows` unchanged for existing X11 global hotkeys until the new behavior is proven equivalent.
   - Add a new helper that enumerates normal windows, excluding `quake_mode_window_id()`.
   - If there are no normal windows, call the existing `open_new(&(), ctx)` path.
   - If Warp has an active normal window, hide/minimize normal windows:
     - On X11, AppKit, and Windows, use the existing `hide_app` behavior.
     - On Wayland, request minimization for normal windows instead of calling `set_visible(false)`, because the current wrapper documents visibility as unsupported on Wayland.
   - If Warp does not have an active normal window, call `activate_app` or `show_window_and_focus_app` on the most recently active normal window. Keep activation best-effort on Wayland; do not retry in a loop or spawn extra processes when focus is denied.
   - If a platform activation token is available and winit or a future platform extension exposes a way to consume it, thread it into this focus request. If not, keep the token-carrying type boundary so the integration can be completed without changing the DBus API.

5. Add or extend window-manager APIs for the Wayland fallback.
   - Add a platform `minimize_window(window_id)` or `minimize_windows(iter)` method to `crates/warpui_core/src/windowing/state.rs` and `crates/warpui_core/src/platform/mod.rs`.
   - Implement it in `crates/warpui/src/windowing/winit/window.rs` by calling the existing `winit::window::Window::set_minimized(true)` path used by `Window::minimize`.
   - Keep the headless and test implementations as no-ops or state updates matching existing `hide_window` test behavior.
   - Do not replace `hide_app` globally; hidden-window semantics differ from minimized-window semantics and existing X11/macOS/Windows behavior should not regress.

6. Update the Wayland settings UI.
   - In `GlobalHotkeyWidget`, replace the single "Not supported on Wayland. See docs." row with explanatory text plus the system shortcut command from `PRODUCT.md` Behavior #2.
   - Include a copy button if existing settings-page copy affordances can be reused without adding new UI infrastructure; otherwise render the command as selectable text and defer the copy button.
   - Keep the existing global-hotkey dropdown/keybinding editor hidden on native Wayland.
   - Leave the X11/Xwayland rendering path untouched.
   - Keep the Linux window-system warning but update it to mention that in-app hotkey registration is disabled on Wayland while the system shortcut command remains available.

7. Update Linux packaging and docs surfaces.
   - Add channel-specific helper wrappers next to the existing `warp-terminal` wrappers in the Debian, RPM, AppImage, and Arch packaging scripts.
   - If desktop-entry actions are supported consistently by the packaging pipeline, optionally add a "Toggle Warp" desktop action that invokes the helper; this is a convenience only and not required for the compositor custom-shortcut flow.
   - Update Warp docs linked by the settings row with the product setup outline and without recommending `wmctrl` as the supported path.

## End-to-end flow
1. The user presses a compositor-managed shortcut.
2. The compositor runs `warp-terminal-toggle`.
3. The helper execs `warp-terminal --toggle`.
4. If a per-channel Warp DBus service exists, the startup forwarding path sends `ActivateAction("toggle-visibility")` to that service and the helper process exits.
5. The running Warp instance receives the DBus event, dispatches the root action, and either opens, shows/focuses, hides, or minimizes normal windows according to the current platform and focus state.
6. If no DBus service exists, the process starts the full Warp app and launch handling opens one normal terminal window.

## Testing and validation
Behavior mapping from `product.md`:

- Behavior #1, #2, #16, #18: add UI/unit coverage around `GlobalHotkeyWidget` if the settings widget has existing render tests; otherwise manually verify the Wayland row in Settings > Features with keyboard navigation and a screen reader smoke test.
- Behavior #3: package tests or script assertions should verify each Linux channel produces a matching toggle helper and that the helper invokes the corresponding channel launcher with `--toggle`.
- Behavior #4, #10, #13: add a unit or integration test for `pass_startup_args_to_existing_instance` with `toggle_visibility` to ensure it calls `ActivateAction` on the channel's well-known DBus name and exits without opening duplicate windows.
- Behavior #5, #6, #8, #14: add root-view unit tests around the new external-toggle helper with synthetic window-manager state for no windows, one inactive normal window, one active normal window, multiple windows, and an existing quake window.
- Behavior #7, #11, #17: perform manual Wayland validation on GNOME and KDE. Record whether activation is granted, denied, or minimized for each desktop and make the docs match observed behavior.
- Behavior #9: run the existing X11 global hotkey path and a new `warp-terminal --toggle` smoke test under X11/Xwayland to confirm the visible show/hide behavior remains equivalent.
- Behavior #12: grep docs and packaging changes for `wmctrl`/`xdotool`; they should not be required dependencies or the primary recommendation.
- Behavior #15: verify native Wayland still hides the dedicated hotkey-window keybinding editor and does not imply full quake-mode support.

Suggested command checks:

- `cargo fmt`
- `cargo nextest run -p warp_cli`
- `cargo nextest run -p warp_app --no-fail-fast` or the narrow settings/root-view test subset once tests are added.
- Linux package-script dry runs for each supported package format touched by the helper wrapper.

## Parallelization
Parallel sub-agents would help after this spec is approved because the work splits across app IPC, UI, and packaging:

- Agent `linux-ipc-toggle`: local execution in `/workspace/warp`, branch `feature/wayland-toggle-ipc`. Owns `warp_cli::AppArgs`, Linux DBus forwarding, DBus host events, and root action plumbing. Coordinates with UI/packaging agents through the final action name and command flag.
- Agent `settings-docs-wayland-toggle`: local execution in a separate worktree such as `/workspace/warp-ui-toggle`, branch `feature/wayland-toggle-settings`. Owns `GlobalHotkeyWidget` copy, docs copy, and any UI tests. Waits for the canonical command names from `linux-ipc-toggle`.
- Agent `linux-packaging-toggle`: local execution in `/workspace/warp-packaging-toggle`, branch `feature/wayland-toggle-packaging`. Owns Debian/RPM/AppImage/Arch helper wrappers and package dry-run validation. Depends on the `--toggle` CLI flag name but not on the root action implementation.

The final implementation should land as a single PR after merging the three worktrees into one branch, because the user-facing feature is not useful unless the command, IPC handler, settings guidance, and package helper ship together. If only one engineer/agent is available, implement sequentially in this order: CLI/IPC/root action, window fallback, settings, packaging/docs, validation.

## Risks and mitigations
- Risk: Wayland compositors deny activation even when the command is launched from a custom shortcut. Mitigation: keep activation best-effort, request user attention where supported, avoid duplicate processes, and document compositor-specific observations.
- Risk: Replacing `hide_app` with minimization could regress X11/macOS/Windows. Mitigation: introduce a new external-shortcut action and only use minimization as a Wayland fallback.
- Risk: Packaging helper names drift between channels. Mitigation: derive helper names from the same package/channel suffix logic used by the existing launchers and add script assertions.
- Risk: DBus action handling becomes a second URI router. Mitigation: keep a small fixed action enum for app-level actions; continue using `Open` for URIs.
- Risk: Users expect dedicated hotkey window parity. Mitigation: settings and docs explicitly distinguish system shortcut toggling of normal windows from native Wayland quake-mode limitations.

## Follow-ups
- Investigate whether the current winit version can consume xdg-activation tokens from freedesktop `platform_data`; if not, track the upstream requirement separately.
- Revisit dedicated hotkey-window support if a standard portal or winit/Tauri abstraction provides global shortcut registration plus activation/positioning semantics that are not desktop-environment-specific.
- Consider telemetry for the settings copy button and toggle command success/failure only after the command behavior is stable and privacy review approves the event shape.
