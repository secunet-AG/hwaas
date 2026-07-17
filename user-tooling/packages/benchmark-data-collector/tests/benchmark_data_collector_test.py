# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import json
import os
from typing import TextIO

import pytest

from benchmark_data_collector import BenchmarkDataCollector

dummy: dict[str, str] = {"hello": "world"}
dummy_json: str = '{"hello": "world"}'


def test_collector_dumps_correct_data(tmp_path):
    path = os.path.join(tmp_path, "benchmarks")
    data_collector = BenchmarkDataCollector(path)
    data_collector.add_result(dummy)
    data_collector.add_result(dummy_json)

    data_collector.add_metadata({"metadata": 123})

    def check(testfile: TextIO):
        content = testfile.read()
        assert content == '{"hello": "world"}'
        assert json.loads(content)["hello"] == "world"

    with open(os.path.join(data_collector.result_path, "1.json")) as testfile:
        check(testfile)
    with open(os.path.join(data_collector.result_path, "2.json")) as testfile:
        check(testfile)
    with open(os.path.join(data_collector.metadata_path, "1.json")) as testfile:
        content = testfile.read()
        assert content == '{"metadata": 123}'


def test_collector_creates_correct_folder_structure(tmp_path):
    path = os.path.join(tmp_path, "benchmarks")
    _ = BenchmarkDataCollector(path)
    assert os.path.exists(os.path.join(path, "metadata"))
    assert os.path.exists(os.path.join(path, "results"))


def test_collector_rejects_non_json_strings(tmp_path):
    path = os.path.join(tmp_path, "benchmarks")
    data_collector = BenchmarkDataCollector(path)
    with pytest.raises(TypeError):
        data_collector.add_result('"hello": "world"')
