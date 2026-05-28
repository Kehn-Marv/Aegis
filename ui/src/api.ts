// Tiny client for the Aegis REST API. The daemon serves both the MCP
// protocol (POST /mcp) and this REST API at the same `127.0.0.1:7321`.

export type HealthState = "green" | "orange" | "red";

export interface CausalLink {
  service: string;
  signature: string;
  ts: number;
  ts_offset_secs: number;
  sample: string;
}

export interface IncidentMatch {
  incident_id: string;
  similarity: number;
  past_ts: number;
  past_root_cause_service: string;
  past_cause: string | null;
  past_fix: string | null;
  past_resolved_in_minutes: number | null;
}

export interface DecisionCard {
  kind: "decision_card";
  decision_id: string;
  ts: number;
  state: HealthState;
  chain_id: string | null;
  root_cause_service: string | null;
  headline: string;
  suggested_next_step: string;
  business_impact: string | null;
  similar_incidents: IncidentMatch[];
}

export interface Fingerprint {
  id: string;
  chain_id: string;
  ts: number;
  root_cause_service: string;
  services: string[];
  signatures: string[];
  chain: CausalLink[];
  cause: string | null;
  fix: string | null;
  resolved_at: number | null;
  resolved_in_minutes: number | null;
}

export interface GatewayStatus {
  uptime_secs: number;
  online: boolean;
  override_active: boolean;
  diagnostic_active: boolean;
  queue_depth: number;
  events_in: number;
  events_out: number;
  dedup_savings_pct: number;
  unique_signatures: number;
  state: HealthState;
  incidents_remembered: number;
  decision: DecisionCard | null;
}

export interface IncidentsResponse {
  count: number;
  incidents: Fingerprint[];
}

export interface CommandResponse {
  ok: boolean;
  message: string;
}

const BASE = "";

async function getJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const r = await fetch(`${BASE}${path}`, { signal });
  if (!r.ok) throw new Error(`${path} → ${r.status}`);
  return r.json() as Promise<T>;
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const r = await fetch(`${BASE}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  if (!r.ok) throw new Error(`${path} → ${r.status}`);
  return r.json() as Promise<T>;
}

export function fetchStatus(signal?: AbortSignal): Promise<GatewayStatus> {
  return getJson<GatewayStatus>("/api/status", signal);
}

export function fetchIncidents(
  limit = 20,
  signal?: AbortSignal,
): Promise<IncidentsResponse> {
  return getJson<IncidentsResponse>(`/api/incidents?limit=${limit}`, signal);
}

export function acknowledgeDecision(actor?: string) {
  return postJson<{ ok: boolean; message: string; decision_id: string | null; actor: string }>(
    "/api/decision/ack",
    { actor },
  );
}

export function resolveIncident(
  id: string,
  cause: string,
  fix: string,
): Promise<{ ok: boolean; message: string; incident: Fingerprint | null }> {
  return postJson("/api/incidents/" + encodeURIComponent(id) + "/resolve", {
    cause,
    fix,
  });
}

export function sendCommand(
  command: string,
  seconds?: number,
): Promise<CommandResponse> {
  return postJson<CommandResponse>("/api/command", { command, seconds });
}
