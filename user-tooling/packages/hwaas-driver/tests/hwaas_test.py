# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import uuid

import pytest
import responses
from responses import matchers

from hwaas_driver import Hwaas

DUMMY_UUID = "2b0578ae-ad40-4340-88e2-dd1667617abd"


@pytest.fixture
def basic_hwaas_responses():
    with responses.RequestsMock(assert_all_requests_are_fired=True) as rsps:
        expected_json_rsd = {
            "machines": {
                "bmr1": {"platform": "bmrType1"},
                "bmr2": {"platform": "bmrType1"},
            }
        }
        rsps.post(
            url="http://api.hwaas.placeholder.com/v5/contexts",
            body=DUMMY_UUID,
            status=200,
            match=[matchers.json_params_matcher(expected_json_rsd)],
        )
        rsps.delete(
            url=f"http://api.hwaas.placeholder.com/v5/contexts/{DUMMY_UUID}",
            status=200,
        )
        yield rsps


def test_hwaas_context_creation_and_destruction(basic_hwaas_responses):
    hwaas_configuration = {
        "machines": {
            "bmr1": {"platform": "bmrType1"},
            "bmr2": {"platform": "bmrType1"},
        },
        "apiUrl": "http://api.hwaas.placeholder.com/v5",
    }

    hws = Hwaas(hwaas_configuration)
    assert hws.context == uuid.UUID(DUMMY_UUID)


def test_hwaas_network_creation_works(basic_hwaas_responses):
    expected_netsetup = {"bmr1": {"LAN0": {}, "LAN1": {}}, "bmr2": {"LAN0": {}}}
    basic_hwaas_responses.put(
        url=f"http://api.hwaas.placeholder.com/v5/contexts/{DUMMY_UUID}/networks/network1",
        status=200,
        match=[matchers.json_params_matcher(expected_netsetup)],
    )

    hwaas_configuration = {
        "machines": {
            "bmr1": {"platform": "bmrType1"},
            "bmr2": {"platform": "bmrType1"},
        },
        "networks": {
            "network1": {"bmr1": {"LAN0": {}, "LAN1": {}}, "bmr2": {"LAN0": {}}}
        },
        "apiUrl": "http://api.hwaas.placeholder.com/v5",
    }

    hws = Hwaas(hwaas_configuration)
    assert "network1" in hws.networks
