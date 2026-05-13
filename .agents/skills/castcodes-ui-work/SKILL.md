---
name: castcodes-ui-work
description: Implement or review CastCodes UI changes. Use when editing UI, settings, menus, onboarding, command palettes, unavailable hosted-service states, public copy, or visual affordances in the CastCodes app while preserving shared Warp UI conventions and CastCodes OSS boundaries.
---

# castcodes-ui-work

## Workflow

Read `.agents/skills/warp-ui-guidelines/SKILL.md` before UI edits. Keep those shared component rules in force, then apply the CastCodes-specific checks below.

For each UI change:

1. Identify whether the surface is public CastCodes copy, inherited internal terminology, or compatibility state.
2. Reuse existing view components, themes, menus, and model patterns before adding one-offs.
3. If the UI reaches hosted services, apply `castcodes-fork-local-boundary` first.
4. Remove dead affordances in OSS builds. Prefer hidden, disabled, or explicit unavailable states over clickable flows that fail later.
5. Add focused tests when the UI change changes availability, copy, commands, or channel-specific behavior.

## CastCodes UI Rules

- Public product copy should say `CastCodes`, not Warp, unless the text is intentionally explaining inherited compatibility.
- OSS builds should not invite sign-in, billing, upstream cloud, hosted telemetry, hosted crash reporting, shared sessions, or upstream release-feed flows.
- Do not fork shared button themes or hard-code colors when existing `ActionButtonTheme`, `Theme`, or appearance accessors fit.
- Avoid adding new feature-specific design primitives when a shared component already expresses the state.
- Keep command palette, settings, onboarding, resource center, and menu entries aligned with actual OSS availability.

## Common Areas

Read `references/ui-map.md` when the relevant UI ownership is not obvious.

Use targeted searches for labels and commands before editing:

```bash
rg -n "Warp|CastCodes|cloud|sign in|billing|telemetry|crash|shared session|update|Oz" app/src crates
```

## Verification

Run the focused test or compile check for the touched area. For most UI code:

```bash
cargo check -p warp --bin cast-codes --features gui
```

For public copy or product identity:

```bash
./script/check_rebrand
```

For hosted-service availability changes, add or update tests proving the OSS UI does not expose the unavailable flow.

## Related Skills

- `warp-ui-guidelines`
- `castcodes-rebrand-surface`
- `castcodes-fork-local-boundary`
- `castcodes-dev-loop`
