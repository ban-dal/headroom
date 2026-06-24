//! Turns interception on and off across the env-based clients.
//!
//! ON delegates to the validated `headroom init <client>` commands so we never
//! drift from the CLI's injection logic. OFF has no CLI counterpart, so we
//! surgically remove exactly what `init` wrote, keyed off the same markers the
//! CLI uses (`headroom/cli/init.py`):
//!   - Claude Code: `~/.claude/settings.json` PreToolUse hook whose command
//!     contains `headroom-init-claude`.
//!   - Codex: `~/.codex/config.toml` provider/feature blocks delimited by the
//!     `# --- Headroom init ... ---` marker lines.

use std::path::PathBuf;
use std::process::Command;

use crate::proxy::resolve_headroom;

const CLAUDE_HOOK_MARKER: &str = "headroom-init-claude";
const CODEX_PROVIDER_START: &str = "# --- Headroom init provider ---";
const CODEX_PROVIDER_END: &str = "# --- end Headroom init provider ---";
const CODEX_FEATURE_START: &str = "# --- Headroom init features ---";
const CODEX_FEATURE_END: &str = "# --- end Headroom init features ---";

fn run_init(client: &str) -> Result<String, String> {
    let bin = resolve_headroom().ok_or_else(|| "headroom binary not found".to_string())?;
    let out = Command::new(&bin)
        .args(["init", client])
        .output()
        .map_err(|e| format!("failed to run `headroom init {client}`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`headroom init {client}` failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[tauri::command]
pub fn intercept_on() -> Result<Vec<String>, String> {
    Ok(vec![run_init("claude")?, run_init("codex")?])
}

#[tauri::command]
pub fn intercept_off() -> Result<Vec<String>, String> {
    let mut report = Vec::new();
    report.push(remove_claude_hook()?);
    report.push(remove_codex_blocks()?);
    Ok(report)
}

fn claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude/settings.json"))
}

fn codex_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".codex/config.toml"))
}

/// Drop any PreToolUse hook group that contains a `headroom-init-claude` command.
fn remove_claude_hook() -> Result<String, String> {
    let path = match claude_settings_path() {
        Some(p) if p.is_file() => p,
        _ => return Ok("Claude: nothing to remove".into()),
    };

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;

    let mut removed = 0usize;
    if let Some(groups) = value
        .get_mut("hooks")
        .and_then(|h| h.get_mut("PreToolUse"))
        .and_then(|p| p.as_array_mut())
    {
        let before = groups.len();
        groups.retain(|group| {
            let has_marker = group
                .get("hooks")
                .and_then(|hs| hs.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| c.contains(CLAUDE_HOOK_MARKER))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            !has_marker
        });
        removed = before - groups.len();
    }

    if removed == 0 {
        return Ok("Claude: no headroom hook present".into());
    }

    let pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    std::fs::write(&path, pretty + "\n").map_err(|e| e.to_string())?;
    Ok(format!("Claude: removed {removed} headroom hook(s)"))
}

/// Strip the provider and feature marker blocks (inclusive) from codex config.
fn remove_codex_blocks() -> Result<String, String> {
    let path = match codex_config_path() {
        Some(p) if p.is_file() => p,
        _ => return Ok("Codex: nothing to remove".into()),
    };

    let mut text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut removed = 0usize;
    for (start, end) in [
        (CODEX_PROVIDER_START, CODEX_PROVIDER_END),
        (CODEX_FEATURE_START, CODEX_FEATURE_END),
    ] {
        while let Some(stripped) = strip_block(&text, start, end) {
            text = stripped;
            removed += 1;
        }
    }

    if removed == 0 {
        return Ok("Codex: no headroom blocks present".into());
    }

    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(format!("Codex: removed {removed} headroom block(s)"))
}

/// Remove the first `start..=end` marker block, including a trailing newline.
fn strip_block(text: &str, start: &str, end: &str) -> Option<String> {
    let s = text.find(start)?;
    let e = text[s..].find(end)? + s + end.len();
    let mut e = e;
    if text[e..].starts_with('\n') {
        e += 1;
    }
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..s]);
    out.push_str(&text[e..]);
    Some(out)
}
