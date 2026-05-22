// Lightweight client for the Aegis REST API. The Rust daemon serves both
// the MCP protocol (POST /mcp) and this REST API (GET /api/status,
// POST /api/command). The browser only needs the REST API.

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
}

export interface CommandResponse {
  ok: boolean;
  message: string;
}

const BASE = "";

export async function fetchStatus(signal?: AbortSignal): Promise<GatewayStatus> {
  const r = await fetch(`${BASE}/api/status`, { signal });
  if (!r.ok) throw new Error(`status ${r.status}`);
  return r.json();
}

export async function sendCommand(
  command: string,
  seconds?: number,
): Promise<CommandResponse> {
  const body: Record<string, unknown> = { command };
  if (seconds != null) body.seconds = seconds;
  const r = await fetch(`${BASE}/api/command`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok) throw new Error(`command ${r.status}`);
  return r.json();
}
