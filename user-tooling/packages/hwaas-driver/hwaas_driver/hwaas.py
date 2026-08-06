# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

"""
This module provides functionality to interact with the HWaaS. Please refer to
the documentation of the different classes to see how they are intended to be
used.

## Logging

This module has basic logging capabilities. There are two ways to control the
amount of logging:

1. Using the `LOGLEVEL` environment variable. The different levels are `DEBUG`,
`INFO`, `WARNING`, `ERROR` and `CRITICAL`.

2. Using the `logging` Python library:

```python
import logging
hwaas_logger = logging.getLogger("hwaas_driver")

# Now you can interact with the logger, e.g. change the logging level
hwaas_logger.setLevel(logging.INFO)
```

The default logging level is `WARNING`.
"""

import hashlib
import json
import logging
import os
import random
import shutil
import sys
import tempfile
import time
import uuid
from typing import Any

import requests
from requests_toolbelt import MultipartEncoder, MultipartEncoderMonitor  # type: ignore
from tqdm import tqdm

logger = logging.getLogger(__name__)


class HwaasConnector:
    """
    Class representing the connection to the HWAAS API. Allows to do http
    requests to the HWAAS.

    HwaasConnector is initialized with the HWaaS Url,
    which acts as the root of the api endpoint.

    The root may look something like this:
    ```http://api.hwaas.placeholder.com/v5/```
    and the exact endpoint URI is passed to functions
    relative to this base Url.
    """

    hwaas_url: str

    def __init__(self, hwaas_url: str) -> None:
        self.hwaas_url = hwaas_url

    def post(self, uri: str, **kwargs) -> requests.Response:  # type: ignore
        """
        Send a HTTP POST request directly to the HWaaS API.

         The root of the HWaaS endpoint is already configured for this HwaasConnector,
         hence the specific endpoint should be provided relative to the base uri.
         Thus the uri should neither be empty nor have a leading or trailing '/'.

         Please refer to the OAS documentation for a list of possible endpoints.

         Args:
             uri: Uri of HWaaS API endpoint after the root hwaas url.
             **kwargs: Optional arguments that POST takes.
         Returns:
             Response: object
        """

        uri = f"/{uri}" if uri else ""
        url: str = f"{self.hwaas_url}{uri}"
        logger.debug("%s", url)
        return requests.post(url, **kwargs)

    def put(self, uri: str, **kwargs) -> requests.Response:  # type: ignore
        """
        Send a HTTP PUT request directly to the HWaaS API.

        The root of the HWaaS endpoint is already configured for this HwaasConnector,
        hence the specific endpoint should be provided relative to the base uri.
        Thus the uri should neither be empty nor have a leading or trailing '/'.

        Please refer to the OAS documentation for a list of possible endpoints.

        Args:
            uri: URI of HWaaS API endpoint.
            **kwargs: Optional arguments that PUT takes.
        Returns:
            Response: object
        """

        uri = f"/{uri}" if uri else ""
        url = f"{self.hwaas_url}{uri}"
        logger.debug("%s", url)
        return requests.put(url, **kwargs)

    def get(self, uri: str, **kwargs) -> requests.Response:  # type: ignore
        """
        Send a HTTP GET request directly to the HWaaS API.

        The root of the HWaaS endpoint is already configured for this HwaasConnector,
        hence the specific endpoint should be provided relative to the base uri.
        Thus the uri should neither be empty nor have a leading or trailing '/'.

        Please refer to the OAS documentation for a list of possible endpoints.

        Args:
            uri: URI of HWaaS API endpoint.
            **kwargs: Optional arguments that GET takes.
        Returns:
            Response: object
        """

        uri = f"/{uri}" if uri else ""
        url = f"{self.hwaas_url}{uri}"
        logger.debug("%s", url)
        return requests.get(url, **kwargs)

    def delete(self, uri: str, **kwargs) -> requests.Response:  # type: ignore
        """
        Send a HTTP DELETE request directly to the HWaaS API.

        The root of the HWaaS endpoint is already configured for this HwaasConnector,
        hence the specific endpoint should be provided relative to the base uri.
        Thus the uri should neither be empty nor have a leading or trailing '/'.

        Please refer to the OAS documentation for a list of possible endpoints.

        Args:
            uri: URI of HWaaS API endpoint.
            **kwargs: Optional arguments that DELETE takes.
        Returns:
            Response: object
        """

        uri = f"/{uri}" if uri else ""
        url = f"{self.hwaas_url}{uri}"
        logger.debug("%s", url)
        return requests.delete(url, **kwargs)


