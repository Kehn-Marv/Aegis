// Aegis Workload — live dashboard. Polls /api/state and paints the room.
const $ = (id) => document.getElementById(id);
const fmt = new Intl.NumberFormat("en-US");
const SIGNALS = [
  ["logs", "Logs", "structured log records", "&#9776;"],
  ["traces", "Traces", "distributed request spans", "&#8594;"],
  ["metrics", "Metrics", "counters, gauges, histograms", "&#9633;"],
  ["errors", "Error tracking", "exceptions on failed spans", "&#9888;"],
];

let last = { t: 0, reqs: 0, rps: 0 };

async function poll() {
  try {
    const r = await fetch("/api/state", { cache: "no-store" });
    if (!r.ok) throw new Error(r.status);
    render(await r.json());
  } catch (e) {
    $("health-label").textContent = "workload unreachable";
    $("health-dot").className = "dot";
  }
}

function render(s) {
  // health
  const dot = $("health-dot");
  dot.className = "dot " + s.overall_state;
  $("health-label").textContent =
    { green: "All healthy", orange: "Degraded", red: "Incident active" }[s.overall_state] || "—";

  // requests/sec from the delta between polls
  const now = Date.now();
  if (last.t) {
    const dt = (now - last.t) / 1000;
    if (dt > 0) last.rps = Math.max(0, Math.round((s.total_requests - last.reqs) / dt));
  }
  last.t = now;
  last.reqs = s.total_requests;

  renderStats(s);
  renderBanner(s);
  renderFleet(s);
  renderSignals(s);
  renderGateway(s);
  renderControls(s);
  renderFeed(s);
}

function stat(k, v, sub) {
  return `<div class="stat"><div class="k">${k}</div><div class="v">${v}</div><div class="s">${sub || ""}</div></div>`;
}

function renderStats(s) {
  const res = s.resources || {};
  $("stats").innerHTML = [
    stat("Throughput", last.rps + "<span style='font-size:14px;color:var(--text-3)'>/s</span>", fmt.format(s.total_requests) + " total"),
    stat("Error rate", (s.error_rate * 100).toFixed(2) + "%", fmt.format(s.total_errors) + " failed"),
    stat("CPU", (res.cpu ?? 0) + "%", "fleet average"),
    stat("Memory", fmt.format(Math.round(res.memory ?? 0)) + " MiB", "resident"),
    stat("In-flight", fmt.format(s.inflight ?? 0), "active requests"),
  ].join("");
}

function renderBanner(s) {
  const b = $("banner");
  const inc = s.active_incident;
  if (!inc) {
    b.className = "banner";
    return;
  }
  b.className = "banner show " + inc.severity;
  $("banner-pill").textContent = inc.severity === "red" ? "Incident" : "Degraded";
  $("banner-title").textContent = inc.title;
  $("banner-sum").textContent = inc.summary;
  $("banner-timer").textContent = `${inc.elapsed_secs.toFixed(0)}s / ${inc.duration_secs}s · recovering automatically`;
}

function renderFleet(s) {
  $("fleet-meta").textContent = `${s.services.length} services`;
  $("fleet").innerHTML = s.services
    .map((svc) => {
      const cls = { healthy: "", degraded: "degraded", down: "down", silent: "silent" }[svc.status] || "";
      const dotCls = { healthy: "green", degraded: "orange", down: "red", silent: "" }[svc.status] || "";
      return `<div class="svc ${cls}">
        <div class="top"><span class="dot ${dotCls}"></span><span class="name">${svc.name}</span></div>
        <div class="role">${svc.role}</div>
        <div class="nums">
          <div>p95<b>${svc.p95_ms}<span style="font-size:11px;color:var(--text-3)">ms</span></b></div>
          <div>errors<b>${(svc.error_rate * 100).toFixed(1)}%</b></div>
          <div>reqs<b>${fmt.format(svc.requests)}</b></div>
        </div>
      </div>`;
    })
    .join("");
}

function renderSignals(s) {
  const sig = (s.telemetry && s.telemetry.signals) || {};
  const rows = SIGNALS.map(
    ([key, label, desc, ico]) => `<div class="signal">
      <span class="ico">${ico}</span>
      <div><div class="label">${label}</div><div class="desc">${desc}</div></div>
      <span class="count">${fmt.format(sig[key] || 0)}</span>
    </div>`
  ).join("");

  const t = s.telemetry || {};
  let exportHtml;
  if (t.otlp_enabled) {
    exportHtml = `<div class="row"><span class="dot green"></span><span>Exporting to <b>${t.otlp_endpoint}</b> via ${t.otlp_protocol}</span></div>`;
  } else {
    exportHtml = `<div class="row"><span class="dot"></span><span>Produced locally. Set <b>OTEL_EXPORTER_OTLP_ENDPOINT</b> to ship to Splunk via the OTel Collector.</span></div>`;
  }
  $("signals").innerHTML = rows + `<div class="export">${exportHtml}</div>`;
}

function renderGateway(s) {
  const g = s.gateway || {};
  $("gateway").innerHTML = `
    <div class="kv"><span class="key">Connection</span><span class="val"><span class="dot ${g.connected ? "green" : "red"}" style="display:inline-block;margin-right:6px"></span>${g.connected ? "connected" : "reconnecting"}</span></div>
    <div class="kv"><span class="key">Target</span><span class="val">${g.target || "—"}</span></div>
    <div class="kv"><span class="key">Lines sent</span><span class="val">${fmt.format(g.lines_sent || 0)}</span></div>
    <div class="kv"><span class="key">Buffered</span><span class="val">${fmt.format(g.buffered || 0)}</span></div>`;
}

function renderControls(s) {
  const next = s.next_incident_in_secs;
  $("controls").innerHTML = `
    <div class="toggle" id="ap-toggle">
      <span class="switch ${s.autopilot ? "on" : ""}" id="ap-switch"></span>
      <div><div class="label" style="font-weight:550">Autopilot</div>
      <div class="desc" style="font-size:12px;color:var(--text-3)">${
        s.autopilot
          ? next != null
            ? `next incident in ~${next}s`
            : "incident in progress"
          : "paused — manual only"
      }</div></div>
    </div>
    <div class="hint">The workload runs itself. Or inject a scenario now:</div>
    <div class="triggers">${s.scenarios
      .map((sc) => `<button class="btn" data-key="${sc.key}" ${s.active_incident ? "disabled" : ""}>${sc.title}</button>`)
      .join("")}</div>`;

  $("ap-toggle").onclick = async () => {
    await fetch(`/api/autopilot/${s.autopilot ? "off" : "on"}`, { method: "POST" });
    poll();
  };
  document.querySelectorAll(".btn[data-key]").forEach((btn) => {
    btn.onclick = async () => {
      await fetch(`/api/incident/${btn.dataset.key}`, { method: "POST" });
      poll();
    };
  });
}

function renderFeed(s) {
  const t = (ts) => new Date(ts * 1000).toLocaleTimeString([], { hour12: false });
  $("feed").innerHTML =
    (s.events || [])
      .map((e) => `<div class="ev"><time>${t(e.ts)}</time><span class="tag ${e.kind}">${e.kind}</span><span class="msg">${e.message}</span></div>`)
      .join("") || `<div class="ev"><span class="msg" style="color:var(--text-3)">Waiting for activity…</span></div>`;
}

poll();
setInterval(poll, 1500);
