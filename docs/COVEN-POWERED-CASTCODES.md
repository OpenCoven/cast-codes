# Coven-Powered CastCodes

CastCodes is the singular public proof surface for Coven.

The simple product story is:

> CastCodes is the product users open. Coven is the runtime that powers it.

CastCodes is a local-first AI coding workspace. Coven powers it with project-scoped harness sessions, runtime authority, local logs and artifacts, explicit policy boundaries, and orchestration primitives. OpenCoven is the umbrella behind that direction.

## Product Boundary

Public beginner docs should lead with this hierarchy:

1. **CastCodes** — the workspace and proof surface.
2. **Coven** — the daemon/runtime/API/session authority under the workspace.
3. **OpenCoven** — the organization, lab, and ecosystem behind the product direction.

Advanced docs may mention other clients when needed, but beginner/product copy should not make users learn comux, OpenMeow, OpenClaw bridge details, optional clients, or historical prototypes before they understand why CastCodes matters.

Preferred public wording:

- `Coven powers CastCodes.`
- `CastCodes is a local-first AI coding workspace powered by Coven.`
- `Run Codex, Claude Code, and future harnesses as visible project-scoped lanes.`
- `Inspect their work, preserve context, review diffs, verify changes, and merge with confidence.`
- `Coven is invisible until you need to trust it. CastCodes is where you feel it.`

Avoid public-first wording that presents OpenCoven as a bundle of optional surfaces:

- `Coven works with comux, OpenMeow, OpenClaw, CastCodes, and future clients.`
- `comux is the cockpit.`
- `OpenMeow is the intake layer.`
- `OpenClaw is required for orchestration.`
- `Users can choose from many optional OpenCoven surfaces.`

## Runtime Authority

Coven stays the runtime authority. CastCodes should not duplicate policy in a way that can drift from the daemon.

Coven owns or should own:

- project-root and working-directory validation;
- harness identity and launch policy;
- session lifecycle state;
- append-only event history;
- log and artifact redaction rules;
- approval and action policy;
- handoff records between harnesses; and
- compatibility contracts for local runtime APIs.

CastCodes owns the user-facing workspace:

- terminal tabs and workspace lanes;
- editor and file context;
- agent panel and harness picker;
- changed-file and diff review surfaces;
- verification result display;
- merge, PR, archive, and cleanup actions behind explicit approval;
- command-palette rituals and templates; and
- retrospective and handoff views at the end of a task.

## What comux Proved

comux is reference/prototype evidence, not the future-facing flagship product. Its durable primitives should be folded into CastCodes-native concepts.

| comux primitive | CastCodes target |
| --- | --- |
| Pane | Agent lane / terminal tab / workspace lane |
| Worktree isolation | CastCodes/Coven isolated task lane |
| Agent launcher registry | CastCodes harness picker backed by Coven/Cast Agent |
| Multi-select launch | Multi-harness CastCodes lane creation |
| Ritual | CastCodes command palette ritual/template |
| File browser/diff | Native editor diff/review surface |
| Merge/PR flow | CastCodes review, verification, PR, cleanup workflow |
| Lifecycle hooks | Coven/Cast Agent events and hooks |
| Coven bridge | Direct CastCodes/Coven integration |

Public docs can say:

> comux proved the terminal cockpit model. Its durable primitives are being folded into CastCodes so Coven has one primary product surface.

## Public Mentions To Retire

Retire these from beginner/product copy:

- comux as the active public cockpit;
- OpenMeow as required intake;
- OpenClaw bridge as the primary story;
- optional clients as a first-contact explanation; and
- broad ecosystem diagrams before the CastCodes + Coven story is clear.

Keep them only in migration, compatibility, legacy, or advanced architecture docs.

## Migration Phases

### Phase 1: Public Framing

- CastCodes README leads with `CastCodes is a local-first AI coding workspace powered by Coven`.
- Coven README and docs lead with `Coven powers CastCodes`.
- Public roadmap treats comux as proof/migration input, not a second flagship cockpit.
- Demo-loop language shifts from `comux + Coven` to `CastCodes + Coven`.

### Phase 2: CastCodes Runtime Parity

- Launch an isolated agent lane from repository context.
- Choose harnesses through a CastCodes picker backed by Coven/Cast Agent.
- Create or attach a worktree/branch per lane.
- Show live terminal/output and structured session status.
- Preserve and render logs/artifacts safely.
- Show changed files and inline diffs.

### Phase 3: Review And Handoff

- Run verification gates and display results.
- Generate PR, merge, archive, and cleanup actions behind explicit approval.
- Record handoff packets between harnesses.
- End each task with a retrospective:
  - what worked;
  - what was missing; and
  - what should become a CastCodes/Coven issue.

### Phase 4: Rituals And Multi-Harness Workflows

CastCodes should support command-palette rituals/templates such as:

- Start Coding
- Review Stack
- Release Check
- Fix OpenClaw
- Coven Dogfood Quest
- Multi-Harness Review

These are roadmap/parity milestones unless the current app has shipped the exact workflow.

## Acceptance Criteria

- A new user can read CastCodes public docs and understand the product without learning comux, OpenMeow, or OpenClaw first.
- Coven docs identify CastCodes as the primary public workspace/proof surface.
- Coven remains described as daemon/runtime/API/session authority.
- comux references are clearly legacy, migration, reference, or advanced architecture context.
- Public claims distinguish shipped behavior from roadmap/parity milestones.
- Docs do not expose secrets, private gateway URLs, or personal infrastructure.
