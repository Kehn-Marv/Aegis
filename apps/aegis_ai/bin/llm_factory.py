# Copyright (c) 2026 Aegis Contributors
# Licensed under the MIT License (see LICENSE at repo root).
#
# Centralised construction of the splunklib.ai.OpenAIModel used by every
# Aegis AI Splunk-app entry point. Single source of truth so a Splunk
# admin only has to set environment variables once on their search head.
#
# `splunklib.ai.OpenAIModel` accepts any OpenAI-compatible chat-completions
# endpoint. By default we point at a local Ollama server running
# `gpt-oss:20b` (the same model identifier Splunk Hosted Models publishes
# for SLIM). That keeps the "same model id, different runtime" story
# intact: the moment a customer has a Splunk Cloud account with SLIM
# provisioned, they flip AEGIS_AI_LLM_BASE_URL to the SLIM endpoint and
# every entry point in this app starts using Splunk-hosted gpt-oss:20b
# with zero code change.

from __future__ import annotations

import os

from splunklib.ai import OpenAIModel

# Defaults that "just work" on a single-machine dev setup:
#   * Splunk Enterprise running natively on the host
#   * Ollama running natively on the same host
#
# On Splunk-in-Docker or Splunk-on-Linux pointing at a Windows host
# running Ollama, override AEGIS_AI_LLM_BASE_URL accordingly
# (e.g. http://host.docker.internal:11434/v1).
DEFAULT_BASE_URL = "http://127.0.0.1:11434/v1"
DEFAULT_MODEL = "gpt-oss:20b"
# Ollama ignores the api_key, but splunklib.ai.OpenAIModel still requires
# a non-empty string. A literal space is the convention used in the
# splunk-sdk-python example apps.
DEFAULT_API_KEY = " "


def build_llm_model() -> OpenAIModel:
    """Return the LLM model configured for this Splunk deployment.

    Environment variables (typically set in `$SPLUNK_HOME/etc/splunk-launch.conf`
    or `$SPLUNK_HOME/etc/apps/aegis_ai/local/passwords.conf`):
        AEGIS_AI_LLM_BASE_URL   default http://127.0.0.1:11434/v1
        AEGIS_AI_LLM_MODEL      default gpt-oss:20b
        AEGIS_AI_LLM_API_KEY    default " "  (ignored by Ollama)
    """
    base_url = os.environ.get("AEGIS_AI_LLM_BASE_URL", DEFAULT_BASE_URL)
    model = os.environ.get("AEGIS_AI_LLM_MODEL", DEFAULT_MODEL)
    # Consider Splunk's secret storage for production use:
    # https://dev.splunk.com/enterprise/docs/developapps/manageknowledge/secretstorage/secretstoragepython
    api_key = os.environ.get("AEGIS_AI_LLM_API_KEY", DEFAULT_API_KEY)

    return OpenAIModel(
        model=model,
        base_url=base_url,
        api_key=api_key,
    )
