// Aegis Workload — live dashboard. Polls /api/state and paints the room.
const $ = (id) => document.getElementById(id);
const fmt = new Intl.NumberFormat("en-US");
const SIGNALS = [
  ["logs", "Logs", "structured log records",
    `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><line x1="3" y1="4" x2="13" y2="4"/><line x1="3" y1="8" x2="11" y2="8"/><line x1="3" y1="12" x2="9" y2="12"/></svg>`],
  ["traces", "Traces", "distributed request spans",
    `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2 8h4l2-4 2 8 2-4h2"/></svg>`],
  ["metrics", "Metrics", "counters, gauges, histograms",
    `<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="9" width="3" height="6" rx="0.5"/><rect x="5.5" y="5" width="3" height="10" rx="0.5"/><rect x="10" y="2" width="3" height="13" rx="0.5"/></svg>`],
  ["errors", "Error tracking", "exceptions on failed spans",
    `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M8 1L14.5 13H1.5L8 1z"/><line x1="8" y1="6" x2="8" y2="9"/><circle cx="8" cy="11" r="0.5" fill="currentColor"/></svg>`],
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
    stat("Throughput", last.rps + "<span style='font-size:12px;color:var(--ink-muted)'>/s</span>", fmt.format(s.total_requests) + " total"),
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
  $("banner-title").textContent = inc.title;
  $("banner-sum").textContent = inc.summary;
  $("banner-timer").textContent = `${inc.elapsed_secs.toFixed(0)}s / ${inc.duration_secs}s · recovering automatically`;
}

let fleetBuilt = false;
let eqAnimId = null;
let svcEnergy = []; // per-service energy level for bar heights

function renderFleet(s) {
  $("fleet-meta").textContent = `${s.services.length} services`;

  // Update energy levels from live metrics every poll
  svcEnergy = s.services.map(svc => {
    const activity = Math.min(1, svc.requests / 2000);
    const stress = Math.min(1, svc.p95_ms / 150);
    return 0.3 + activity * 0.5 + stress * 0.2;
  });

  if (!fleetBuilt || $("fleet").children.length === 0) {
    // Build DOM once
    const cards = s.services.map((svc, si) => {
      const cls = { healthy: "", degraded: "degraded", down: "down", silent: "silent" }[svc.status] || "";
      let bars = "";
      for (let b = 0; b < 12; b++) {
        bars += `<span class="eq-bar"></span>`;
      }
      return `<div class="svc ${cls}" data-svc-idx="${si}" data-status="${svc.status}">
        <div class="top"><span class="name">${svc.name}</span></div>
        <div class="role">${svc.role}</div>
        <div class="eq-meter">${bars}</div>
        <div class="nums">
          <div>p95<b class="v-p95">${svc.p95_ms}<span style="font-size:9px;color:rgba(255,250,218,0.35)">ms</span></b></div>
          <div>errors<b class="v-err">${(svc.error_rate * 100).toFixed(1)}%</b></div>
          <div>reqs<b class="v-reqs">${fmt.format(svc.requests)}</b></div>
        </div>
      </div>`;
    });
    const COLS = 3;
    const rows = Math.ceil(cards.length / COLS);
    // Explicit rows: card rows (auto) interleaved with thin separator rows so a
    // single vertical line can span 1 / -1 across the whole grid and intersect
    // each horizontal line, instead of being chopped up per row.
    $("fleet").style.gridTemplateRows = Array.from(
      { length: rows * 2 - 1 },
      (_, r) => (r % 2 === 0 ? "auto" : "max-content"),
    ).join(" ");

    const out = cards.slice();
    // Two continuous vertical dividers spanning every row.
    out.push(`<div class="fleet-vsep" style="grid-column:2;grid-row:1/-1"></div>`);
    out.push(`<div class="fleet-vsep" style="grid-column:4;grid-row:1/-1"></div>`);
    // Full-width horizontal dividers, one in each gap between card rows.
    for (let r = 1; r < rows; r++) {
      out.push(`<div class="fleet-sep" style="grid-row:${r * 2};grid-column:1/-1"></div>`);
    }
    $("fleet").innerHTML = out.join("");
    fleetBuilt = true;
    startEqLoop();
  } else {
    // DOM already exists — just update text values in-place
    const svcEls = $("fleet").querySelectorAll(".svc");
    s.services.forEach((svc, si) => {
      const el = svcEls[si];
      if (!el) return;
      const cls = { healthy: "", degraded: "degraded", down: "down", silent: "silent" }[svc.status] || "";
      el.className = "svc " + cls;
      el.dataset.status = svc.status;
      el.querySelector(".v-p95").innerHTML = `${svc.p95_ms}<span style="font-size:9px;color:rgba(255,250,218,0.35)">ms</span>`;
      el.querySelector(".v-err").textContent = `${(svc.error_rate * 100).toFixed(1)}%`;
      el.querySelector(".v-reqs").textContent = fmt.format(svc.requests);
    });
  }
}

