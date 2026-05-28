import { useCallback, useEffect, useRef, useState } from "react";
import {
  acknowledgeDecision,
  fetchIncidents,
  fetchStatus,
  resolveIncident,
  sendCommand,
  type Fingerprint,
  type GatewayStatus,
} from "./api";
import { ActivityLog, type LogEntry } from "./components/ActivityLog";
import { DecisionCardPanel } from "./components/DecisionCardPanel";
import { IncidentMemoryPanel } from "./components/IncidentMemoryPanel";
import { StatusBanner } from "./components/StatusBanner";
import { StatusTiles } from "./components/StatusTiles";
import { ToolsPanel } from "./components/ToolsPanel";

const POLL_INTERVAL_MS = 2000;
const INCIDENT_POLL_INTERVAL_MS = 5000;
const MAX_LOG = 200;

export default function App() {
  const [status, setStatus] = useState<GatewayStatus | null>(null);
  const [incidents, setIncidents] = useState<Fingerprint[]>([]);
  const [reachable, setReachable] = useState(false);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const [showIncidents, setShowIncidents] = useState(false);
  const idRef = useRef(0);

  const append = useCallback((entry: Omit<LogEntry, "id" | "ts">) => {
    setLog((prev) => {
      const id = ++idRef.current;
      const next: LogEntry = { ...entry, id, ts: Date.now() };
      return [next, ...prev].slice(0, MAX_LOG);
    });
  }, []);

  // Poll the gateway status. This drives the decision card too — when the
  // daemon emits a new card, it lands in `status.decision` on the next tick.
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

  // Refresh the incident memory list separately so it doesn't block status
  // polling and uses a slightly longer interval.
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const r = await fetchIncidents(20);
        if (cancelled) return;
        setIncidents(r.incidents);
      } catch {
        /* leave the list as-is */
      }
    };
    tick();
    const handle = setInterval(tick, INCIDENT_POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, []);

  const send = useCallback(
    async (command: string, seconds?: number) => {
      setBusy(true);
      const t0 = performance.now();
      try {
        const resp = await sendCommand(command, seconds);
        append({
          direction: "out",
          label: `cmd ${command}`,
          detail: resp.message,
          ok: resp.ok,
          latency_ms: performance.now() - t0,
        });
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
          detail: `error: ${String(err)}`,
          ok: false,
          latency_ms: performance.now() - t0,
        });
      } finally {
        setBusy(false);
      }
    },
    [append],
  );

  const handleAcknowledge = useCallback(async () => {
    setBusy(true);
    const t0 = performance.now();
    try {
      const resp = await acknowledgeDecision();
      append({
        direction: "out",
        label: "decision ack",
        detail: resp.message + (resp.decision_id ? ` (${resp.decision_id})` : ""),
        ok: resp.ok,
        latency_ms: performance.now() - t0,
      });
    } catch (err) {
      append({
        direction: "out",
        label: "decision ack",
        detail: `error: ${String(err)}`,
        ok: false,
        latency_ms: performance.now() - t0,
      });
    } finally {
      setBusy(false);
    }
  }, [append]);

  const handleResolve = useCallback(
    async (id: string, cause: string, fix: string) => {
      setBusy(true);
      const t0 = performance.now();
      try {
        const resp = await resolveIncident(id, cause, fix);
        append({
          direction: "out",
          label: `resolve ${id.slice(0, 8)}…`,
          detail: resp.message,
          ok: resp.ok,
          latency_ms: performance.now() - t0,
        });
        try {
          const r = await fetchIncidents(20);
          setIncidents(r.incidents);
        } catch {
          /* ignore */
        }
      } catch (err) {
        append({
          direction: "out",
          label: `resolve ${id.slice(0, 8)}…`,
          detail: `error: ${String(err)}`,
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
      <DecisionCardPanel
        status={status}
        reachable={reachable}
        onAcknowledge={handleAcknowledge}
        onShowMore={() => setShowIncidents(true)}
        onDifferent={() =>
          append({
            direction: "out",
            label: "decision feedback",
            detail: "operator marked the current card as 'looks different'",
            ok: true,
            latency_ms: 0,
          })
        }
        busy={busy}
      />
      <StatusTiles status={status} />
      {(showIncidents || incidents.length > 0) && (
        <IncidentMemoryPanel
          incidents={incidents}
          onResolve={handleResolve}
          busy={busy}
        />
      )}
      <ToolsPanel
        onSend={send}
        busy={busy}
        online={status?.online ?? true}
        overrideActive={status?.override_active ?? false}
        diagnosticActive={status?.diagnostic_active ?? false}
      />
      <ActivityLog entries={log} />
      <footer className="px-6 pb-8 text-center text-xs text-slate-600">
        Aegis · <span className="font-mono">v0.2.0</span> · REST{" "}
        <span className="font-mono">/api/status</span> · MCP{" "}
        <span className="font-mono">/mcp</span>
      </footer>
    </div>
  );
}
