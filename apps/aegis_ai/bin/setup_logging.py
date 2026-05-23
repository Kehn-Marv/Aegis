# Copyright (c) 2026 Aegis Contributors
# Licensed under the MIT License (see LICENSE at repo root).
#
# Mirrors the splunk-sdk-python example apps so logs land where Splunkers
# expect them: `index="_internal" source="*aegis_ai.log"`.

import logging
import logging.handlers
import os


def setup_logging(app_name: str) -> logging.Logger:
    """Per-app rotating file logger.

    To see logs from this logger, run this SPL in Splunk:
        index="_internal" source="*aegis_ai.log"
    """
    splunk_home: str = os.environ.get("SPLUNK_HOME", os.path.join("/opt", "splunk"))
    log_path: str = os.path.join(splunk_home, "var", "log", "splunk", f"{app_name}.log")

    logger = logging.getLogger(app_name)
    if logger.handlers:
        return logger
    logger.setLevel(logging.DEBUG)

    handler = logging.handlers.RotatingFileHandler(log_path, maxBytes=1024 * 1024, backupCount=5)
    handler.setFormatter(
        logging.Formatter(f"%(asctime)s %(levelname)s [{app_name}] %(message)s")
    )
    logger.addHandler(handler)
    return logger
