# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

"""
This module provides functionality to collect benchmark data from tests.
Please refer to the documentation of the main class to see how it is
intended to be used.

## Logging

This module has basic logging capabilities. There are two ways to control the
amount of logging:

1. Using the `LOGLEVEL` environment variable. The different levels are `DEBUG`,
`INFO`, `WARNING`, `ERROR` and `CRITICAL`.

2. Using the `logging` Python library:

```python
import logging
benchmark_data_collector_logger = logging.getLogger("benchmark_data_collector")

# Now you can interact with the logger, e.g. change the logging level
benchmark_data_collector_logger.setLevel(logging.INFO)
```

The default logging level is `WARNING`.
"""

import json
import logging
import os
from typing import Any

logger = logging.getLogger(__name__)


class BenchmarkDataCollector:
    """
    The benchmark data collector.
    It stores given data to the desired store path with the following directory
    structure:

    - benchmark_collector_root_path
        - metadata
            - 1.json
            - 2.json
        - results
            - 1.json
            - 2.json
            - 3.json

    Any data that is stored using the `add_metadata` method is stored in the
    metadata directory.

    Any data that is stored using the `add_result` method is stored in the result
    directory and represents a single benchmark result which can be merged with
    the metadata and uploaded to the analytics tooling afterwards.

    Args:
        path: The path to the root directory of the benchmark data collector

    Returns:
        Instance of BenchmarkDataCollector
    """

    metadata_path: str
    result_path: str
    metadata_counter: int
    result_counter: int

    def __init__(self, path: str) -> None:
        self.result_path = os.path.join(os.getcwd(), path, "results")
        self.metadata_path = os.path.join(os.getcwd(), path, "metadata")
        if os.path.exists(os.path.join(os.getcwd(), path)):
            logger.warn(
                "The benchmark data store path is already existent! "
                "Previous outputs might be overwritten.",
            )

        logger.info("Creating benchmark data result path %s.", self.result_path)
        os.makedirs(self.result_path, exist_ok=True)
        self.result_counter = 1

        logger.info("Creating benchmark metadata path %s.", self.metadata_path)
        os.makedirs(self.metadata_path, exist_ok=True)
        self.metadata_counter = 1

    @staticmethod
    def _is_json(data: str) -> bool:
        try:
            json.loads(data)
        except ValueError:
            return False
        return True

    def _store_data(self, data: str | dict[str, Any], filepath: str) -> None:
        actual_data: str = ""
        if isinstance(data, str) and BenchmarkDataCollector._is_json(data):
            actual_data = data
        elif isinstance(data, dict):
            actual_data = json.dumps(data)
            logger.debug("Converting data to json \n %s", actual_data)
        else:
            raise TypeError(
                "The passed data input is neither "
                "a Python dict object nor JSON formatted text!"
            )

        logger.debug(f"Writing data to {filepath}")
        with open(filepath, "w") as f:
            f.write(actual_data)

    def add_result(self, data: str | dict[str, Any]) -> None:
        """
        Dumps results to the benchmark data result store.

        Args:
            data: The benchmark data that should be stored.
                  It could be either a JSON formatted string or a Python dict.

        Returns:
            None

        Raises:
            TypeError: Raised when the data is neither a JSON formatted string nor
                       a Python dict.
        """
        filepath = os.path.join(self.result_path, f"{self.result_counter}.json")
        self.result_counter += 1
        self._store_data(data, filepath)

    def add_metadata(self, data: str | dict[str, Any]) -> None:
        """
        Dumps metadata to the benchmark data metadata store.

        Args:
            data: The metadata that should be stored.
                  It can be either a JSON formatted string or a Python dict.

        Returns:
            None

        Raises:
            TypeError: Raised when the data is neither a JSON formatted string nor
                       a Python dict
        """
        filepath = os.path.join(self.metadata_path, f"{self.metadata_counter}.json")
        self.metadata_counter += 1
        self._store_data(data, filepath)
