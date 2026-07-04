# AGENTS.md - CastCodes

This repository is the CastCodes fork. Keep changes focused, local-first, and aligned with the staged rebrand.

## Hard Rules

### No AI Attribution

Do not add, preserve, suggest, or generate any credit line, footer, badge, signature, hidden marker, metadata, commit trailer, generated-file header, README note, documentation note, source comment, or UI label that credits an AI model, AI assistant, AI vendor, coding agent, or generation tool.

Forbidden attribution includes phrasing such as generated with/by, created with/by, made with/by, powered by AI, or co-authored-by lines that name an AI tool, model, vendor, or coding harness.

If a scaffold, agent, template, formatter, previous output, or merge artifact inserts AI attribution, remove it before finalizing. Do not ask whether to keep it. CastCodes output should read as authored by the project maintainers only.

AI/model names may appear only when they are part of real product behavior, dependency documentation, API configuration, compatibility notes, user-requested explanatory text, or tests for those behaviors. They must never appear as credit, authorship, provenance, or generated-by attribution.

Run this guard before submitting docs, specs, prompts, or generated artifacts:

```bash
./script/check_ai_attribution
```

### Rebrand Guard

CastCodes is a staged fork. Do not run blind repository-wide replacements. Many internal crate names and compatibility references intentionally still use upstream names.

Run this guard before public-surface changes:

```bash
./script/check_rebrand
```

## Phase 1 Design Contract

CastCodes should feel like a sleek, minimalist, editor-grade workspace: dark-first, dense, precise, and calm. Avoid decorative chrome, chunky cards, oversized marketing-style copy, and one-off visual decisions.

Use the shared token files and theme notes as the source of truth:

- `resources/design-tokens.css`
- `app/src/themes/default_themes.rs::castcodes_dark`
- `DESIGN-CHANGES.md`

Core constraints:

- Background: `#0f0f12`
- Surface: `#161619`
- Elevated surface: `#1e1e22`
- Title/status chrome: `#0a0a0d`
- Border: `rgba(255,255,255,0.08)`
- Text primary: `#e8e8ed`
- Text secondary: `#8e8e9a`
- Text muted: `#5a5a65`
- Accent: `#7c3aed`
- Accent hover: `#6d28d9`
- Gold accent: `#d4a84b`, highlights only
- Radius: 4px for compact items, 6px for controls, 8px maximum for larger surfaces
- Motion: 100-150ms ease-in-out, no bouncy effects

For UI work, prefer semantic tokens or local constants that match these values. Do not introduce a new palette unless the design contract is intentionally updated first.

## Verification

Use the smallest meaningful gate for the touched area:

```bash
./script/check_ai_attribution
./script/check_rebrand
cargo check -p warp-app --bin cast-codes --features gui
```

If the full app check is too expensive for the current change, run a targeted check and say exactly what was and was not verified.

## Pull Requests

The `create-pr` skill (`.agents/skills/create-pr/SKILL.md`) is the source of truth for opening PRs. Key rules for this fork:

- Base branch is `main` (never `master`); remote is `OpenCoven/cast-codes`. Branch names are `type/kebab-summary`; PR titles are Conventional Commits (`type(scope): summary`).
- Issues are tracked on GitHub, not Linear. Link with `Closes #NNN`.
- Before opening or updating any PR, run `./script/check_ai_attribution` and `./script/check_rebrand`; for code changes also run `./script/presubmit`.
- The **No AI Attribution** hard rule applies to commit messages, PR bodies, and trailers — do not add `Co-Authored-By:` lines naming an AI model, vendor, assistant, or coding harness, even if a global default would. The template's "PR via CastCodes" box is product provenance, not an AI credit, and is fine to check.
