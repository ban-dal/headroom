import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

const PROXY_PORT = 8787;
const DASHBOARD_URL = `http://127.0.0.1:${PROXY_PORT}/dashboard`;

type Check = {
  name: string;
  status: "pass" | "warn" | "fail" | string;
  summary: string;
  hint: string | null;
};

type Doctor = {
  port: number;
  installed_version: string | null;
  exit_code: number;
  checks: Check[];
};

type DashboardMode = "inline" | "window";

const STATUS_DOT: Record<string, string> = {
  pass: "#30d158",
  warn: "#ff9f0a",
  fail: "#ff453a",
};

const CHECK_LABELS: Record<string, string> = {
  version: "Version",
  claude: "Claude",
  codex: "Codex",
  shell: "Shell env",
  savings: "Savings",
  budget: "Budget",
  deployments: "Deployments",
};

function App() {
  const [doctor, setDoctor] = useState<Doctor | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const [dashboardMode, setDashboardMode] = useState<DashboardMode>("inline");
  const refreshInFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;

    try {
      const result = await invoke<Doctor>("doctor_status");
      setDoctor(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      refreshInFlight.current = false;
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, [refresh]);

  const proxyCheck = doctor?.checks.find((c) => c.name === "proxy");
  const proxyUp = proxyCheck?.status === "pass";
  const clientChecks = doctor?.checks.filter((c) => c.name !== "proxy") ?? [];
  const intercepting =
    clientChecks.length > 0 && clientChecks.every((c) => c.status === "pass");
  const healthLabel = proxyUp ? "Healthy" : "Stopped";
  const updatedAt = useMemo(
    () =>
      new Intl.DateTimeFormat(undefined, {
        hour: "numeric",
        minute: "2-digit",
      }).format(new Date()),
    [doctor],
  );

  const act = async (label: string, cmd: string) => {
    setBusy(label);
    setError(null);
    try {
      await invoke(cmd);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const togglePinned = async () => {
    const next = !pinned;
    setPinned(next);
    setError(null);

    try {
      const confirmed = await invoke<boolean>("set_pinned", { pinned: next });
      setPinned(confirmed);
    } catch (e) {
      setPinned(!next);
      setError(String(e));
    }
  };

  const showDashboardWindow = async () => {
    setDashboardMode("window");
    setError(null);

    try {
      await invoke("open_dashboard_window");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <main className="app">
      <div className="popover-arrow" />

      <header className="topbar">
        <div>
          <div className="brand-line">
            <span className="brand">Headroom</span>
            <span className="version">
              {doctor?.installed_version ? `v${doctor.installed_version}` : "v--"}
            </span>
          </div>
          <div className="subline">
            <span
              className="dot"
              style={{ background: proxyUp ? STATUS_DOT.pass : STATUS_DOT.fail }}
            />
            {healthLabel}
            <span className="muted">Updated {updatedAt}</span>
          </div>
        </div>
        <button
          className={pinned ? "icon-button active" : "icon-button"}
          onClick={togglePinned}
          title={pinned ? "Disable always on top" : "Keep window on top"}
          aria-pressed={pinned}
        >
          Pin
        </button>
      </header>

      {error && <div className="error">{error}</div>}

      <section className="panel control-panel">
        <div className="row primary-row">
          <div>
            <span className="row-label">
              <span
                className="dot"
                style={{ background: proxyUp ? STATUS_DOT.pass : STATUS_DOT.fail }}
              />
              Proxy
            </span>
            {proxyCheck && <p className="summary">{proxyCheck.summary}</p>}
          </div>
          <button
            className="action-button"
            disabled={busy !== null}
            onClick={() =>
              act(
                proxyUp ? "stop" : "start",
                proxyUp ? "stop_proxy" : "start_proxy",
              )
            }
          >
            {busy === "start" || busy === "stop"
              ? "..."
              : proxyUp
                ? "Stop"
                : "Start"}
          </button>
        </div>

        <div className="row compact-row">
          <span className="row-label">Intercept all requests</span>
          <button
            className={intercepting ? "switch on" : "switch"}
            disabled={busy !== null || !proxyUp}
            onClick={() =>
              act(
                intercepting ? "off" : "on",
                intercepting ? "intercept_off" : "intercept_on",
              )
            }
          >
            {busy === "on" || busy === "off"
              ? "..."
              : intercepting
                ? "On"
                : "Off"}
          </button>
        </div>
      </section>

      <section className="panel checks-panel">
        <div className="section-heading">
          <span>Doctor</span>
          <span className="muted">{clientChecks.length} checks</span>
        </div>
        <div className="checks">
          {clientChecks.map((c) => (
            <div key={c.name} className="check-row">
              <span
                className="dot"
                style={{ background: STATUS_DOT[c.status] ?? "#8e8e93" }}
              />
              <span className="check-name">{CHECK_LABELS[c.name] ?? c.name}</span>
              <span className="check-summary">{c.hint ?? c.summary}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="panel dashboard-panel">
        <div className="section-heading">
          <span>Dashboard</span>
          <div className="segmented">
            <button
              className={dashboardMode === "inline" ? "selected" : ""}
              onClick={() => setDashboardMode("inline")}
            >
              Inline
            </button>
            <button
              className={dashboardMode === "window" ? "selected" : ""}
              onClick={showDashboardWindow}
            >
              Window
            </button>
          </div>
        </div>

        {dashboardMode === "inline" ? (
          <div className="dashboard-frame">
            {proxyUp ? (
              <iframe title="dashboard" src={DASHBOARD_URL} />
            ) : (
              <div className="placeholder">Start proxy to preview dashboard</div>
            )}
          </div>
        ) : (
          <div className="window-mode">
            <div>
              <strong>Dashboard opened separately</strong>
              <span>Move and resize it like a normal macOS window.</span>
            </div>
            <button
              className="action-button"
              disabled={!proxyUp}
              onClick={showDashboardWindow}
            >
              Show
            </button>
          </div>
        )}
      </section>
    </main>
  );
}

export default App;
