# Headroom Desktop

A macOS menu-bar app that runs the Headroom proxy and shows its dashboard,
so every `claude` / `codex` invocation is intercepted without `headroom wrap`.

## What it does

- **Menu-bar popover** — click the tray icon to open a 420×600 panel
  (no Dock icon; `ActivationPolicy::Accessory`).
- **Proxy supervisor** — locates your installed `headroom` binary and runs
  `headroom proxy --port 8787`, reaping it on quit.
- **Intercept toggle** — *On* runs `headroom init claude` + `headroom init codex`
  (durable env/config routing); *Off* surgically removes exactly what `init`
  wrote (the `headroom-init-claude` PreToolUse hook and the
  `# --- Headroom init ... ---` blocks in `~/.codex/config.toml`).
- **Live status** — polls `headroom doctor --json` every 5s.
- **Dashboard** — embeds `http://127.0.0.1:8787/dashboard` in an iframe.

## Interception model (MVP)

Config-injection only, matching `gglucass/headroom-desktop`: clients that read
`ANTHROPIC_BASE_URL` / `OPENAI_BASE_URL` (Claude Code CLI, Codex, VS Code) are
routed through the local proxy. The Anthropic **GUI desktop app** is *not*
intercepted — that would require TLS MITM with a custom root CA, which this MVP
deliberately omits.

### Known MVP limitations

- Off-toggle reverses the Claude hook + Codex config blocks only. Shell-profile
  exports written by `headroom init` (if any) are not removed.
- The proxy is not bundled; `headroom` must be installed (`pip install headroom`)
  or pointed to via the `HEADROOM_BIN` env var.

## Develop

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```
