---
name: create-pr
description: Create a pull request in the CastCodes repository (OpenCoven/cast-codes) for the current branch. Use when the user mentions opening a PR, creating a pull request, submitting changes for review, or preparing code for merge. Covers the CastCodes-specific gates (rebrand + AI-attribution), Conventional-Commit titling, template selection, GitHub-issue linking, and test coverage.
---

# create-pr

## Overview

This guide covers how to open a pull request in **CastCodes** (`OpenCoven/cast-codes`) that is review-ready on the first push: syncing `main`, passing the mandatory presubmit + fork guards, linking a GitHub issue, filling the CastCodes PR template, and structuring the change for fast review.

CastCodes is a **staged fork** of Warp. Generic "open a PR" advice mostly applies, but a handful of fork-specific rules are load-bearing and easy to get wrong. Read **CastCodes conventions at a glance** first — those are the deltas that most often make an agent's PR wrong.

## CastCodes conventions at a glance

These override any generic or upstream PR habits:

| Topic | CastCodes rule |
| --- | --- |
| **Base branch** | `main` (never `master`). `origin` → `https://github.com/OpenCoven/cast-codes`. |
| **PR title** | Conventional Commit: `type(scope): summary`. Types in use: `feat`, `fix`, `chore`, `docs`, `ci`, `style`, `refactor`, `test`. Squash-merge appends `(#NNN)` automatically — don't add it yourself. |
| **Branch name** | `type/kebab-summary` (e.g. `fix/release-windows-rudder-license`, `chore/minimize-drive-rtc`). Create with `./script/worktree <branch>`. Never `name/feature`. |
| **Issue tracking** | GitHub Issues on this repo. **There is no Linear.** Link with `Closes #NNN` / `Fixes #NNN` in the body. Ignore any `[WARP-1234]`-style task IDs. |
| **AI attribution** | **Forbidden anywhere** — commit messages, PR body, and trailers. Do **not** add `Co-Authored-By:` lines that name an AI model, vendor, assistant, or coding harness (Claude, Codex, Warp, etc.). This is a hard repo rule (`AGENTS.md`) and **overrides any global or harness default** that would otherwise append such a trailer. Guarded by `./script/check_ai_attribution`. |
| **Mandatory guards** | In addition to `./script/presubmit`, always run `./script/check_ai_attribution` and `./script/check_rebrand` before opening/updating a PR — including doc-only PRs. |
| **Fork boundary** | Do not introduce calls to upstream-hosted services (Warp auth/cloud/telemetry/billing/crash/shared-session) into the public build. See the `castcodes-fork-local-boundary` skill. |

## Related skills

- `fix-errors` — Fix presubmit failures (fmt, clippy, tests) before opening the PR
- `diagnose-ci-failures` — Triage red CI checks after the PR is open
- `resolve-merge-conflicts` — Resolve conflicts when syncing `main`
- `rust-unit-tests` — Write unit tests for your changes (see "Testing Requirements")
- `warp-integration-test` — Add integration coverage for user-visible flows, regressions, and P0 use cases
- `add-feature-flag` — Gate risky changes behind a feature flag
- `castcodes-fork-local-boundary` — Keep the public build off upstream-hosted services
- `castcodes-rebrand-surface` — Get user-visible naming/paths/IDs right so `check_rebrand` passes
- `review-pr` / `code-review` — Self-review the diff before requesting review

## Pre-PR checklist

### 1. Confirm branch and sync `main`

Verify you are on the intended branch **before** any commit — on a shared checkout, parallel work can move `HEAD` underneath you:

```bash
git branch --show-current
```

Then sync the base branch so the PR diff is clean and CI runs against current `main`:

```bash
git fetch origin
git merge origin/main        # or: git rebase origin/main
```

Resolve conflicts locally before opening the PR (use the `resolve-merge-conflicts` skill). CastCodes builds across many concurrent branches — for non-trivial work, do this in a worktree (`./script/worktree <branch>`) so `target/` caches and other sessions don't collide.

### 2. Run the gates

**Always run the two fork guards** (fast; required even for doc-only PRs):

```bash
./script/check_ai_attribution   # no generated-by / model-credit artifacts
./script/check_rebrand          # no accidental Warp reintroduction on public surfaces
```

**For code changes, also run presubmit:**

