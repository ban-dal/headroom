//! Thin wrapper over `headroom doctor --json`.
//!
//! `doctor` exits non-zero when the proxy is down but still emits the JSON
//! report on stdout, so we parse stdout regardless of exit status.

use std::process::Command;

use crate::proxy::resolve_headroom_cli_candidates;

fn parse_doctor_json(stdout: &[u8], stderr: &[u8]) -> Result<serde_json::Value, String> {
    let stdout = String::from_utf8_lossy(stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        let stderr = String::from_utf8_lossy(stderr);
        let stderr = stderr.trim();
        return if stderr.is_empty() {
            Err("doctor produced no JSON on stdout".to_string())
        } else {
            Err(format!(
                "doctor produced no JSON on stdout; stderr: {stderr}"
            ))
        };
    }

    serde_json::from_str(trimmed).map_err(|e| format!("failed to parse doctor json: {e}"))
}

fn run_doctor_json(bin: &std::path::Path) -> Result<serde_json::Value, String> {
    let out = Command::new(bin)
        .args(["doctor", "--json"])
        .output()
        .map_err(|e| format!("failed to run doctor: {e}"))?;

    parse_doctor_json(&out.stdout, &out.stderr)
        .map_err(|e| format!("{e} (exit status: {})", out.status))
}

#[tauri::command]
pub fn doctor_status() -> Result<serde_json::Value, String> {
    let candidates = resolve_headroom_cli_candidates();
    if candidates.is_empty() {
        return Err("headroom binary not found".to_string());
    }

    let mut errors = Vec::new();
    for bin in candidates {
        match run_doctor_json(&bin) {
            Ok(value) => return Ok(value),
            Err(error) => errors.push(format!("{}: {error}", bin.display())),
        }
    }

    Err(format!(
        "failed to run `headroom doctor --json` with any candidate:\n{}",
        errors.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_doctor_json;

    #[test]
    fn empty_stdout_reports_missing_json_instead_of_parse_eof() {
        let error = parse_doctor_json(b"", b"sidecar failed").unwrap_err();

        assert!(error.contains("doctor produced no JSON on stdout"));
        assert!(error.contains("sidecar failed"));
        assert!(!error.contains("EOF"));
    }
}
