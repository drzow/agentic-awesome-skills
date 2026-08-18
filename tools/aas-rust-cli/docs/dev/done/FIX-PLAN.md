# Fix Plan: `aas` Rust MCP Server Audit Findings

**Handoff plan** — self-contained for a fresh session. Read `AUDIT-2026-08-18.md` (same dir) for full evidence.

## Context

- Crate: `aas` v1.0.1 at `tools/aas-rust-cli` (binary `aas`, entrypoint `src/main.rs`). MCP server = stdio JSON-RPC in `src/mcp/`.
- Build/test: `cargo build`, `cargo test`, `cargo clippy --all-targets` (currently 22 bin / 27 test warnings), `cargo audit` (3 unsound-level git2 advisories, no CVEs).
- Live protocol test harness: bottom of `AUDIT-2026-08-18.md`.

## Repo constraints (from root `AGENTS.md`)

- **Mandatory multi-agent workflow** for the fix work itself (3 devs → master implementer → 3 reviewers → final reviewer; when working alone, simulate with 3 independent angles + critical self-review). Do not skip for "small" changes.
- Conventional commits (`fix:`, `refactor:`, `chore:`).
- **Never push to remote repos; never push directly to `main`.** Local commits/branches only unless explicitly told otherwise.
- PR gate for this crate: `cargo test && cargo clippy --all-targets` (aim for 0 new warnings).

## Work items (in order)

### PR 1 — MCP protocol correctness (H1, H2, H3)

Unblocks spec-compliant MCP clients. Smallest, highest-impact change.

**1a. H1 — banner off stdout** (`src/main.rs:248`)
- Change `println!("AAS MCP server starting on stdio...");` → `eprintln!(...)` (or delete).
- Verify: run the harness; first stdout line must be the `initialize` response JSON.

**1b. H2 — real notification handling** (`src/mcp/server.rs`)
- `McpMessage` is `#[serde(untagged)]`; `Request { id: Option<Value>, ... }` swallows every notification.
- Change: make `id` **required** on `Request` (`id: Value`). Untagged ordering then forces id-less messages into `Notification { method, params }`.
- In the dispatch loop: on `Notification`, match `method`:
  - `"notifications/initialized"` → ignore (no response, per spec).
  - any other notification → ignore (log to stderr if desired); **never** send a response.
- On `Request`: keep existing dispatch; unknown method → `-32601` error **with the request's id**.
- Note: JSON-RPC allows `id: null` on requests; `id: Value` accepts that — fine.
- Add unit tests in `src/mcp/server.rs` (or existing test module):
  - id-less message → no response written, no error.
  - `notifications/initialized` → no response.
  - unknown method with id → `-32601` echoing that id.
  - valid `tools/list` → response with id.
  (Test by feeding lines into the parse/dispatch function and capturing written output; refactor the loop body into a testable `handle_line(line) -> Option<String>` if needed.)
- Verify: harness output = exactly 2 JSON lines (ids 1 and 2), no third line.

**1c. H3 — version** (`src/mcp/server.rs`)
- `serverInfo` `"1.0.0"` → `env!("CARGO_PKG_VERSION")`.
- Verify: `initialize` response contains `"version":"1.0.1"`.

### PR 2 — `aas activate` (M1)

**2a. Full-skill extraction** (`src/store/bare_repo.rs`)
- Add `pub fn extract_skill(&self, id: &str, dest_dir: &Path) -> Result<()>`:
  - Validate `id` with `crate::utils::path_validation::validate_skill_id` first.
  - Walk the git tree at `skills/{id}/` recursively (mirror the tree-walk pattern in `read_blob_at_path`, ~line 290-318) and write every blob under `dest_dir/{id}/...`, creating parent dirs.
  - Error if the skill dir or its `SKILL.md` is missing.
- Unit test: temp bare repo with a multi-file skill (SKILL.md + `references/x.md`), extract, assert both files present with correct bytes.

**2b. Fix activation** (`src/cli/activate.rs:59-82`)
- Replace the symlink-to-bare-store + SKILL.md-only-copy logic:
  1. `extract_skill(id, <a materialized location>)` — e.g. `base_dir/skills/{id}` (working dir next to `store/`, add to any cleanup paths) or the cache dir.
  2. If platform supports symlinks: `symlink(materialized_path, dest)`; else copy the directory.
  3. The existing copy fallback (`get_blob_at_path`, line 73) must go — it silently drops subdirectories.
- Keep `deactivate` behavior (remove symlink/dir).
- Verify: `aas activate <id>` → `ls -la <target>/<id>` shows a real, readable `SKILL.md` (and subdirs for multi-file skills); `readlink` target exists.

### PR 3 — Hardening (L1, L2, L3)

- **L1** (`src/mcp/tools.rs`): clamp `limit` — `search_skills` `min(limit, 50)`, `filter_skills` `min(limit, 200)`, defaults unchanged.
- **L2** (`src/mcp/tools.rs`): call `validate_skill_id` at the top of `get_skill` (and anywhere a user-supplied id is used), returning a clean JSON-RPC error on failure. Matches CLI behavior in `src/cli/get_skill.rs`.
- **L3** (`src/mcp/server.rs`): cap stdin line length (e.g. 1 MiB); over-limit → JSON-RPC parse/invalid-request error, close or skip line.
- Add/extend unit tests for each.

### PR 4 — Cleanup (L4, L6, L7)

- **L4:** genericize parse-error message to the client (keep detail on stderr); consider protocol-version negotiation (echo client's version if supported, else `2025-03-26`).
- **L6:** `cargo clippy --all-targets` → 0 warnings (`&PathBuf`→`&Path`, needless `to_string`, empty format strings, `option_map_unit_fn`, etc.).
- **L7:** `src/cli/update.rs` — compute real `old_skill_count` (read previous index before rebuild); `src/cli/init.rs` — derive version from `index.json` instead of hardcoded value.
- **M2 (optional, needs product decision):** scope `AAS_SKIP_TLS_VERIFY` to explicit CLI use / louder warning. Do not remove silently.

## Final verification gate

```bash
cd tools/aas-rust-cli
cargo build
cargo test
cargo clippy --all-targets   # 0 warnings
cargo audit                  # no new advisories vs baseline (3 unsound git2)
# live protocol check (see AUDIT-2026-08-18.md):
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"audit","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | ./target/debug/aas mcp 2>/dev/null
```

Pass criteria:
1. Exactly 2 JSON lines on stdout; no banner; no response to the notification.
2. `serverInfo.version` == `1.0.1`.
3. `aas activate` produces working (non-dangling) skill entries incl. subdirectories.
4. All tests green, clippy clean.

## Out of scope

- Upstream git2 unsoundness advisories (L5) — track in dependency-update pass, not this plan.
- Any change to skill content, `data/`, `CATALOG.md`, `skills_index.json` (maintainer-owned generated artifacts).