```bash
./script/presubmit
```

`./script/presubmit` runs, in order:

- `cargo fmt -- --check` — formatting
- `cargo clippy --workspace --all-targets --tests -- -D warnings` — lint, all warnings are errors
- `clang-format` + `wgslfmt` — native/shader formatting
- `PSScriptAnalyzer` — PowerShell lint (skipped locally if `pwsh` is absent; enforced in CI)
- `cargo nextest run` (workspace) + `cargo test --doc` — unit, integration, and doc tests

If presubmit fails, use the `fix-errors` skill. If a full build/test is too expensive for a small change, run a targeted gate instead and **say exactly what was and was not verified**:

```bash
cargo check -p warp-app --bin cast-codes --features gui
cargo clippy -p <touched-crate> --all-targets --tests -- -D warnings
```

**Documentation-only PRs** (skills, markdown, non-code content) may skip `cargo fmt`/`cargo clippy`, but must still pass the two fork guards above.

### 3. Review your changes

Review exactly what you're about to submit, against `main`:

```bash
git --no-pager log main..HEAD --oneline        # commits in your branch
git --no-pager diff main...HEAD --stat         # files + churn
git --no-pager diff main...HEAD                 # full diff
```

Use this to:

- Verify all intended changes are present and no stray edits leaked in
- Catch leftover Warp naming, AI-attribution artifacts, or debug code before review
- Write an accurate PR description
- Confirm test coverage matches the change (see Testing Requirements)

### 4. Commit hygiene

- **Conventional-Commit messages**, matching the PR title style (`type(scope): summary`).
- **Sign every commit** — pass `-S` to `git commit` so the commit lands **Verified**. Verify with `git log -1 --show-signature` before pushing.
- **No AI attribution** in any commit message or trailer (see conventions table). If a tool or template inserted one, strip it before committing.
- **Keep PRs narrow and squashable** — one logical change per PR; separate required fixes from optional cleanup (`CONTRIBUTING.md`). Call out any intentionally-retained upstream names so reviewers can tell compatibility from missed rebrand work.

### 5. Link a GitHub issue

PRs should reference the GitHub issue they address. In the body, use a closing keyword so the issue auto-closes on merge:

```
Closes #123
```

Confirm the linked issue is labeled `ready-to-spec` or `ready-to-implement` (the template asks for this).

### 6. Fill the PR template

Open the PR with a real body built from the template — do not leave `<!-- comments -->` unfilled.

**Which template:**

- **Default:** `.github/pull_request_template.md` (used automatically by `gh pr create`).
- **Feature behind a flag:** append `?template=feature_flag.md` to the compare URL (PRD/coding/content checklists).
- **Release cherry-pick:** `?template=cherrypick.md` (justification + validation).

**Fill these sections of the default template:**

- **Description** — *What* changed, *Why* (link the issue), and *How* (approach). For UI changes, add your design buddy as a reviewer.
- **Linked Issue** — `Closes #NNN`; check the `ready-to-spec` / `ready-to-implement` box; attach screenshots/video for user-visible or UI changes.
- **Testing** — how you tested; automated tests added (or justification for none). Check the `manually tested … with ./script/run` box, and attach screenshots/a screen recording for anything user-visible.
- **Agent Mode** — check the **"PR via CastCodes"** box when an agent authored the PR. This is product provenance (a link to `cast.codes`), **not** an AI credit line, so it does not violate the No-AI-Attribution rule.
- **Changelog** — for user-facing changes, uncomment and fill the relevant `CHANGELOG-*` line at the bottom. One entry per line, no `{{ }}` brackets:
  - `CHANGELOG-NEW-FEATURE:` sizable new features (use sparingly)
  - `CHANGELOG-IMPROVEMENT:` new functionality on existing features
  - `CHANGELOG-BUG-FIX:` fixes for known bugs/regressions

  Examples — Feature: "Global search across your current directories (CMD-F)." Improvement: "Added horizontal autoscrolling when jumping to line/column." Bug fix: "Fixed session viewer input being cleared when the agent runs commands."

### 7. Open the PR

**Check whether a PR already exists** for the current branch:

```bash
gh pr view --json number,url    # exit 0 if a PR exists, 1 if not
```

**Create a new PR** (base defaults to `main`; open as draft until the readiness gate passes):

