# Context Economy

Compact near 160k/200k window (or 250k/1M) — not a flat %. Safe: research→plan, plan→implementation, after abandoning an approach. Unsafe: mid-implementation, active debugging.

`CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=50` set (`settings.json`) — can fire early on some builds. If unexpected mid-task compaction happens, stop relying on it, compact manually at the safe points above instead.

Subagents default haiku (`CLAUDE_CODE_SUBAGENT_MODEL=haiku`). `cavecrew-builder` stays sonnet (it edits code) — don't override.

Prefer thin native CLI (`rtk`) over MCP server for same capability. Keep enabled MCP servers to what's actively used per project.
