import { useCallback, useEffect, useState } from "react";
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

const STATUS_DOT: Record<string, string> = {
  pass: "#34c759",
  warn: "#ff9f0a",
  fail: "#ff453a",
};

function App() {
  const [doctor, setDoctor] = useState<Doctor | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<Doctor>("doctor_status");
      setDoctor(result);
      setError(null);
    } catch (e) {
      setError(String(e));
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

  return (
    <main className="app">
      <header className="header">
        <span className="title">Headroom</span>
        <span className="version">
          {doctor?.installed_version ? `v${doctor.installed_version}` : "—"}
        </span>
      </header>

      {error && <div className="error">{error}</div>}

      <section className="card">
        <div className="row">
          <span className="row-label">
            <span
              className="dot"
              style={{ background: proxyUp ? STATUS_DOT.pass : STATUS_DOT.fail }}
            />
            Proxy
          </span>
          <button
            disabled={busy !== null}
            onClick={() =>
              act(
                proxyUp ? "stop" : "start",
                proxyUp ? "stop_proxy" : "start_proxy",
              )
            }
          >
            {busy === "start" || busy === "stop"
              ? "…"
              : proxyUp
                ? "Stop"
                : "Start"}
          </button>
        </div>
        {proxyCheck && <p className="summary">{proxyCheck.summary}</p>}
      </section>

      <section className="card">
        <div className="row">
          <span className="row-label">Intercept all requests</span>
          <button
            className={intercepting ? "toggle on" : "toggle"}
            disabled={busy !== null || !proxyUp}
            onClick={() =>
              act(
                intercepting ? "off" : "on",
                intercepting ? "intercept_off" : "intercept_on",
              )
            }
          >
            {busy === "on" || busy === "off"
              ? "…"
              : intercepting
                ? "On"
                : "Off"}
          </button>
        </div>
        {clientChecks.map((c) => (
          <div key={c.name} className="client">
            <span
              className="dot"
              style={{ background: STATUS_DOT[c.status] ?? "#8e8e93" }}
            />
            <span className="client-name">{c.name}</span>
            <span className="client-summary">{c.hint ?? c.summary}</span>
          </div>
        ))}
      </section>

      <section className="dashboard">
        {proxyUp ? (
          <iframe title="dashboard" src={DASHBOARD_URL} />
        ) : (
          <div className="placeholder">Start the proxy to view the dashboard</div>
        )}
      </section>
    </main>
  );
}

export default App;