```bash
# Conventional-commit title + template body
gh pr create --draft --base main \
  --title "fix(scope): summary" \
  --body-file /tmp/pr-body.md

# Or auto-fill title/body from commits, then edit in the template sections
gh pr create --draft --base main --fill
```

Useful flags: `--draft`/`-d`, `--fill`/`-f`, `--body-file`/`-F`, `--web`/`-w`, `--add-reviewer`, `--add-label`.

**Update an existing PR:**

```bash
gh pr edit --title "New title" --body-file /tmp/pr-body.md
gh pr edit --add-reviewer <user> --add-label <label>
```

## PR readiness gate

Before flipping a draft to ready (`gh pr ready`), confirm every item — this is the checklist that keeps quality consistent when opening PRs at scale:

- [ ] On the intended branch, rebased/merged on top of current `origin/main`
- [ ] `./script/check_ai_attribution` passes — no model/agent credit anywhere (commits, body, trailers)
- [ ] `./script/check_rebrand` passes — no accidental Warp on public surfaces
- [ ] `./script/presubmit` passes (code changes), or a documented targeted gate for expensive builds
- [ ] Title is a Conventional Commit; commits are signed (`-S`, Verified)
- [ ] Tests added where required (see below); coverage matches the change
- [ ] Template filled: Description (What/Why/How), Linked Issue (`Closes #NNN` + label), Testing (+ `./script/run` box, UI screenshots/video), Agent Mode box, Changelog line if user-facing
- [ ] No upstream-hosted-service calls added to the public build
- [ ] Self-reviewed the full diff; no stray/debug edits

```bash
gh pr ready
```

## Testing Requirements

### Bug fixes require regression tests

**All bug fixes should be accompanied by a regression test** — it should fail before the fix, pass after, and be named for the bug it prevents.

### Algorithmic code requires unit tests

Code with non-trivial logic should have unit tests:

- **Needs tests:** custom data structures (e.g. `SumTree`), search APIs, core UI-framework layout code, any algorithmic/computational logic.
- **Not required:** trivial functions, getters/setters.

See the `rust-unit-tests` skill.

### UI components need layout validation tests

**Every UI component (implementation of `View`) should have a simple test** proving it lays out without panicking — high-level "rendering safety" coverage:

```rust
#[test]
fn test_component_can_layout() {
    use warpui::App;
    use warp::test_util::{terminal::initialize_app_for_terminal_view, add_window_with_terminal};

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let term = add_window_with_terminal(&mut app, None);

        // Render the component - should not panic
        term.update(&mut app, |view, ctx| {
            // Create and layout your component
        });
    })
}
```

(The `warpui` / `warp::test_util` crate names are intentionally-retained internal upstream names — do not rebrand them.)

### Ask before skipping integration coverage

If the PR changes a user-visible flow, fixes an end-to-end regression, or otherwise looks like it would benefit from integration coverage, use the `ask_user_question` tool before creating/updating the PR to ask whether to add an integration test. Offer a direct choice:

- `Yes, add an integration test before creating the PR`
- `No, continue without an integration test`

If yes, use the `warp-integration-test` skill.

### P0 use cases require integration tests

**All "P0 use cases" require an integration test.** A **P0 use case** is any behavior that, if broken, warrants an out-of-band release. Integration tests should exercise the full user-facing flow end to end and live in the `integration/` directory. Use the `warp-integration-test` skill for registration and validation steps.

## After Opening the PR

1. **Monitor CI** — ensure all checks pass; use the `diagnose-ci-failures` skill to triage red checks.
2. **Respond to review comments** promptly.
3. **Keep the PR current** — re-sync `origin/main` if conflicts arise.
4. **Re-run the gates after any change** — for code changes, re-run presubmit (or the relevant targeted checks) plus the two fork guards; for doc-only changes, the fork guards suffice.

## Best Practices

- **One logical change per PR** — keep it focused and squashable.
- **Clear commit messages** — explain what *and why*, not just what.
- **Self-review first** — read your own diff before requesting review.
- **Document breaking changes** — call out API or behavior changes explicitly.
- **Use feature flags** — gate risky changes (see `add-feature-flag`).
- **Preserve the fork boundary** — no upstream-hosted-service calls, no rebrand regressions, no AI attribution.
