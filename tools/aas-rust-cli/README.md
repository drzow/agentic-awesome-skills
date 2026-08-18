# AAS — Agentic Awesome Skills CLI

Minimal-context skill management for ~1,907 agentic skills.

## Overview

AAS lets agents discover and activate only the skills they need, without loading the entire catalog into context. The system uses:

- **Bare git repo** at `~/.aas/store/` — immutable objects, safe concurrent reads
- **Compact index** (~100 KB JSON) — drives all discovery without loading SKILL.md bodies
- **On-demand fetching** — full content retrieved from local git objects only when needed
- **MCP server** — agents query skills via `aas mcp` over stdio (JSON-RPC 2.0)

## Installation

```bash
# Build and install to ~/.local/bin
cd tools/aas-rust-cli
make install

# Or manually:
cargo build --release
mkdir -p ~/.local/bin
cp target/release/aas ~/.local/bin/
```

> **Note:** `~/.local/bin` is typically already in PATH. If not, add it to your shell config (`~/.bashrc`, `~/.zshrc`):
> ```bash
> export PATH="$HOME/.local/bin:$PATH"
> ```

## Quick Start

```bash
# Initialize the store (clones the skills repo as a bare git repo)
aas init --repo https://github.com/sickn33/agentic-awesome-skills.git

# Check status
aas status

# Search for skills
aas search "security scanning CI pipeline" --limit 10

# List skills by category
aas list --category security --risk safe --limit 20

# Get full content for a skill
aas get brainstorming

# Activate a skill for agent directories
aas activate brainstorming code-review --targets opencode claude-code

# Start MCP server (for agent integration)
aas mcp
```

## MCP Integration with Opencode

Add to `~/.config/opencode/opencode.json`:

```json
{
  "mcp": {
    "aas": {
      "type": "local",
      "command": ["aas", "mcp"]
    }
  }
}
```

The MCP server exposes four tools:

| Tool | Description |
|------|-------------|
| `search_skills(query, limit)` | Keyword search with relevance scoring |
| `get_skill(id, include_content)` | Fetch full SKILL.md content |
| `list_categories()` | List all categories with counts |
| `filter_skills(category?, risk?, tags[], limit)` | Structured filtering |

## Update

```bash
# Fetch latest from origin and rebuild index if changed
aas update

# Preview changes without applying
aas update --dry-run
```

## Insecure Verification

For internal registries with self-signed certificates, disable TLS and SSH host-key verification for a single invocation:

```bash
aas init --repo https://internal.example/skills.git --insecure-no-tls-verify
aas update --insecure-no-tls-verify
```

`AAS_SKIP_TLS_VERIFY` remains supported as a deprecated compatibility path and prints a warning when it enables insecure verification.

## Cache Management

```bash
aas cache info      # Show statistics
aas cache clear     # Remove all cached content
aas cache prune --older-than 30d
```

## Directory Structure

```
~/.aas/
├── index.json           # Compact catalog index (~100 KB)
├── store/               # Bare git repo (all skill data)
│   └── .git/            # Git object database only
├── cache/               # LRU disk cache for fetched content
│   ├── manifest.json    # Cache metadata
│   └── <skill-id>/SKILL.md  # Fetched content
├── meta/
│   └── state.json       # Clone SHA, version, timestamps
└── config.json          # Per-user configuration

Agent directories (managed by `aas activate`):
~/.agents/skills/        # Antigravity / opencode
~/.claude/skills/        # Claude Code
~/.cursor/skills/        # Cursor
```

## Architecture

### Storage Model
A bare git repo (`~/.aas/store/`) stores all skill data as immutable git objects. No working tree means no agent-visible files until explicitly activated. Updates are `git fetch --depth 1` + atomic index replacement.

### Index Format
A compact JSON file (~50-100 KB for 1,907 skills) with pre-tokenized search tokens. Loaded once at startup, enables instant keyword matching without network or filesystem access.

### Search Algorithm
Token-based relevance scoring:
- Query tokens matched against skill.search_tokens and skill.id
- Weighted scoring: id/name = 3.0, description = 2.0, tags = 1.0, other = 0.5
- Diminishing returns bonus for multiple matches

### Activation
Symlinks into agent directories (primary path), with copy fallback on Windows or when symlinks fail. Path validation prevents directory traversal attacks.

## License

MIT
