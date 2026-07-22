// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use axum::async_trait;
use hidg::{Button, Class, Device, Key, Keyboard, Modifiers, Mouse};
use remote_serial::{api::HasSerial, serial::SerialState};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, instrument};
use usb_gadget::{default_udc, function::hid::Hid, RegGadget, Udc};

use crate::usb_config::{UsbConfig, UsbFunction, UsbFunctionConfig, UsbFunctionInfo};

/// Inner AppState, locked together
pub struct InnerState {
    udc: Udc,
    gadget: Option<RegGadget>,
    functions: Option<Vec<UsbFunction>>,
    serials: HashMap<String, SerialState>,
}

/// This trait is solely needed to define "empty" State for OpenAPI generation
/// in src/bin/openapi-generator.rs, since `default_udc()` fails there
#[async_trait]
pub trait UsbConfigurable: Send + Sync + Clone + HasSerial + 'static {
    async fn configure(&self, usb_config: UsbConfig) -> Result<(), std::io::Error>;
    async fn deconfigure(&self) -> Result<(), std::io::Error>;
    async fn get_function_infos(&self) -> Option<Vec<UsbFunctionInfo>>;
    async fn use_keyboard(
        &self,
        modifier: Option<Modifiers>,
        keys: Vec<Key>,
        press: bool,
        release: bool,
    ) -> Result<(), std::io::Error>;
    async fn use_mouse(
        &self,
        buttons: Vec<Button>,
        pointer: (u16, u16),
        wheel: i8,
    ) -> Result<(), std::io::Error>;
}

#[derive(Clone)]
/// AppState: Contains the path to the images and the `InnerState`.
/// The `inner` state is encapsulated in a Mutex, since we want to make sure
/// that access/changes to the state is done one by one for multiple parallel
/// requests.
pub struct AppState {
    pub images_path: Arc<String>,
    inner: Arc<Mutex<InnerState>>,
}

impl AppState {
    /// Initialization with given path.
    pub fn new(images_path: String) -> Self {
        AppState {
            images_path: Arc::new(images_path),
            inner: Arc::new(Mutex::new(InnerState {
                udc: default_udc().expect("default_udc"),
                gadget: None,
                functions: None,
                serials: HashMap::new(),
            })),
        }
    }
}

#[async_trait]
impl UsbConfigurable for AppState {
    #[instrument(skip(self))]
    /// Configure USB OTG with the given config
    async fn configure(&self, mut usb_config: UsbConfig) -> Result<(), std::io::Error> {
        // If functions are currently active, deconfigure them
        if self.inner.lock().await.functions.is_some() {
            let _ = self.deconfigure().await;
            debug!("deconfigured");
        }

        // Map the configured USB drive filenames to absolute Paths
        usb_config.functions = usb_config
            .functions
            .into_iter()
            .map(|function| match function {
                UsbFunctionConfig::Storage { mut luns } => {
                    for lun in &mut luns {
                        lun.path = PathBuf::from(self.images_path.to_string())
                            .join(lun.path.clone())
                            .to_string_lossy()
                            .to_string();
                        debug!(lun.path, "built drive path");
                    }
                    UsbFunctionConfig::Storage { luns }
                }
                f => f,
            })
            .collect();
        debug!("prepared drive paths");

        // Verify configuration
        usb_config.clone().verify()?;
        debug!("USB Config verified");

        // Prepare configuration
        let (gadget, mut functions) = usb_config.make_gadget()?;
        debug!("prepared gadget");

        // Apply configuration
        let mut inner = self.inner.lock().await;
        let reg = gadget.bind(&inner.udc)?;

        debug!("gadget binding done");

        // Start workers for serial ports
        for function in &mut functions {
            match function {
                #[cfg(feature = "usb-serial")]
                UsbFunction::Serial { serial, serial_id } => {
                    let tty_path = match serial.tty() {
                        Ok(tty_path) => tty_path,
                        Err(e) => {
                            error!(error = %e, "cannot obtain tty path for USB OTG serial peripheral");
                            continue;
                        }
                    };
                    let id = tty_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("tty");
                    let tty_path_str = match tty_path.to_str() {
                        Some(tty_path_str) => tty_path_str.to_string(),
                        None => {
                            error!("invalid tty path {tty_path:?}");
                            continue;
                        }
                    };
                    let serial = match remote_serial::serial::tty::new_with_path(tty_path_str) {
                        Ok(serial) => serial,
                        Err(e) => {
                            error!(error = %e, "cannot start serial for tty {tty_path:?}");
                            continue;
                        }
                    };
                    let state = SerialState::new(serial);
                    if let Some(s_id) = serial_id {
                        inner.serials.insert(s_id.to_string(), state);
                    } else {
                        inner.serials.insert(id.to_string(), state);
                        function.set_serial_id(id.to_string());
                    }
                }
                _ => { /* nothing to prepare */ }
            }
        }
        // Keep functions metadata
        inner.functions = Some(functions);
        // Hold onto gadget so we can specifically drop this gadget
        inner.gadget = Some(reg);

        Ok(())
    }

