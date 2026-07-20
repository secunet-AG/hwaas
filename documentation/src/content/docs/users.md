# User Guide

Welcome to the user documentation for the HWaaS!

If you're here we assume that you already have access to a working HWaaS
instance to follow along. If this is not the case, please refer to our [maintainer documentation](./managing-an-instance).

There are multiple methods to interact with HWaaS, each tailored to different
needs.

**We provide the following frontends**

- A HTTP/REST API for low-level interaction: [Context API](/api)
- A CI-based solution with a higher abstraction for scripting or interactive
  use: [User Tooling](/docs/users#user-tooling)
- A low-effort interface for casual or one-time usage: [Web UI](/docs/users#web-ui)

## Context API

The Context API is a HTTP/REST API for programmatic interaction with HWaaS
instances. It's the core for all HWaaS operations and provides the basic
building blocks used by other means of interaction like the [User Tooling](/docs/users#user-tooling)
or [Web UI](/docs/users#web-ui).

Context API is fundamentally built around the concept of a _Context_, which is
an exclusive lease of one or more physical machines (also called Bare Metal
Resources, **BMR** for short) attached to HWaaS. For the duration of this
lease, the BMRs are reserved for use by the holder of the Context. Contexts
have a limited lifetime and will expire either after a preconfigured maximum
time, or when they are prematurely "returned" to HWaaS by the user.

Contexts are reserved from the _Inventory_, which is a complete list of all
registered BMRs. For every BMR, the Inventory tracks the BMRs current
reservation status and its individual properties. [Machine properties](/api#models/MachineProperties) are
used to distinguish individual BMRs and are configured by the HWaaS
administrator during BMR onboarding.

Once you have acquired a Context, you can configure the respective BMR within
the confines that the Context API provides. At the moment, the following
resources can be managed for BMRs inside a Context:

| Type           | Virtual/Physical | Description                                                |
| -------------- | ---------------- | ---------------------------------------------------------- |
| Power          | Physical         | Control a physical power switch for the BMR                |
| USB            | Virtual          | Control a USB OTG gadget for arbitrary I/O                 |
| Keyboard/Mouse | Virtual          | Control a single keyboard and mouse                        |
| Networking     | Physical         | Enable network access between machines with a smart switch |

Each of these resources is bound to the Context that creates it. This means
that when a Context expires, all created resources are released and cleaned up.
Subsequent Contexts have to recreate all desired resources from scratch, although some information, like image uploads, are cached.

The HWaaS and Context API is only concerned with managing BMRs and associated
virtual resources. From this point onward, it's your responsibility to do
something useful with the reserved resources. You can find some inspiration in
the [Examples][?] and [Cookbook][?].

## User Tooling

The User Tooling is a collection of specialized Python modules to run tests and
benchmarks on HWaaS instances. It aims to simplify the onboarding of new
projects while allowing users to create complex tests and benchmarks.

User Tooling uses the [Context API](/api) and abstracts many common tasks into
convenient Python functions. It is fully integrated with the `nix` package
manager and _NixOS_ integration testing framework. When used with `nix` for
your project, User Tooling allows fully declaring the test environment (i.e.
the _Context_ from the [Context API](/api)) and the test to run within this
environment in a single location.

You can find usage examples of this in the [Cookbook](/docs/users#cookbook).

## Web UI

The Web UI is an interactive Web UI to control the [Context API](/api) without
additional tooling. It is integrated into HWaaS deployments by default and aims
to simplify one-off interactions with the [Context API](/api).

In contrast to the [Context API](/api) and [User Tooling](/docs/users#user-tooling), the Web UI has
integrated Keyboard, Video and Mouse (**KVM**) support. This makes it the
preferred choice for interactive device exploration.

## Cookbook

Below, are a few examples for common tasks that one may want to acheive with access to a HWaaS instance.

---

### Context API

#### Booting a Machine From an Image

First, we must reserve a context with our desired machine.

```sh
curl /contexts \
  --request POST \
  --header 'Content-Type: application/json' \
  --data '{
  "machines": {
    "{your_machine_name}": {
      "machine_id": 1,
      "platform": "intel-nuc"
    }
  }
}'
```

This response will return your Context ID, which you will use for further requests.

Next, we need to upload an image to your image store. We will receive a hash back from the store which we can use to later boot our machine from.

```sh
curl /images \
  --form file='@yourimage'
  --request POST \
  --header 'Content-Type: multipart/form-data'
```

Now, we need to assign a drive to the machine under test, in order to boot.

```sh
curl '/contexts/{context_hash}/drives/{drive_name}?image_hash={image_hash}' \
  --request PUT
```

Our machine now has the required dependencies for booting.

```sh
curl '/contexts/{context_hash}/machines/{machine_name}/power' \
  --request PUT
```

### User Tooling

The above example is relatively verbose. In order to more easily interact with these machines, we recommend using the User Tooling library.

The User Tooling library is a Python library that is used in conjunction with NixOS integration tests, and is particularly useful to create CI pipelines.

First, lets create our setup and define our two test images.

```nix
{ pkgs
, yourImage # Your image for testing
,
}:
{
  apiUrl = "http://api.dev.hwaas.factory.secunet.com/v5";

  controlVM = {
    ip = "192.168.44.1";
    interface = "network1";
  };

  machines = {
    bmr1 = {
      platform = "intel-nuc-x86";
      inherit yourImage; # Inherit the image we are providing
    };
    bmr2 = {
      platform = "amd-blade-x86";
      inherit yourImage;
    };
  };
}
```

Now, we can use first configuration to define a number of different NixOS integration tests. Here, we will ping between two machines.

```nix
{ pkgs
, lib
, hwaasTestModules
, envConfig
,
}:
let
  machineOne = "bmr1";
  machineTwo = "bmr2";

  inherit (envConfig) apiUrl;

  machineOnePlatform = envConfig.machines.${machineOne}.platform;
  machineTwoPlatform = envConfig.machines.${machineTwo}.platform;

  machineOneImage = envConfig.machines.${machineOne}.image;
  machineTwoImage = envConfig.machines.${machineTwo}.image;

  controlVMIp = envConfig.controlVM.ip;
  controlVMInterface = envConfig.controlVM.interface;
in
pkgs.hwaasTest {
  name = "Multiple HWaaS machines in one network";
  inherit apiUrl;
  extraPythonPackages = _: [ pkgs.hwaasTimer ];
  nodes.controlVM = _: {
    imports = with hwaasTestModules; [
      hwaasTestVm
    ];

    networking.wireless.enable = lib.mkForce false;

    hwaas.testVm = {
      enable = true;
      networks = {
        "${controlVMInterface}" = {
          ipv4Address = {
            address = controlVMIp;
            prefixLength = 24;
          };
          dhcp = true;
          dhcpConfig = {
            ServerAddress = "${controlVMIp}/24";
            PoolSize = 3;
          };
        };
      };
    };
  };
  machines = {
    ${machineOne} = {
      image = "${machineOneImage}/your_image.iso";
      platform = machineOnePlatform;
    };
    ${machineTwo} = {
      image = "${machineTwoImage}/your_image.iso";
      platform = machineTwoPlatform;
    };
  };
  networks = {
    "${controlVMInterface}" = [
      {
        machine = machineOne;
        networkInterfaces = [ "LAN1" ];
      }
      {
        machine = machineTwo;
        networkInterfaces = [ "LAN1" ];
      }
    ];
  };
  testScript = _: ''
    from hwaas_timer import Timer
    start_all()

    controlVM.wait_for_unit("default.target")

    def ping_fn() -> bool:
      status1, _ = controlVM.execute("ping -c 1 192.168.44.2")
      status2, _ = controlVM.execute("ping -c 1 192.168.44.3")
      return ((status1 + status2) == 0)

    timer = Timer()
    timer.set_wait_until_success(ping_fn, 0.5)
    timer.set_timeout(600)
    timer.start()

    hwaas.machines["${machineOne}"].power_on()
    hwaas.machines["${machineTwo}"].power_on()

    timer.wait()

    def maybe_print_cmd(cmd: str) -> None:
      status, msg = controlVM.execute(cmd)
      if status == 0:
        print(f"Output of {cmd}:")
        print(msg)
        print()

    maybe_print_cmd("networkctl status ${controlVMInterface}")

    assert not timer.is_timeout_expired(), "Could not ping machines on network"
  '';
}
```

Depending on how your repository is setup, you can run this as a check, or integrate it into your CI pipeline.
