# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.2.2]

#### Fixed

- remote-hands: deterministic gadget function order
- Web UI: Fixed a bug preventing the "add machine from existing content" feature to correctly add machines to a context

## [5.2.1]

#### Fixed

- Serial: Fixed WebSocket streaming reliability issues under certain timing conditions
- Serial: Fixed incorrect handling of slow WebSocket clients (lag is now detected explicitly)
- Serial: Fixed inconsistencies between documented and actual API behavior (correcting the OpenAPI specification)

## [5.2.0]

### Added

- Web UI prototype

## [5.1.0] 2025-10-28

### Added

- Introduced `vistair` service for secure, low-level KVM access to physical machines via RDP/WebSocket

### Changed

- **Maintainer:** Config for max_file_size now uses the ByteSize format.

## [5.0.1] 2025-09-10

### Added

- New `GET` API endpoint `/contexts/{context_id}/machines/{machine_name}`, which
  returns the `platform` and `machine_id` of the machine used in the context
- Enabled CORS Layer for the REST API; every origin is allowed for now
- Serve the API with a self-signed cert (HTTP and HTTPS is allowed)

## [5.0.0] 2024-11-05

### Added

- Reservation
  - Machine IDs are now displayed in the inventory.
  - Optional reservation of specific machines via adding the `machine_id` key
    in the reservation request.
- Power
  - Support for multiple power interfaces. The `/power` API now either returns
    or powers all configured power interfaces. Single interfaces can be queried
    or powered by `/power/<interface_name>`.
- Serial
  - `GET` request for `/serial` API to retrieve configured serial names.
- Usb
  - USB OTG HID support for keyboard and mouse via the `/usb/keyboard` and
    `/usb/mouse` API endpoints.

### Changed

- The inventory list is now sorted by `machine_id`
- Serial interfaces are now named as configured instead of numbered from 0 onwards.
- Merged configuring and plugging USB sticks with a `PUT` request to the `/usb` API.
- Improved OpenAPI spec wording, examples and response documentation.
- Using newly added `/reset` API endpoints of all `remote-hands` services to move
  a part of the context termination logic out of the ContextAPI into `remote-hands`

### Fixed

- A bug where a DB insert conflict happened during stuck context termination.

### Removed

- `/usb/functions` and `/usb/plug` API endpoints.

## [4.1.0] 2024-06-20

### Added

- The inventory endpoint now displays the remaining seconds of each machine
  reservation.

## [4.0.1]

### Fixed

- Contexts timeout in seconds instead of milliseconds

## [4.0.0] 2024-02-05

### Changed

- All contexts are now dynamic: They must be reserved through the `POST /contexts`
  endpoint.

### Fixed

- Return a more helpful error message when a resource/endpoint is not found due
  to a trailing slash.

## [3.1.0] 2023-11-13

### Added

- The drive concept is introduced. Images are now no longer configurable as
  boot media. Drives pose a read/write-solution.
  Please consult the official HWaaS documentation to get familiar with this
  new feature.

## [3.0.2] 2023-09-18

### Fixed

- Return full response of aux devices (correct status code and pass all headers)

## [3.0.1] 2023-09-13

### Fixed

- Allow issuing requests larger than 2 MB for aux devices

## [3.0.0] 2023-08-24

### Added

- New endpoint to clear the history of the read serial: `DELETE /contexts/{ctx_id}/machines/{machine_id}/serial/{serial_interface}`

- New PATCH method for incrementally updating network state:
  `PATCH /contexts/{ctx_id}/networks/{network} <<< '{json_patch}'`

### Changed

- When specifying the content of the USB mass storage gadget,
  the attribute `imageHash` was renamed to `fileHashes`.
  Furthermore, it now expects an array of hashes instead of a single element.

- A network interface can now be assigned to a new network regardless
  of its current assignment.

- When deleting a network one now gets a `404 NOT_FOUND` if the network was not found
  instead of a `200 OK` status code.

## [2.0.0] - 2023-07-05

### Changed

- The API version has been bumped hence the version prefix in Context API URLS
  is now `v2` rather than the previously used `v1`.
- The Network API has had a substantial overhaul: In short one now sends a JSON
  consisting of all the machines and interfaces that are to be part of the network
  in a single request. See the updated Network API documentation or OpenAPI spec
  for more information.

## [1.0.0] - 2023-05-26

### Added

- API version prefix reflecting breaking changes
- New websocket endpoint for machine serial: `/contexts/{ctx_id}/machines/{machine_id}/serial/{serial_interface}/websocket`

### Changed

- The REST endpoint `/contexts/{ctx_id}/machines/{machine_id}/serial/{serial_interface}/exception`
  was removed as it was easy to overlook and did not integrate well with the
  serial websocket.
  Infrastructure exceptions are now printed directly into the serial log for
  both the REST endpoints and the websocket.