    /// Deconfigure the current USB OTG state
    async fn deconfigure(&self) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().await;

        inner.functions = None;

        // Drop the various serial workers
        for serial in inner.serials.values() {
            serial.clear_buffer();
        }

        inner.serials = HashMap::new();

        if let Some(reg) = inner.gadget.take() {
            reg.remove().map_err(|e| {
                error!(e = %e, "error removing OTG gadget");
                e
            })?;
        }

        Ok(())
    }

    /// Return function defintion from internal state.
    async fn get_function_infos(&self) -> Option<Vec<UsbFunctionInfo>> {
        self.inner.lock().await.functions.as_ref().map(|functions| {
            functions
                .iter()
                .map(|function| function.get_info())
                .collect()
        })
    }

    /// Writes to keyboard if configured.
    async fn use_keyboard(
        &self,
        modifier: Option<Modifiers>,
        keys: Vec<Key>,
        press: bool,
        release: bool,
    ) -> Result<(), std::io::Error> {
        if let Some(functions) = self.inner.lock().await.functions.as_ref() {
            for function in functions {
                if let UsbFunction::Keyboard(hid) = function {
                    write_to_keyboard(hid, modifier, keys.clone(), press, release)?;
                    break; // there can always only be 1 keyboard per design
                }
            }
        };
        Ok(())
    }

    /// Uses the mouse if configured.
    async fn use_mouse(
        &self,
        buttons: Vec<Button>,
        pointer: (u16, u16),
        wheel: i8,
    ) -> Result<(), std::io::Error> {
        if let Some(functions) = self.inner.lock().await.functions.as_ref() {
            for function in functions {
                if let UsbFunction::Mouse(hid) = function {
                    write_to_mouse(hid, &buttons, pointer, wheel)?;
                    break; // there can always only be 1 mouse per design
                }
            }
        };
        Ok(())
    }
}

/// Implementation of the trait functions for `remote-usb`s `AppState`.
#[async_trait]
impl HasSerial for AppState {
    /// Return the SerialState for the given serial_id.
    async fn get_serial(&self, id: &'_ str) -> Option<SerialState> {
        self.inner.lock().await.serials.get(id).cloned()
    }
    /// Return all known SerialStates.
    async fn get_serials(&self) -> Vec<SerialState> {
        HashMap::clone(&self.inner.lock().await.serials)
            .values()
            .cloned()
            .collect()
    }
    /// Return all known serial ids.
    async fn get_serial_ids(&self) -> Vec<String> {
        self.inner.lock().await.serials.keys().cloned().collect()
    }
}

/// Presses and/or releases the given keys and an optional modifier on the
/// given keyboard hid interface.
fn write_to_keyboard(
    hid: &Hid,
    modifier: Option<Modifiers>,
    keys: Vec<Key>,
    press: bool,
    release: bool,
) -> Result<(), std::io::Error> {
    let (_, minor) = hid.device().unwrap();
    let device_path = format!("/dev/hidg{minor}");
    let mut device = Device::<Keyboard>::open(device_path)?;
    let mut input = Keyboard.input();

    if press {
        if let Some(modi) = modifier {
            input.press_mods(modi);
        }
        for key in &keys {
            input.press_key(*key);
        }
        device.input(&input)?;
    }
    if release {
        if let Some(modi) = modifier {
            input.release_mods(modi);
        }
        for key in keys {
            input.release_key(key);
        }
        device.input(&input)?;
    }

    Ok(())
}

/// Send the given mouse configuration to the given mouse hid interface.
fn write_to_mouse(
    hid: &Hid,
    buttons: &Vec<Button>,
    pointer: (u16, u16),
    wheel: i8,
) -> Result<(), std::io::Error> {
    let (_, minor) = hid.device().unwrap();
    let device_path = format!("/dev/hidg{minor}");
    let mut device = Device::<Mouse>::open(device_path)?;
    let mut input = Mouse.input();

    for button in buttons {
        input.press_button(*button);
    }

    // The crate we are using expects an i16, but we are now using a logical descriptor for absolute coordinates within this range.
    input.set_pointer((
        pointer.0.min(i16::MAX as u16) as i16,
        pointer.1.min(i16::MAX as u16) as i16,
    ));

    input.set_wheel(wheel);
    device.input(&input)?;

    for button in buttons {
        input.release_button(*button);
    }
    device.input(&input)?;

    Ok(())
}
