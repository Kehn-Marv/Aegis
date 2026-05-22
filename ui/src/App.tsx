import { useCallback, useEffect, useRef, useState } from "react";
import { fetchStatus, sendCommand, type GatewayStatus } from "./api";
import { StatusBanner } from "./components/StatusBanner";
import { StatusTiles } from "./components/StatusTiles";
import { NetworkPanel } from "./components/NetworkPanel";
import { CommandConsole } from "./components/CommandConsole";
import { ActivityLog, type LogEntry } from "./components/ActivityLog";

const POLL_INTERVAL_MS = 2000;
const MAX_LOG = 200;

export default function App() {
  const [status, setStatus] = useState<GatewayStatus | null>(null);
  const [reachable, setReachable] = useState(false);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const idRef = useRef(0);

  const append = useCallback((entry: Omit<LogEntry, "id" | "ts">) => {
    setLog((prev) => {
      const id = ++idRef.current;
      const next: LogEntry = { ...entry, id, ts: Date.now() };
      return [next, ...prev].slice(0, MAX_LOG);
    });
  }, []);

  useEffect(() => {
    let cancelled = false;
    const ctrl = new AbortController();

    const tick = async () => {
      try {
        const s = await fetchStatus(ctrl.signal);
        if (cancelled) return;
        setStatus(s);
        setReachable(true);
      } catch {
        if (cancelled) return;
        setReachable(false);
      }
    };

    tick();
    const handle = setInterval(tick, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      ctrl.abort();
      clearInterval(handle);
    };
  }, []);

  const send = useCallback(
    async (command: string, seconds?: number) => {
      setBusy(true);
      const t0 = performance.now();
      const detail = seconds != null ? `${command} (${seconds}s)` : command;
      try {
        const resp = await sendCommand(command, seconds);
        append({
          direction: "out",
          label: `cmd ${command}`,
          detail: resp.message,
          ok: resp.ok,
          latency_ms: performance.now() - t0,
        });
        // Force an immediate status refresh after a command.
        try {
          const s = await fetchStatus();
          setStatus(s);
          setReachable(true);
        } catch {
          /* ignore */
        }
      } catch (err) {
        append({
          direction: "out",
          label: `cmd ${command}`,
          detail: `error: ${String(err)} (${detail})`,
          ok: false,
          latency_ms: performance.now() - t0,
        });
      } finally {
        setBusy(false);
      }
    },
    [append],
  );

  return (
    <div className="mx-auto max-w-5xl">
      <StatusBanner status={status} reachable={reachable} />
      <StatusTiles status={status} />
      <NetworkPanel
        status={status}
        onSetOnline={(o) => send(o ? "online" : "offline")}
        busy={busy}
      />
      <CommandConsole onSend={send} busy={busy} />
      <ActivityLog entries={log} />
      <footer className="px-6 pb-8 text-center text-xs text-slate-600">
        Aegis Edge Telemetry Gateway · <span className="font-mono">v0.1.0</span> ·
        REST API at <span className="font-mono">/api/status</span> · MCP at{" "}
        <span className="font-mono">/mcp</span>
      </footer>
    </div>
  );
}
