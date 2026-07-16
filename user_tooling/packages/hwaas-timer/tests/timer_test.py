# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import time

from hwaas_timer import Timer


def sleep_fn() -> None:
    time.sleep(1)


def true_fn() -> bool:
    return True


def false_fn() -> bool:
    return False


def test_wait_for_simple_function():
    timer = Timer()

    timer.set_wait(sleep_fn)
    timer.set_timeout(2.0)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert not timer.is_timeout_expired()

    assert int(timer.duration()) == 1


def test_timeout_expires():
    timer = Timer()

    timer.set_wait(sleep_fn)
    timer.set_timeout(0.5)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert timer.is_timeout_expired()


def test_wait_until_success_succeeds():
    timer = Timer()

    timer.set_wait_until_success(true_fn)
    timer.set_timeout(2.0)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert not timer.is_timeout_expired()

    assert timer.duration() <= 1.0


def test_wait_until_success_fails():
    timer = Timer()

    timer.set_wait_until_success(false_fn, 0.1)
    timer.set_timeout(0.5)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert timer.is_timeout_expired()


def test_wait_until_failure_succeeds():
    timer = Timer()

    timer.set_wait_until_failure(false_fn)
    timer.set_timeout(2.0)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert not timer.is_timeout_expired()

    assert timer.duration() <= 1.0


def test_wait_until_failure_fails():
    timer = Timer()

    timer.set_wait_until_failure(true_fn, 0.1)
    timer.set_timeout(0.5)
    timer.start()

    timer.wait()
    assert not timer.is_running()
    assert timer.is_timeout_expired()
