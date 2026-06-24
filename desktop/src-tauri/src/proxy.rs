//! Supervises a locally-installed `headroom` proxy process.
//!
//! The desktop app does not bundle the proxy (MVP); it locates the user's
//! installed `headroom` binary and spawns `headroom proxy --port <PORT>`.
//! A GUI app launched from Finder inherits a minimal PATH, so we probe the
//! usual install locations explicitly rather than relying on PATH alone.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;

pub const PROXY_PORT: u16 = 8787;

/// Holds the spawned proxy child so we can stop it and reap it on exit.
#[derive(Default)]
pub struct ProxyState(pub Mutex<Option<Child>>);

fn push_if_file(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !candidates.iter().any(|p| p == &path) {
        candidates.push(path);
    }
}

fn explicit_headroom_bin() -> Option<PathBuf> {
    std::env::var("HEADROOM_BIN").ok().map(PathBuf::from)
}

fn sidecar_headroom_bin() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("headroom-proxy"))
}

fn installed_headroom_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = dirs::home_dir() {
        push_if_file(&mut candidates, home.join(".local/bin/headroom"));
    }
    push_if_file(&mut candidates, PathBuf::from("/opt/homebrew/bin/headroom"));
    push_if_file(&mut candidates, PathBuf::from("/usr/local/bin/headroom"));

    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            push_if_file(&mut candidates, PathBuf::from(dir).join("headroom"));
        }
    }

    candidates
}

/// Resolve the `headroom` executable for long-lived proxy supervision.
/// Preference order:
///   1. `HEADROOM_BIN` override
///   2. bundled PyInstaller sidecar next to our own executable (standalone app)
///   3. common install dirs, then the inherited PATH
pub fn resolve_headroom_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = explicit_headroom_bin() {
        push_if_file(&mut candidates, explicit);
    }

    // Tauri copies `externalBin` next to the app executable, stripping the
    // target-triple suffix — so it lands as `headroom-proxy`.
    if let Some(sidecar) = sidecar_headroom_bin() {
        push_if_file(&mut candidates, sidecar);
    }

    for candidate in installed_headroom_candidates() {
        push_if_file(&mut candidates, candidate);
    }

    candidates
}

/// Resolve `headroom` for short-lived CLI commands such as `doctor` and `init`.
/// In `tauri dev`, the sidecar is a PyInstaller onefile binary in target/debug;
/// spawning it every poll is much slower than using an installed CLI.
pub fn resolve_headroom_cli_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = explicit_headroom_bin() {
        push_if_file(&mut candidates, explicit);
    }

    for candidate in installed_headroom_candidates() {
        push_if_file(&mut candidates, candidate);
    }

    if let Some(sidecar) = sidecar_headroom_bin() {
        push_if_file(&mut candidates, sidecar);
    }

    candidates
}

pub fn resolve_headroom() -> Option<PathBuf> {
    resolve_headroom_candidates().into_iter().next()
}

#[tauri::command]
pub fn proxy_path() -> Option<String> {
    resolve_headroom().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn start_proxy(state: tauri::State<ProxyState>) -> Result<u16, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    // Already running and alive? Reuse it.
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => return Ok(PROXY_PORT),
            _ => {
                *guard = None;
            }
        }
    }

    let bin = resolve_headroom().ok_or_else(|| {
        "headroom not found. Install it (pip install headroom) or set HEADROOM_BIN.".to_string()
    })?;

    let child = Command::new(&bin)
        .args(["proxy", "--port", &PROXY_PORT.to_string()])
        .spawn()
        .map_err(|e| format!("failed to start proxy: {e}"))?;

    *guard = Some(child);
    Ok(PROXY_PORT)
}

#[tauri::command]
pub fn stop_proxy(state: tauri::State<ProxyState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}
