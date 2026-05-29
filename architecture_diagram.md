# Aegis — Architecture Diagram

Aegis sits between your services and Splunk. A self-driving **workload**
microservice produces telemetry; the **Aegis gateway** (one Rust daemon)
quiets the noise, finds what broke first, and remembers every fix; **AI**
turns that into a decision an on-call engineer can act on; and **Splunk** is
the system of record and the surface everything lands on.

```mermaid
flowchart LR
    subgraph WL["Workload microservice (Python + OpenTelemetry)"]
        APP["Simulated service fleet<br/>payment-api · checkout · orders · auth …"]
        OTEL["OTel SDK<br/>logs · metrics · traces"]
    end

    subgraph EDGE["Aegis gateway (Rust daemon — one process)"]
        ING["Ingest TCP/UDP"]
        GATE["Noise gate<br/>signature dedup"]
        CAUSAL["Causal chain<br/>what broke first"]
        MEM[("Incident memory<br/>SQLite + similarity")]
        DEC["Decision engine<br/>green / orange / red"]
        API["REST API + MCP server"]
    end

    SIDE["AI sidecar<br/>MiniLM embeddings"]
    AGENT["AegisOps agent<br/>observe → reason → act"]
    LLM["LLM brain<br/>Ollama · AI Toolkit | ai · Splunk Hosted Models"]
    UI["React control panel<br/>+ external AI agents via MCP"]
    COL["OTel Collector"]

    subgraph SPL["Splunk"]
        HEC[/"HEC"/]
        IDX[("Indexed events")]
        DASH["Dashboard Studio + CDTSM forecast"]
        SAPP["Aegis AI app<br/>alert action + | aegisreason (splunklib.ai)"]
        SMCP["Splunk MCP Server"]
    end

    APP --> OTEL
    APP -- raw logs (TCP) --> ING
    OTEL -- OTLP --> COL --> HEC
    ING --> GATE --> CAUSAL --> DEC
    GATE <-->|classify| SIDE
    DEC <--> MEM
    DEC -->|processed events| HEC
    HEC --> IDX --> DASH
    IDX --> SAPP --> LLM

    API <--> UI
    AGENT -->|REST /api/decision| API
    AGENT -->|tools/call| SMCP
    AGENT --> LLM
    AGENT -->|audit| HEC

    classDef wl fill:#eaf3ff,stroke:#0071e3,color:#0b2a4a;
    classDef edge fill:#e9f9f0,stroke:#1aa653,color:#0a3b22;
    classDef ai fill:#fff3e0,stroke:#b7791f,color:#5a3a00;
    classDef spl fill:#f1edff,stroke:#5e5ce6,color:#2a235a;
    class APP,OTEL wl;
    class ING,GATE,CAUSAL,MEM,DEC,API edge;
    class SIDE,AGENT,LLM,UI,COL ai;
    class HEC,IDX,DASH,SAPP,SMCP spl;
```

## 1. How the application interacts with Splunk

* **Processed events → HEC.** The gateway dedupes, correlates, and decides,
  then ships compact events to Splunk's **HTTP Event Collector** across eight
  sourcetypes (`aegis:raw`, `metric`, `summary`, `causal`, `decision`,
  `incident`, `silent`, `selfmetric`). Net effect: up to ~99.96% less ingest
  on a crash-looping service.
* **OpenTelemetry → Collector → Splunk.** The workload exports logs, metrics,
  and traces over **OTLP** to an OpenTelemetry Collector, which forwards them
  to Splunk via the `splunk_hec` exporter.
* **Dashboard Studio** reads the indexed events for a panel-per-pillar view;
  **CDTSM** adds forecast panels.
* **MCP, both directions.** Aegis runs its own **MCP server** (8 tools); the
  AegisOps agent is an **MCP client** of the official **Splunk MCP Server**,
  so every observational SPL call is auditable.

## 2. How AI models / agents are integrated

* **AI sidecar** (FastAPI + sentence-transformers) classifies each new log
  signature once with MiniLM embeddings; the gateway attaches the verdict.
* **AegisOps agent** runs `observe → reason → act`: it reads the gateway's
  decision card, grounds an LLM prompt in it, and may call low-risk Aegis
  tools — auditing every decision to Splunk.
* **One LLM flag, three transports:** local **Ollama**, Splunk **AI Toolkit
  `| ai`**, or **Splunk Hosted Models**.
* **Aegis AI app** (Splunkbase-shaped) adds a Custom Alert Action and the
  `| aegisreason` SPL command, both powered by `splunklib.ai.Agent`.

## 3. Data flow between services, APIs, and components

```text
workload ──raw logs (TCP 5140)──▶ Aegis gateway ──processed (HEC)──▶ Splunk
   │                                   │  ▲                            │
   └── OTLP ▶ OTel Collector ▶ HEC ────┘  │ REST + MCP (7321)          ▼
                                          ├──▶ React control panel   Dashboards
                                          ├──▶ external AI agents (MCP)
                                          └──▶ AegisOps agent ──▶ LLM ──▶ HEC audit
```

The control plane shares one in-memory `Arc<Control>`: the UI's REST poll, the
MCP `latest_decision` tool, and the agent all read the exact same live state
the data plane writes on its hot path.

---

The full deep dive — per-stage state machines, memory/perf envelope, and the
file map — lives in [`docs/architecture.md`](docs/architecture.md).
