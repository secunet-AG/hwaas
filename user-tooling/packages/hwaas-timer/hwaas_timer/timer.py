# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

"""This module provides a simple timer class. Please refer to the documentation
of the Timer class."""

import logging
import multiprocessing
import threading
import time
from collections.abc import Callable
from datetime import datetime, timedelta, timezone
from functools import partial
from typing import Any

logger = logging.getLogger(__name__)


class Timer:
    """A simple timer that is able to measure the time it takes until a certain
    predicate is true. Intended usage:

    ```python
    # The predicate is provided as a function, in this case we have the function
    # is_server_reachable that checks whether a server is reachable via ping or
    # something similar.
    timer = Timer()
    timer.set_wait_until_success(is_server_reachable)
    # Also we don't wait longer than 30 seconds. Setting no timeout at all
    # makes the timer wait forever.
    timer.set_timeout(30.0)

    start_server()
    timer.start()

    # The timer is now running in the background.
    # You can either busy wait for the timer by waiting until timer.is_running()
    # returns false, or you can do
    timer.wait()

    if timer.is_timeout_expired():
        print("Timeout expired before server was reachable.")
    else:
        d = timer.duration()
        print(f"Server was reachable after {d} seconds.")
    ```
    """

    class Worker:
        """This class handles the creation of the timer thread and the thread
        that runs the predicate-function."""

        timeout: float | None = None
        fn: Callable[[], None] | None = None
        __thread: threading.Thread | None = None

        end: datetime | None = None
        timeout_expired: bool = False

        # Record failures from worker threads
        error: BaseException | None = None

        def __init__(self) -> None:
            pass

        @staticmethod
        def __worker_fn(worker: Any) -> None:
            real_worker: multiprocessing.Process | None = None

            try:
                # The function which this process executes. At first, it starts the
                # "real worker", and joins it.
                real_worker = multiprocessing.Process(target=worker.fn)
                real_worker.start()
                real_worker.join(worker.timeout)

                # Now this process waits, either until the real worker is done
                # working, or until the timeout expires.
                if real_worker.is_alive():
                    logger.debug("Timeout expired.")
                    real_worker.kill()
                    # Need to join here again to wait until they have actually left
                    real_worker.join()
                    worker.timeout_expired = True
                else:
                    logger.debug("No timeout expired.")
                    worker.end = datetime.now(tz=timezone.utc)

            except BaseException as error:
                worker.error = error
                logger.exception("Timer worker failed")

        def start(self) -> str | None:
            """Start the timer thread and the worker thread. Returns None if no
            error occurred, otherwise returns a string with the error."""
            if self.fn is None:
                return "Cannot start timer without a success criteria."

            self.__thread = threading.Thread(
                target=self.__worker_fn,
                args=(self,),
            )

            self.__thread.start()
            return None

        def is_alive(self) -> bool:
            """Returns whether the timer thread is alive."""
            if self.__thread is not None:
                return self.__thread.is_alive()
            return False

        def join(self) -> None:
            """Joins the timer thread."""
            if self.__thread is not None:
                self.__thread.join()
            # Propagate recorded errors
            if self.error is not None:
                raise RuntimeError("Timer worker failed") from self.error

    __worker: Worker
    __start: datetime | None = None

    def __init__(self) -> None:
        self.__worker = Timer.Worker()

    def set_timeout(self, timeout: float | timedelta) -> None:
        """Sets the timeout of the timer to the given value. If the given value
        is a float it is interpreted as seconds.

        Args:
            timeout: The desired timeout.

        Raises:
            ValueError: If the given timeout is smaller than 0.
        """
        if isinstance(timeout, timedelta):
            self.__worker.timeout = timeout.total_seconds()
        else:
            self.__worker.timeout = timeout

        if self.__worker.timeout < 0:
            logger.error("The timeout cannot be negative!")
            raise ValueError("The timeout cannot be negative!")

    def set_wait(self, fn: Callable[[], None]) -> None:
        """Instructs the timer to measure the time it takes until the given
        function returns.

        Args:
            fn: A function.
        """
        self.__worker.fn = fn

    def set_wait_until_success(
        self, fn: Callable[[], bool], sleep: float | timedelta = 0.0
    ) -> None:
        """Instructs the timer to measure the time it takes until the given
        function returns True. Retries the function if it returns False and
        sleeps for the given amount of time between retries.

        Args:
            fn: A function that returns a boolean.
            sleep: The time between invocations of `fn`.
        """
        s = sleep if isinstance(sleep, float) else sleep.total_seconds()
        self.__worker.fn = partial(Timer._wait_until, fn, True, s)

    def set_wait_until_failure(
        self, fn: Callable[[], bool], sleep: float | timedelta = 0.0
    ) -> None:
        """Instructs the timer to measure the time it takes until the given
        function returns False. Retries the function if it returns True and
        sleeps for the given amount of time between retries.

        Args:
            fn: A function that returns a boolean.
            sleep: The time between invocations of `fn`.
        """
        s = sleep if isinstance(sleep, float) else sleep.total_seconds()
        self.__worker.fn = partial(Timer._wait_until, fn, False, s)

    def start(self) -> None:
        """Starts the timer. Throws an error if no function has been set.
        Please note that the timer may run forever if no timeout is set!

        Raises:
            ValueError: If no function has been set using `set_wait`,
                `set_wait_until_success` or `set_wait_until_failure`.
        """
        self.__start = datetime.now(tz=timezone.utc)
        if error := self.__worker.start():
            logging.error(error)
            raise ValueError(error)

    def is_running(self) -> bool:
        """Returns whether the timer is currently running.

        Returns:
            True if the timer is running, False if the timer hasn't been
                started or if the timer already stopped.
        """
        return self.__worker.is_alive()

    def is_timeout_expired(self) -> bool:
        """Returns whether the timeout has expired.

        Returns:
            True if the timeout expired, False otherwise.
        """
        return self.__worker.timeout_expired

    def wait(self) -> None:
        """Blocks until the timer expired or until the given function returned
        the expected value. Does not block if the timer hasn't been started."""
        if self.__worker.is_alive():
            self.__worker.join()

    def duration(self) -> float | None:
        """Returns the duration in seconds the timer ran. Returns None if the
        timer hasn't been started, if the timer is still running or if the
        timeout expired.

        Returns:
            The time the timer ran in seconds. Returns None if the timer hasn't
                been started, if the timer is still running or if the timeout
                expired.
        """
        if (start := self.__start) and (end := self.__worker.end):
            return (end - start).total_seconds()
        return None

    @staticmethod
    def _wait_until(fn: Callable[[], bool], desired: bool, sleep: float) -> None:
        # Executes the given function until it returns the desired value. Sleeps
        # for the given amount in seconds between invocations of fn.
        while fn() != desired:
            time.sleep(sleep)