/* ---- Persistent equalizer animation — never stops --------------------- */
function startEqLoop() {
  if (eqAnimId) return; // already running
  const allBars = []; // { el, svcIdx, nextTick }
  document.querySelectorAll(".svc").forEach(svcEl => {
    const si = parseInt(svcEl.dataset.svcIdx, 10);
    svcEl.querySelectorAll(".eq-bar").forEach(bar => {
      allBars.push({
        el: bar,
        svcIdx: si,
        nextTick: performance.now() + Math.random() * 400,
      });
    });
  });

  function frame(now) {
    for (const b of allBars) {
      if (now < b.nextTick) continue;
      const svcEl = b.el.closest(".svc");
      if (svcEl && svcEl.dataset.status === "silent") {
        b.el.style.height = "8%";
        b.el.style.opacity = "0.2";
        b.nextTick = now + 1000;
        continue;
      }
      b.el.style.opacity = "";
      const energy = svcEnergy[b.svcIdx] || 0.5;
      const rand = Math.random();
      const spike = Math.random() < 0.15 ? 0.3 : 0;
      const h = Math.max(10, Math.min(95, Math.round((rand * energy + spike) * 100)));
      b.el.style.height = h + "%";
      // Next change: random 120-500ms — each bar on its own schedule
      b.nextTick = now + 120 + Math.random() * 380;
    }
    eqAnimId = requestAnimationFrame(frame);
  }
  eqAnimId = requestAnimationFrame(frame);
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
    exportHtml = `<div class="row"><span>Set <b>OTEL_EXPORTER_OTLP_ENDPOINT</b> to export to Splunk</span></div>`;
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

// Only rebuild the controls when their structure actually changes — otherwise
// the toggle + buttons would be re-created every 1.5s, dropping keyboard focus
// and visibly flickering. The live countdown text is updated separately below.
let controlSig = null;

function apDesc(s) {
  const next = s.next_incident_in_secs;
  if (!s.autopilot) return "paused — manual only";
  return next != null ? `next incident in ~${next}s` : "incident in progress";
}

function renderControls(s) {
  const sig = `${s.autopilot}|${!!s.active_incident}|${s.scenarios.map((x) => x.key).join(",")}`;

  if (sig !== controlSig) {
    controlSig = sig;
    $("controls").innerHTML = `
      <div class="toggle" id="ap-toggle" role="switch" tabindex="0" aria-checked="${s.autopilot}" aria-label="Toggle autopilot">
        <span class="switch ${s.autopilot ? "on" : ""}" id="ap-switch"></span>
        <div><div class="label" style="font-weight:800;color:var(--ink);font-size:12px">${s.autopilot ? "On" : "Off"}</div>
        <div class="desc" id="ap-desc" style="font-size:10px;color:var(--ink-muted)">${apDesc(s)}</div></div>
      </div>
      <div class="hint">The workload runs itself. Or inject a scenario now:</div>
      <div class="triggers">${s.scenarios
        .map((sc) => `<button class="btn" data-key="${sc.key}" ${s.active_incident ? "disabled" : ""}>${sc.title}</button>`)
        .join("")}</div>`;

    const toggleAutopilot = async () => {
      await fetch(`/api/autopilot/${s.autopilot ? "off" : "on"}`, { method: "POST" });
      poll();
    };
    const toggle = $("ap-toggle");
    toggle.onclick = toggleAutopilot;
    toggle.onkeydown = (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        toggleAutopilot();
      }
    };
    document.querySelectorAll(".btn[data-key]").forEach((btn) => {
      btn.onclick = async () => {
        await fetch(`/api/incident/${btn.dataset.key}`, { method: "POST" });
        poll();
      };
    });
  }

  // Live countdown — refreshed every tick without rebuilding the controls.
  const desc = $("ap-desc");
  if (desc) desc.textContent = apDesc(s);
}

function renderFeed(s) {
  const feed = $("feed");
  const prevTop = feed.scrollTop;
  feed.innerHTML =
    (s.events || [])
      .map((e) => `<div class="ev"><time>${feedTime(e.ts)}</time><span class="tag ${e.kind}">${e.kind}</span><span class="msg">${e.message}</span></div>`)
      .join("") || `<div class="ev"><span class="msg" style="color:rgba(255,250,218,0.3)">Waiting for activity…</span></div>`;
  // Keep the reader anchored where they were instead of snapping to the top.
  feed.scrollTop = prevTop;
}

function feedTime(ts) {
  return new Date(ts * 1000).toLocaleTimeString([], { hour12: false });
}

poll();
setInterval(poll, 1500);
