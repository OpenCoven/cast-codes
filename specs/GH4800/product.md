# PRODUCT.md — Wayland-compatible Warp toggle command

Issue: https://github.com/warpdotdev/warp/issues/4800

## Summary
Warp cannot safely register its own global hotkey while running as a native Wayland client, so Linux Wayland users need a standards-based way to bind their desktop environment's own shortcut handling to Warp. Add a Linux shortcut command that users can run from GNOME, KDE, Sway, Hyprland, and other compositor shortcut settings to show, focus, or hide Warp without depending on `wmctrl`, X11 grabs, or desktop-environment-specific integrations.

Figma: none provided.

## Problem
Today the settings UI disables the global hotkey controls on Wayland and links to docs. Users can switch Warp to X11/Xwayland or configure their compositor manually, but Warp does not expose a dedicated command that can be bound to a compositor shortcut. Running `warp-terminal` from a shortcut launches Warp or opens another window rather than toggling the already-running app.

## Goals / Non-goals
Goals:

- Give Linux Wayland users a copyable, documented command that they can bind in their system/compositor keyboard shortcut settings.
- Keep Warp's in-app global hotkey registration disabled on native Wayland, because Warp cannot own global key capture there.
- Support the same command across supported Linux desktop environments through freedesktop-compatible application activation and Warp's existing single-instance IPC.
- Preserve the current X11/Xwayland global hotkey behavior for users who run Warp with X11 window management.
- Avoid external dependencies such as `wmctrl`, `xdotool`, shell scripts, or compositor-specific extensions.

Non-goals:

- Warp does not implement compositor-native shortcut registration for GNOME, KDE, Sway, Hyprland, or any other specific desktop environment.
- Warp does not claim full dedicated hotkey-window/quake-mode parity on native Wayland when the compositor does not allow programmatic positioning, hiding, or activation.
- Warp does not re-enable X11 global key grabs while running as a native Wayland client.
- Warp does not maintain or publish desktop-environment-specific extensions as part of this feature.

## Behavior
1. On Linux Wayland, the Global hotkey row in Settings > Features continues to make clear that Warp cannot register an in-app global hotkey on Wayland. It must not present the existing keybinding editor as if Warp can capture the shortcut itself.

2. On Linux Wayland, the same settings area provides a "System shortcut command" path for users who want global-key behavior through their compositor:
   - It explains that the user should create a custom shortcut in their desktop environment or window manager.
   - It shows the command to bind: `warp-terminal-toggle`.
   - It includes the equivalent invocation `warp-terminal --toggle` for users who prefer not to rely on the helper executable.
   - It offers a copy affordance for the command when copy-to-clipboard is available.

3. The command name is channel-aware wherever Warp already installs channel-specific launchers. Stable users see `warp-terminal-toggle`; Preview/Dev/Local/Oss builds see the matching helper name or a documented equivalent based on that channel's installed `warp-terminal...` command. Users never need to know the internal binary path under `/opt/warpdotdev/...`.

4. When the user runs the toggle command and no Warp instance for that channel is already running, Warp launches normally and opens one normal terminal window. The command exits successfully after handing off launch to Warp.

5. When the user runs the toggle command while Warp is running and no normal Warp window is focused, Warp attempts to show and focus the most recently active normal Warp window. If no normal window exists, Warp opens a new normal terminal window.

6. When the user runs the toggle command while a normal Warp window is focused, Warp hides or minimizes Warp's normal windows using the best behavior available on the current windowing system. Quake/dedicated hotkey windows are not included in this normal-window toggle.

7. On native Wayland, showing/focusing a running Warp window is best-effort and uses the activation context provided by the desktop shortcut launcher when available. If the compositor denies activation, Warp must not spin, repeatedly launch new instances, steal focus through non-standard tools, or depend on `wmctrl`; it should leave the existing window state intact and may request user attention if the platform supports it.

8. On native Wayland, hiding a visible Warp window is also best-effort. If the compositor or toolkit cannot hide an already-mapped window, minimizing is an acceptable fallback. If neither hiding nor minimizing is accepted, the command is a safe no-op rather than closing sessions or destroying terminal state.

