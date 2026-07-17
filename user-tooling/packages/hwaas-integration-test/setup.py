# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

from hwaas_driver import Hwaas, get_collector
import atexit
import json
import signal
import sys
import traceback


def junit_exception_hook(exc_type, exc_value, tb):
    """
    The exception hook to handle script errors for junit xml correctly.

    We encountered an issue that if the test script fails, the junit xml report
    is still reporting success, as the error message is not logged with the junit
    logger. To solve this issue we add an exception hook and log the error to the
    junit log as well.
    """
    # cover if nixos test driver has a special exceptionhook
    filename = tb.tb_frame.f_code.co_filename
    name = tb.tb_frame.f_code.co_name
    line_no = tb.tb_lineno
    trace = "".join(str(line) for line in traceback.format_tb(tb))

    msg = f"File {filename} line {line_no}, in {name}\nTraceback:\n{trace}"
    log.error(msg)  # noqa: F821

    sys.__excepthook__(exc_type, exc_value, tb)


sys.excepthook = junit_exception_hook

# mypy: disable-error-code="name-defined"
hwaasConfig = json.loads(HWAAS_CONFIG)  # noqa: F821

# Initialize BenchmarkDataCollector. Afterwards a call to get_collector is sufficient.
benchmark_collector = get_collector()
benchmark_collector.add_metadata(hwaasConfig)

hwaas = Hwaas(hwaasConfig)

# Handle graceful deletion of the hwaas context on standard exit:
atexit.register(hwaas.__del__)

# Handle graceful context deletion on SIGTERM.
# This is important, as a Gitlab CI "cancel" calls SIGTERM on the process.
signal.signal(signal.SIGTERM, hwaas._on_sigterm)
