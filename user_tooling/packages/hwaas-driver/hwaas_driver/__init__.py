# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import logging
import os

from hwaas_driver.hwaas import Hwaas, HwaasConnector, HwaasMachine, HwaasNetwork
from hwaas_driver.hwaas_benchmark_data_collector import get_collector

__all__ = ["Hwaas", "HwaasConnector", "HwaasMachine", "HwaasNetwork", "get_collector"]

logger = logging.getLogger(__name__)
# We log to stderr by default.
handler = logging.StreamHandler()
handler.setFormatter(
    logging.Formatter("%(asctime)s %(levelname)s %(funcName)s(): %(message)s")
)
logger.addHandler(handler)

try:
    loglevel = os.getenv("LOGLEVEL", "WARNING").upper()
    logger.setLevel(loglevel)
except ValueError:
    logger.setLevel(logging.WARNING)
logger.debug("Added a stderr logging handler to logger: %s", __name__)
