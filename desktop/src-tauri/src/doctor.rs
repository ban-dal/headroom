//! Thin wrapper over `headroom doctor --json`.
//!
//! `doctor` exits non-zero when the proxy is down but still emits the JSON
//! report on stdout, so we parse stdout regardless of exit status.

use std::process::Command;

use crate::proxy::resolve_headroom;

#[tauri::command]
pub fn doctor_status() -> Result<serde_json::Value, String> {
    let bin = resolve_headroom()
        .ok_or_else(|| "headroom binary not found".to_string())?;

    let out = Command::new(&bin)
        .args(["doctor", "--json"])
        .output()
        .map_err(|e| format!("failed to run doctor: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim())
        .map_err(|e| format!("failed to parse doctor json: {e}"))
}
