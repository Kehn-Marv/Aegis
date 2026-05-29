"""Aegis Workload — a self-driving telemetry microservice.

This package simulates a small e-commerce service fleet (payment-api,
checkout, orders, auth, …) that continuously emits realistic
OpenTelemetry **logs, metrics, and traces** and streams its raw log lines
straight into the Aegis edge gateway.

You start it once; it decides what to do on its own — steady traffic most
of the time, with the occasional cascade, crash-loop, latency spike, or
silent service injected automatically so Aegis always has something real
to reason about.
"""

__version__ = "0.1.0"
