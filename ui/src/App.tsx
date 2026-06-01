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
import { IncidentMemoryPanel, type IncidentFocus } from "./components/IncidentMemoryPanel";
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
  const [incidentFocus, setIncidentFocus] = useState<IncidentFocus | null>(null);
  /** Track the last seen decision card ID so we can reset focus on a new card */
  const lastDecisionIdRef = useRef<string | null>(null);
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

  // When a genuinely NEW active card arrives, reset the "Show me more" filter.
  // We only act on a real incoming card (non-null, non-green) — when the backend
  // clears decision to null we stay put, same rule as the pinned-card logic.
  const cardId = status?.decision?.decision_id ?? null;
  const cardState = status?.decision?.state ?? null;
  useEffect(() => {
    if (cardId && cardState !== "green" && cardId !== lastDecisionIdRef.current) {
      setIncidentFocus(null);
    }
  }, [cardId, cardState]);

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
    <div className="mx-auto max-w-5xl flex flex-col gap-4 pb-4" style={{ paddingTop: 36 }}>
      <div className="rise px-4" style={{ animationDelay: "0ms", marginBottom: 24 }}>
        <StatusBanner status={status} reachable={reachable} />
      </div>

      <div id="decision" className="rise px-4 scroll-mt-4" style={{ animationDelay: "60ms" }}>
        <DecisionCardPanel
          status={status}
          reachable={reachable}
          onAcknowledge={handleAcknowledge}
          onShowMore={() => {
            const d = status?.decision;
            // Track this card so a new card resets the filter
            lastDecisionIdRef.current = d?.decision_id ?? null;
            setShowIncidents(true);
            setIncidentFocus({
              rootCauseService: d?.root_cause_service ?? null,
              chainId: d?.chain_id ?? null,
              similar: (d?.similar_incidents ?? []).map((m) => ({
                id: m.incident_id,
                similarity: m.similarity,
              })),
              nonce: Date.now(),
            });
            requestAnimationFrame(() =>
              document.getElementById("memory")?.scrollIntoView({ behavior: "smooth" }),
            );
          }}
          onDifferent={() =>
            append({
              direction: "out",
              label: "decision feedback",
              detail: "operator marked the current card as 'looks different'",
              ok: true,
              latency_ms: 0,
            })
          }
          onActionTaken={() => {
            // Reset incident memory filter so it reverts to normal view
            setIncidentFocus(null);
          }}
          busy={busy}
        />
      </div>

      <div className="rise px-4" style={{ animationDelay: "120ms" }}>
        <StatusTiles status={status} />
      </div>

      {(showIncidents || incidents.length > 0) && (
        <div id="memory" className="rise px-4 scroll-mt-4" style={{ animationDelay: "160ms" }}>
          <IncidentMemoryPanel
            incidents={incidents}
            onResolve={handleResolve}
            busy={busy}
            focus={incidentFocus}
          />
        </div>
      )}

      <div id="tools" className="rise px-4 scroll-mt-4" style={{ animationDelay: "200ms" }}>
        <ToolsPanel
          onSend={send}
          busy={busy}
          online={status?.online ?? true}
          overrideActive={status?.override_active ?? false}
          diagnosticActive={status?.diagnostic_active ?? false}
        />
      </div>

      <div className="rise px-4" style={{ animationDelay: "240ms" }}>
        <ActivityLog entries={log} />
      </div>

      <footer className="px-4 pt-2 text-center">
        <div className="badge badge-muted inline-flex items-center gap-2 font-mono text-[9px] uppercase tracking-[1.4px]">
          Aegis · v0.2.0 · REST /api/status · MCP /mcp
        </div>
      </footer>
    </div>
  );
}
