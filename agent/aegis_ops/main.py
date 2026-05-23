"""AegisOps Agent entrypoint."""

from __future__ import annotations

import asyncio
import logging
import time
from pathlib import Path

import typer

from .actuator import Actuator
from .auditor import Auditor
from .config import AegisOpsCfg, LLMCfg
from .gateway_client import GatewayClient
from .models import Decision, DecisionRecord
from .observer import CdtsmCfg, Observer
from .policy import PolicyEngine
from .reasoner import Reasoner
from .splunk_client import SplunkClient
from .splunk_mcp_client import SplunkMcpClient
from .transports import LLMTransport, OllamaTransport, SplunkAITransport

log = logging.getLogger("aegis_ops")
app = typer.Typer(help="Autonomous AI agent for Aegis edge gateways.")


@app.command()
def run(
    config: Path = typer.Option(
        Path("configs/aegis-ops.toml"),
        "--config",
        "-c",
        help="Path to the TOML config.",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Skip all actuation and audit writes. Useful for prompt iteration.",
    ),
    once: bool = typer.Option(
        False,
        "--once",
        help="Run a single tick and exit (smoke test).",
    ),
    verbose: bool = typer.Option(False, "--verbose", "-v"),
) -> None:
    """Run the observe -> reason -> act loop."""
    logging.basicConfig(
        level=logging.DEBUG if verbose else logging.INFO,
        format="%(asctime)s %(levelname)-5s %(name)s %(message)s",
    )
    cfg = AegisOpsCfg.load(config)
    effective_dry_run = dry_run or cfg.agent.dry_run
    asyncio.run(_main_loop(cfg, dry_run=effective_dry_run, once=once))


async def _main_loop(cfg: AegisOpsCfg, *, dry_run: bool, once: bool) -> None:
    splunk: SplunkClient | None = None
    if cfg.splunk.enabled:
        mcp: SplunkMcpClient | None = None
        if cfg.splunk.mcp_enabled:
            mcp = SplunkMcpClient(
                endpoint=cfg.splunk.mcp.endpoint,
                token=cfg.splunk.token,
                verify_tls=cfg.splunk.mcp.verify_tls,
                timeout=cfg.splunk.mcp.timeout_secs,
                tool_name=cfg.splunk.mcp.tool_name_or_none,
            )
        splunk = SplunkClient(
            url=cfg.splunk.url,
            token=cfg.splunk.token,
            verify_tls=cfg.splunk.verify_tls,
            mcp=mcp,
        )

    transport = _build_transport(cfg.llm, splunk)

    observer = Observer(
        splunk=splunk,
        earliest=cfg.splunk.earliest,
        latest=cfg.splunk.latest,
        cdtsm=CdtsmCfg(
            enabled=cfg.observe.cdtsm_enabled and cfg.splunk.enabled,
            horizon_minutes=cfg.observe.cdtsm_horizon_minutes,
            history_window=cfg.observe.cdtsm_history_window,
            queue_spl=cfg.observe.queue_forecast_spl,
            queue_threshold=cfg.observe.queue_forecast_breach_threshold,
            savings_spl=cfg.observe.savings_forecast_spl,
            savings_drop_threshold_pct=cfg.observe.savings_forecast_drop_threshold_pct,
        ),
    )
    reasoner = Reasoner(transport=transport)
    policy = PolicyEngine(
        mode=cfg.policy.mode,
        min_confidence=cfg.policy.min_confidence,
        cooldown_secs=cfg.policy.cooldown_secs,
    )
    actuator = Actuator(dry_run=dry_run)
    auditor = Auditor(cfg.audit, dry_run=dry_run or not cfg.audit.enabled)
    gateways = {gw.name: GatewayClient(gw.url) for gw in cfg.gateways}

    log.info(
        "AegisOps starting: %d gateway(s), policy=%s, dry_run=%s, llm=%s, splunk=%s, audit=%s",
        len(gateways),
        cfg.policy.mode,
        dry_run,
        transport.name,
        f"on/{splunk.transport_label}" if splunk is not None else "off",
        "on" if cfg.audit.enabled and not dry_run else "off",
    )

    try:
        while True:
            tick_start = time.time()
            await asyncio.gather(
                *[
                    _tick_one(
                        name=name,
                        client=client,
                        observer=observer,
                        reasoner=reasoner,
                        policy=policy,
                        actuator=actuator,
                        auditor=auditor,
                    )
                    for name, client in gateways.items()
                ],
                return_exceptions=True,
            )
            if once:
                return
            elapsed = time.time() - tick_start
            sleep_for = max(0.5, cfg.agent.loop_interval_secs - elapsed)
            await asyncio.sleep(sleep_for)
    finally:
        for c in gateways.values():
            await c.close()
        await transport.close()
        if splunk is not None:
            await splunk.close()
        await auditor.close()


def _build_transport(cfg: LLMCfg, splunk: SplunkClient | None) -> LLMTransport:
    if cfg.transport == "ollama":
        # Enforce the Decision JSON schema at decode time so even small
        # models (e.g. qwen2.5:3b downshift) can't emit malformed JSON.
        schema = Decision.model_json_schema() if cfg.ollama.enforce_schema else None
        return OllamaTransport(
            url=cfg.ollama.url,
            model=cfg.ollama.model,
            timeout_secs=cfg.ollama.timeout_secs,
            format_schema=schema,
        )
    if cfg.transport == "splunk_ai":
        if splunk is None:
            raise RuntimeError(
                "llm.transport='splunk_ai' requires [splunk] url+token to be set. "
                "See docs/splunk-blocker.md for the current trial-environment caveat."
            )
        return SplunkAITransport(
            splunk=splunk,
            provider=cfg.splunk_ai.provider,
            model=cfg.splunk_ai.model,
        )
    if cfg.transport == "aitk_ollama":
        # Sugar for splunk_ai pre-wired to a user-defined AITK Ollama
        # connection. Same code path, more honest config.
        # See docs/aitk-ollama.md for the AITK Connection Management setup.
        if splunk is None:
            raise RuntimeError(
                "llm.transport='aitk_ollama' requires [splunk] url+token to be set "
                "(the AITK `| ai` command runs inside Splunk). "
                "See docs/aitk-ollama.md for setup."
            )
        return SplunkAITransport(
            splunk=splunk,
            provider=cfg.aitk_ollama.provider,
            model=cfg.aitk_ollama.model,
        )
    raise ValueError(f"unknown llm.transport: {cfg.transport!r}")


async def _tick_one(
    *,
    name: str,
    client: GatewayClient,
    observer: Observer,
    reasoner: Reasoner,
    policy: PolicyEngine,
    actuator: Actuator,
    auditor: Auditor,
) -> None:
    try:
        observation = await observer.observe(name, client)
    except Exception as exc:
        log.warning("observe failed for %s: %s", name, exc)
        return

    decision, prompt, raw = await reasoner.reason(observation)
    exec_mode = policy.classify(decision)
    result, err = await actuator.act(decision, exec_mode, client)

    log.info(
        "[%s] decision=%s(%s) conf=%.2f exec=%-9s | %s",
        name,
        decision.action,
        f"{decision.duration_secs}s" if decision.duration_secs else "-",
        decision.confidence,
        exec_mode,
        decision.justification[:80],
    )

    await auditor.record(
        DecisionRecord(
            ts=time.time(),
            gateway=name,
            observation=observation,
            decision=decision,
            exec_mode=exec_mode,
            actuator_result=result,
            actuator_error=err,
            prompt=prompt,
            raw_model_response=raw,
        )
    )


if __name__ == "__main__":
    app()
