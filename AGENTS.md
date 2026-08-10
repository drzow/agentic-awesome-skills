# Repository Guidelines

## What This Repo Is

AAS Core (Agentic Awesome Skills) — an installable library of 2,007+ agent skills distributed as Markdown skill files, npm package, and mirrored plugin bundles. The canonical product is the local catalog; agents search it, choose skills, validate selections, and produce reproducible stack manifests.

This is a fork of the upstream Agentic Awesome Skills project. The primary divergence is the addition of the AAS MCP server written in Rust (replacing the original JavaScript implementation). The JS MCP server is retained only to simplify pulling upstream SKILLS updates — it is not used or maintained.

## Structure at a Glance

- `skills/<skill-id>/SKILL.md` — **canonical skill source**. Lowercase, hyphenated IDs. Skills may include subdirectories (references, examples, scripts). The entire `skills/<skill-id>/**` subtree is treated as skill content.
- `plugins/agentic-awesome-skills/`, `plugins/agentic-awesome-skills-claude/` — **mirrored plugin distributions** of canonical skills. Must be synchronized when source changes.
- `plugins/agentic-bundle-*` — curated skill bundles for specific domains (web, security, data, etc.).
- `tools/scripts/` — Node and Python scripts for validation, indexing, sync, auditing, releases. Tests live in `tools/scripts/tests/`.
- `apps/web-app/` — hosted catalog browser (Vite + TypeScript). Run `npm run app:dev`, `npm run app:test`.
- `data/*.json`, `skills_index.json`, `CATALOG.md` — **generated artifacts**. Never commit these in contributor PRs.
- `docs/`, `docs_zh-CN/`, `docs/vietnamese/` — user and contributor documentation.

## Commands You Need

```bash
npm ci                               # install root deps (Node >= 22)
npm run validate                     # validate skill frontmatter, required sections, schema
npm run validate:references          # check reference integrity across skills
npm run security:docs                # safety checks for commands, credentials, network guidance
npm run test                         # run local test suite (Node assertions + Python unittest)
npm run test -- --local              # same (explicit)
npm run test -- --network            # run network-dependent tests only
ENABLE_NETWORK_TESTS=1 npm run test  # run all tests including network ones
npm run build                        # full sync chain: validate → index → bundles → metadata → catalog → build
```

**PR gate:** `npm run validate && npm run test && npm run security:docs`

**Sync chain:** `npm run chain` runs the full pipeline (validate, plugin-compat-sync, index, bundles-sync, metadata-sync, catalog, aas-v1-catalog).

**Release:** `npm run release:prepare` then `npm run release:publish`. Never hand-edit version surfaces.

## Critical Constraints

- **Generated artifacts are maintainer-owned.** Files in `data/`, `CATALOG.md`, `skills_index.json` must not appear in contributor PRs. CI enforces this.
- **Skills are Markdown with frontmatter.** Every skill needs: YAML frontmatter (name, description, risk, source, date_added), `## When to Use`, examples, and limitations. Start new skills from `docs/contributors/skill-template.md`.
- **Mirrors must stay in sync.** Changing `skills/<id>/SKILL.md` means checking whether the skill is distributed under `plugins/agentic-awesome-skills/` or `plugins/agentic-awesome-skills-claude/`. Run `npm run bundles:sync` to synchronize.
- **Python scripts use bundled Python, not system.** Scripts in `tools/scripts/` that need Python call `tools/scripts/run-python.js` which handles the environment. Do not assume `python3` is available.
- **Test shards are supported:** `npm run test -- --shard-index=0 --shard-count=4`.

## Skill Authoring

1. Create directory: `skills/<lowercase-hyphen-id>/`
2. Add `SKILL.md` with YAML frontmatter + required sections from the template at `docs/contributors/skill-template.md`.
3. The `risk` field must be one of: `safe`, `none`, `moderate`, `critical`. Critical-risk skills require explicit maintainer review.
4. Run `npm run validate` locally before submitting.

## AAS Core CLI

- `aas` — main CLI binary (installed via npm bin). Provides skill discovery and stack management.
- `aas-mcp` — MCP server for agent integration.
- `aas stack validate` — validate a proposed stack manifest.
- `aas stack plan` — produce an immutable plan without applying changes.

## Contributing

- Use conventional-style commits: `feat:`, `fix:`, `docs:`, `chore:`.
- PRs must be source-only (no generated artifacts). CI blocks them if they include data/*.json, CATALOG.md, or skills_index.json.
- PR template includes a Quality Bar Checklist and requires linked issues when applicable.

## Development Methodology

Every task — feature development, debugging, or fixes — follows a multi-agent workflow:

1. **Three developers** each receive the same task and work on separate git worktrees.
2. A **master implementer** collects all three implementations, takes the best from each, and synthesizes a complete solution.
3. **Three reviewers** provide critical feedback on the synthesized solution. This loops: developers → master developer → reviewers → developers until all reviewers are satisfied.
4. The final product goes to a **final reviewer** for sign-off.
5. The approved solution is committed and pushed only to the local repo (the git worktree source). It should merge to `main` if not already on it.

Do not push upstream to any remote repos unless explicitly instructed.

## Agent-Specific Notes

- **Read from current base.** After creating a worktree or topic branch, re-read `AGENTS.md`, `.github/MAINTENANCE.md` (if present), and `package.json` from that branch. Instructions inherited from the checkout that launched the task supersede anything from another base — if a mandatory gate is absent on the current base, do not recover it from history or an installed copy; check `origin/main` for removal history first.
- **Never push directly to `main`.** Even when the user says "push to main" — that phrase names the target state, not the mechanism. Use topic branches and `npm run merge:batch` for maintainer merges. Let canonical-sync PRs own generated artifacts.
- **Use the maintainer skill for repo maintenance.** For PR merge batches, canonical sync, release work, or AAS Core changes, follow `skills/antigravity-maintainer-batch-release/SKILL.md`. Do not substitute raw GitHub APIs or generic push helpers.
- **Critical-risk skills require explicit review.** The `risk` field affects merge authority — `manual-review_required` means a maintainer must attest with the exact head SHA; heuristic local scores are never merge authority.
- **Respect nested `AGENTS.md` files** inside skill subtrees — they may contain additional constraints.