class HwaasMachine:
    """
    Class representing a HWAAS machine. Allows user interaction with the machine
    e.g. attaching images or power cycling.

    """

    hwaas_connector: HwaasConnector
    machine_uri: str
    name: str
    context: uuid.UUID
    drives: list[str]

    def __init__(
        self,
        context: uuid.UUID,
        name: str,
        hwaas_connector: HwaasConnector,
    ) -> None:
        self.name = name
        self.context = context
        self.machine_uri = f"contexts/{context}/machines/{name}"
        self.hwaas_connector = hwaas_connector
        self.drives = []

    def power_on(self) -> None:
        """
        Power on the HWaaS machine.

        Raises an HTTPError on failure.
        """
        logger.info("Powering on machine %s", self.name)
        self.hwaas_connector.put(f"{self.machine_uri}/power").raise_for_status()

    def power_off(self) -> None:
        """
        Power off the HWaaS machine hard.

        Raises an HTTPError on failure.
        """
        logger.info("Powering off machine %s", self.name)
        self.hwaas_connector.delete(f"{self.machine_uri}/power").raise_for_status()

    def get_machine_info(self) -> dict:
        """
        Returns the id and platform of the machine.

        Raises an HTTPError on failure

        """
        response = self.hwaas_connector.get(f"{self.machine_uri}")
        if response.status_code != 200:
            logger.warning("got status code %s", response.status_code)
            return {}

        return json.loads(response.text)

    def get_network_interfaces(self) -> list[str]:
        """
        Returns a list of HWaaS network interfaces of the machine. The network
        interfaces can be used to create HWaaS networks.

        Returns an empty list on failure.
        """
        response = self.hwaas_connector.get(f"{self.machine_uri}/network-interfaces")
        if response.status_code != 200:
            logger.warning("got status code %s", response.status_code)
            return []

        return json.loads(response.text)

    def __sha256(self, filepath: str) -> str:
        with open(filepath, "rb") as f:
            digest = hashlib.file_digest(f, "sha256")
            return digest.hexdigest()

    def usb_attach(self, usb_function: str) -> None:
        """
        Configure and plug the virtual USB stick.

        Raises an HTTPError on failure.
        """
        logger.info("Attach USB to machine %s", self.name)
        self.hwaas_connector.put(
            f"{self.machine_uri}/usb",
            data=usb_function,
            headers={"Content-Type": "application/json"},
        ).raise_for_status()

    def usb_detach(self) -> None:
        """
        De-configure and unplug the virtual USB stick.

        Raises an HTTPError on failure.
        """
        logger.info("Detach USB from machine %s", self.name)
        self.hwaas_connector.put(
            f"{self.machine_uri}/usb",
            data="[]",
            headers={"Content-Type": "application/json"},
        ).raise_for_status()

    def __create_drive(self, image_path: str) -> str:
        """
        Uploads the image provided and creates a drive entry from the given image path.
        """
        logger.info("Attach drive %s", image_path)
        image_hash = self.__sha256(f"{image_path}")
        r = self.hwaas_connector.get(f"images/{image_hash}")
        if r.status_code != 200:
            with open(image_path, "rb") as image:
                with tqdm(
                    desc="Image upload",
                    total=os.stat(image_path).st_size,
                    unit="B",
                    unit_scale=True,
                    unit_divisor=1024,
                ) as bar:
                    files = {
                        "upload_file": (
                            os.path.basename(image_path),
                            image,
                            "application/octet-stream",
                        )
                    }
                    monitor = MultipartEncoderMonitor(
                        MultipartEncoder(fields=files),
                        lambda mon: bar.update(mon.bytes_read - bar.n),
                    )
                    self.hwaas_connector.post(
                        "images",
                        data=monitor,
                        headers={"Content-Type": monitor.content_type},
                    ).raise_for_status()

        drive = str(uuid.uuid4())

        self.hwaas_connector.put(
            f"contexts/{self.context}/drives/{drive}?image_hash={image_hash}"
        ).raise_for_status()

        self.drives.append(drive)

        return drive

    def __create_usb_function(self, drives: list[str]) -> str:
        """
        Create the JSON data needed to attach a drive with possibly multiple images.
        """

        luns = [{"path": drive, "cdrom": False, "read_only": False} for drive in drives]

        usb_function = [
            {
                "type": "storage",
                "luns": luns,
            }
        ]

        return json.dumps(usb_function)

    def attach_drive(self, image_path: str) -> None:
        """
        Given a path to a bootable image, the function makes the image available
        to HWaaS and attaches the image to the HWaaS machine via USB.

        This function uses [tqdm](https://github.com/tqdm/tqdm) to show the
        progress bar, which may not work properly in some cloud logging
        consoles (e.g. log spam). Please refer to this
        [FAQ](https://github.com/tqdm/tqdm?tab=readme-ov-file#faq-and-known-issues)
        to learn how to fix this.

        Args:
            image_path: A path to the bootable image.

        Raises:
            HTTPError: An error occurred while sending a request.
        """
        self.attach_drives([image_path])

    def attach_drives(self, image_paths: list[str]) -> None:
        """
        Given the paths to one or more bootable images,
        the function makes the images available to HWaaS
        and attaches the images to the HWaaS machine via USB.

        This function uses [tqdm](https://github.com/tqdm/tqdm) to show the
        progress bar, which may not work properly in some cloud logging
        consoles (e.g. log spam). Please refer to this
        [FAQ](https://github.com/tqdm/tqdm?tab=readme-ov-file#faq-and-known-issues)
        to learn how to fix this.

        Args:
            image_paths: A List with paths to the bootable images.

        Raises:
            HTTPError: An error occurred while sending a request.
        """
        drives = [self.__create_drive(image_path) for image_path in image_paths]

        usb_function = self.__create_usb_function(drives)

        self.usb_attach(usb_function)


