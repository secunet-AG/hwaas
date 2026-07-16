# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import pytest
from deepmerge import always_merger  # type: ignore

from data_upload import (
    json_from_directory,
    json_from_file,
)


@pytest.fixture(scope="session")
def create_json_files(tmp_path_factory):
    directory = tmp_path_factory.mktemp("json")

    def make_json_file(name, content):
        json_file = directory / name
        json_file.write_text(content)
        return json_file

    json_files = []
    json_files.append(
        make_json_file("machine1.json", '{"machine1": {"cpus": 12, "memory": 4096}}')
    )

    json_files.append(
        make_json_file("machine2.json", '{"machine2": {"cpus": 2, "memory": 1024}}')
    )

    json_files.append(
        make_json_file(
            "kernel_versions.json",
            '{"machine1": {"kernel": "1.0"}, "machine2": {"kernel": "6.0"}}',
        )
    )

    json_files.append(
        make_json_file("broken_file.json", '{"machine3": {cpus: 4, memory: 2048}}')
    )

    return directory, json_files


def test_parse_json_from_file(create_json_files):
    _, json_files = create_json_files
    assert json_from_file(json_files[0]) == {"machine1": {"cpus": 12, "memory": 4096}}
    assert json_from_file(json_files[1]) == {"machine2": {"cpus": 2, "memory": 1024}}


def test_parse_json_from_directory(create_json_files):
    expected_json = {
        "machine1": {"cpus": 12, "memory": 4096, "kernel": "1.0"},
        "machine2": {"cpus": 2, "memory": 1024, "kernel": "6.0"},
    }

    json_directory, _ = create_json_files
    json_list = json_from_directory(json_directory)
    actual_json = json_list[0]
    for j in json_list[1:]:
        actual_json = always_merger.merge(actual_json, j)

    assert expected_json == actual_json