9. On X11 and Xwayland, the toggle command uses the same observable show/hide-all-windows behavior as the existing "Show/hide all windows" global hotkey mode. Existing configured global hotkeys, keybindings, and the "Use Wayland for window management" setting continue to work exactly as they do today.

10. The command is idempotent with respect to process instances: repeatedly invoking it never creates an unbounded series of Warp processes. If an existing instance can be reached, the helper forwards the action to that instance and exits.

11. The command is safe to invoke from:
    - GNOME Custom Shortcuts.
    - KDE Custom Shortcuts.
    - Sway/Hyprland/i3-style config entries.
    - A shell prompt.
    - Desktop-entry `Exec` actions if Warp chooses to expose one later.

12. The settings copy does not tell users that `wmctrl` is required or installed by default. If docs mention `wmctrl` or compositor-specific snippets as optional community workarounds, they are clearly marked as optional and outside Warp's supported path.

13. The command targets the current installed Warp channel only. Invoking the Stable helper toggles Stable, invoking the Preview helper toggles Preview, and so on; it must not accidentally toggle a different channel's DBus service or windows.

14. If multiple normal Warp windows exist and Warp is not focused, the toggle command brings back the most recently active normal window when Warp can determine it. If Warp cannot determine recency, choosing any existing normal window is acceptable, but opening a new window is not unless there are no normal windows.

15. The dedicated hotkey window settings remain unavailable on native Wayland unless Warp can provide the behavior without compositor-specific APIs. Users who need the existing dedicated hotkey-window behavior are still directed to run Warp under X11/Xwayland or use a compositor/extension workflow outside Warp's supported path.

16. The Wayland settings guidance includes a concise setup outline:
    1. Open system keyboard shortcut settings.
    2. Create a custom shortcut.
    3. Use `warp-terminal-toggle` as the command.
    4. Assign the user's preferred keybinding.

17. The command returns a non-zero exit code and a concise terminal-facing error only when it cannot launch Warp or cannot contact/start the per-channel Warp application service. Unsupported focus or hide requests caused by Wayland compositor policy do not produce noisy terminal errors for normal shortcut use.

18. Accessibility and localization: the new settings text and copy button have accessible labels that describe the shortcut command and copy action. The row remains usable with keyboard navigation and screen readers.

## Success criteria
- A Linux Wayland user can bind a compositor shortcut to `warp-terminal-toggle` and use it to launch Warp if it is not running.
- If Warp is running, the same shortcut contacts the existing Warp instance instead of opening duplicate instances.
- On X11/Xwayland, the command toggles visibility consistently with the existing show/hide global hotkey behavior.
- On native Wayland, the command provides the best standards-based show/focus/minimize behavior available without `wmctrl` or desktop-environment-specific code, and the UI clearly sets expectations where compositor policy limits behavior.

## Validation
- Verify the settings guidance appears only for Linux Wayland and that X11/Xwayland global hotkey settings are unchanged.
- Verify `warp-terminal-toggle` and `warp-terminal --toggle` from a shell launch Warp when no instance is running.
- Verify repeated invocations while Warp is already running do not create duplicate processes.
- Verify a GNOME or KDE custom shortcut can run the command.
- Verify X11/Xwayland behavior matches the existing "Show/hide all windows" mode.
- Verify native Wayland fallback behavior on at least GNOME and KDE documents whether the compositor grants activation, minimizes, or denies a specific operation.

## Open product questions
1. Should the first release expose only the "show/hide all windows" target, or should it also expose an experimental dedicated-hotkey-window target for compositors where positioning and hiding happen to work?

2. Should the settings row link directly to Warp docs for per-desktop setup snippets, or keep setup instructions entirely in-app?

3. If native Wayland activation is denied, should Warp surface a one-time toast in the existing Warp window explaining that the compositor blocked focus, or should failures remain silent to avoid interrupting shortcut workflows?
