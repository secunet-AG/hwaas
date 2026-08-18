# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

"""
This module provides a benchmark data collector instance tailored for the use with the
HWaaS test driver. It collects metadata from the test driver  by default.

"""

from benchmark_data_collector import BenchmarkDataCollector

collector: BenchmarkDataCollector = BenchmarkDataCollector("benchmarks")


def get_collector() -> BenchmarkDataCollector:
    """
    Get the hwaas test driver benchmark data collector instance.

    Returns:
        The instance of the BenchmarkDataCollector for the HWaaS test driver.
    """
    return collector