class HwaasNetwork:
    """
    Class representing a HWAAS network. Given a network configuration, the
    HwaasNetwork takes care of creating the network with the given machines.
    """

    network_name: str
    context: uuid.UUID
    hwaas_connector: HwaasConnector
    hwaas_dir: str

    def __init__(
        self,
        network_name: str,
        network_config: dict[str, Any],
        context: uuid.UUID,
        hwaas_connector: HwaasConnector,
    ):
        logger.info("Create HWAAS network from config: %s", network_config)
        self.network_name = network_name
        self.context = context
        self.hwaas_connector = hwaas_connector

        self.hwaas_connector.put(
            f"contexts/{self.context}/networks/{self.network_name}",
            data=json.dumps(network_config),
            headers={"Content-Type": "application/json"},
        ).raise_for_status()

        self.hwaas_dir = os.environ.get("HWAASDIR", tempfile.mkdtemp())
        os.environ["HWAASDIR"] = self.hwaas_dir

        with open(
            os.path.join(self.hwaas_dir, f"network.conf.{self.network_name}"), "w"
        ) as network_file:
            # the hwaas_url contains a http:// or https:// prefix, and we need to strip
            # it to build a valid ws url
            base_url = self.hwaas_connector.hwaas_url.removeprefix(
                "https://"
            ).removeprefix("http://")
            network_file.write(
                f"WS_PROXY_URI=ws://{base_url}/contexts/{self.context}/networks/{self.network_name}/websocket"
            )
        logger.info("Created network: %s", self.network_name)

    def __del__(self) -> None:
        logger.info("Release HWAAS network")
        try:
            shutil.rmtree(self.hwaas_dir)
        except Exception:
            pass


class Hwaas:
    """
    Class representing a single interaction with the HWAAS. Basically
    corresponds to a HWAAS context. Given a HWAAS configuration, the Hwaas class
    takes care of reserving the machines in a HWAAS context, creating any drives
    for the machines and creating the HWAAS networks.

    The Hwaas class holds all machines and networks, allowing the user to interact
    with them and taking care of cleaning everything up.

    An example HWAAS configuration in python could look like the following:

    ```json
    hwaas_configuration = {
        "machines": {
            "bmr1": {
                "image": "<path/to/image>",
                "platform": "bmrType1",
            },
            "bmr2": {
                "image": "<path/to/image>",
                "platform": "bmrType1",
            }
        },
        "networks": {
            "network1": {
                "bmr1": {
                    "LAN1": {}
                },
                "bmr2": {
                    "LAN1": {},
                    "LAN2": {}
                }
            }
        }
    }
    ```
    """

    hwaas_connector: HwaasConnector
    context: uuid.UUID

    machines: dict[str, HwaasMachine]
    """
    dictionary of strings to currently active HWaaS machines.
    """

    networks: dict[str, HwaasNetwork]
    """
    dictionary of strings to currently active HWaaS networks.
    """

    def __init__(self, hwaas_configuration: dict[str, Any]) -> None:
        try:
            self.hwaas_connector = HwaasConnector(
                hwaas_url=hwaas_configuration["apiUrl"]
            )
            self.context, self.machines = self.__create_context_and_machines(
                hwaas_configuration.get("machines", {})
            )
            self.__create_drives(hwaas_configuration.get("machines", {}))
            self.networks = self.__create_networks(
                hwaas_configuration.get("networks", {})
            )
        except Exception as e:
            logger.error("Error: %s.", e)
            if self.context:
                self.hwaas_connector.delete(f"contexts/{self.context}")
            raise e

    def __del__(self) -> None:
        logger.info("Shutting down, deleting context: %s", self.context)
        self.hwaas_connector.delete(f"contexts/{self.context}")

    def _on_sigterm(self, signum: Any, frame: Any) -> Any:
        """
        Handle SIGTERM by calling the destructor on ourselves.
        This function is registered as a signal handler in setup.py to
        handle SIGTERM cleanly.
        """
        self.__del__()
        sys.exit(0)

    def __create_context_and_machines(
        self, machine_config: dict[str, Any]
    ) -> tuple[uuid.UUID, dict[str, HwaasMachine]]:
        resource_descriptor = {
            "machines": {
                f"{name}": {
                    "platform": f"{config['platform']}",
                    **(
                        {"machine_id": config["machine_id"]}
                        if "machine_id" in config
                        else {}
                    ),
                }
                for name, config in machine_config.items()
            }
        }

        response = self.hwaas_connector.post("contexts", json=resource_descriptor)
        # polling with full jitter and exponential backoff, until a machine is free
        # jitter here, because we could have a number of pipelines looking for a machine
        backoff = 0.2  # start with 200ms sleep
        max_sleep_time = 30.0

        while response.status_code != 200:
            delay = random.uniform(0, backoff)
            logger.info("Could not get context: %s.", response.text)
            logger.info("Trying again in %.1fs.", delay)
            time.sleep(delay)
            backoff = min(backoff * 2, max_sleep_time)
            response = self.hwaas_connector.post("contexts", json=resource_descriptor)

        context = uuid.UUID(response.text)
        logger.info("Got Context: %s", context)

        hwaas_machines = {
            name: HwaasMachine(context, name, self.hwaas_connector)
            for name in machine_config.keys()
        }

        return context, hwaas_machines

    def __create_drives(self, machine_config: dict[str, Any]) -> None:
        for name, config in machine_config.items():
            if "image" in config:
                images = [config["image"]]

                if "additionalImages" in config:
                    images.extend(config["additionalImages"])

                self.machines[name].attach_drives(images)

    def __create_networks(
        self, network_config: dict[str, Any]
    ) -> dict[str, HwaasNetwork]:
        hwaas_networks = {
            name: HwaasNetwork(name, config, self.context, self.hwaas_connector)
            for name, config in network_config.items()
        }

        return hwaas_networks
