// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::hid_report::{
    DEFAULT_KEYBOARD_REPORT_DESCRIPTOR, DEFAULT_KEYBOARD_REPORT_LENGTH,
    DEFAULT_MOUSE_REPORT_DESCRIPTOR, DEFAULT_MOUSE_REPORT_LENGTH,
};
use itertools::Itertools;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::io::{Error, ErrorKind};
#[cfg(feature = "usb-ethernet")]
use usb_gadget::function::net::{Net, NetClass};
#[cfg(feature = "usb-serial")]
use usb_gadget::function::serial::{Serial, SerialClass};
use usb_gadget::{
    Class, Config, Gadget, Id, Strings,
    function::{
        Handle,
        hid::Hid,
        msd::{Lun, Msd},
    },
};

/// User-facing meta information after configuration
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UsbFunctionInfo {
    #[cfg(feature = "usb-ethernet")]
    Ethernet,
    Keyboard,
    Mouse,
    #[cfg(feature = "usb-serial")]
    Serial {
        serial_id: Option<String>,
    },
    Storage {
        luns: Vec<StorageLunConfig>,
    },
}

/// Internal meta information after configuration
#[derive(Debug)]
pub enum UsbFunction {
    #[cfg(feature = "usb-ethernet")]
    Ethernet(Net),
    Keyboard(Hid),
    Mouse(Hid),
    #[cfg(feature = "usb-serial")]
    Serial {
        serial: Serial,
        serial_id: Option<String>,
    },
    Storage {
        msd: Msd,
        luns: Vec<StorageLunConfig>,
    },
}

impl UsbFunction {
    /// Return user-facing meta information for the given internal function.
    pub fn get_info(&self) -> UsbFunctionInfo {
        match self {
            #[cfg(feature = "usb-ethernet")]
            UsbFunction::Ethernet(..) => UsbFunctionInfo::Ethernet,
            UsbFunction::Keyboard(..) => UsbFunctionInfo::Keyboard,
            UsbFunction::Mouse(..) => UsbFunctionInfo::Mouse,
            #[cfg(feature = "usb-serial")]
            UsbFunction::Serial { serial_id, .. } => UsbFunctionInfo::Serial {
                serial_id: serial_id.clone(),
            },
            UsbFunction::Storage { luns, .. } => UsbFunctionInfo::Storage { luns: luns.clone() },
        }
    }

    #[cfg(feature = "usb-serial")]
    /// Configure the name of a serial interface
    pub fn set_serial_id(&mut self, id: String) {
        if let UsbFunction::Serial { serial_id, .. } = self {
            *serial_id = Some(id);
        }
    }
}

// Only exists for serde default usage.
// Waiting for serde to enable `default_value` or alike.
const fn default_storage_lun_config_cdrom() -> bool {
    false
}

// Only exists for serde default usage.
// Waiting for serde to enable `default_value` or alike.
const fn default_storage_lun_config_read_only() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Ord, PartialOrd, Eq, PartialEq)]
/// Config for one LUN
pub struct StorageLunConfig {
    /// Image path
    pub path: String,
    /// Emulate a CD-ROM drive?
    #[serde(default = "default_storage_lun_config_cdrom")]
    pub cdrom: bool,
    /// Read-only?
    #[serde(default = "default_storage_lun_config_read_only")]
    pub read_only: bool,
}

/// Configuration in request
///
/// The order of functions is always determined by this given
/// top-to-bottom discriminant order.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UsbFunctionConfig {
    #[cfg(feature = "usb-serial")]
    Serial {
        serial_id: Option<String>,
    },
    Storage {
        luns: Vec<StorageLunConfig>,
    },
    Keyboard,
    Mouse,
    #[cfg(feature = "usb-ethernet")]
    Ethernet,
}

impl UsbFunctionConfig {
    /// The type of a UsbFunctionConfig as a String.
    /// Needed to be able to count how many types of the same kind are
    /// configured, since the inner configuration (e.g. `luns`) could be
    /// different and therefore normal comparison with `Eq` does not work
    fn function_type(&self) -> String {
        match self {
            #[cfg(feature = "usb-ethernet")]
            UsbFunctionConfig::Ethernet => "Ethernet".to_string(),
            UsbFunctionConfig::Keyboard => "Keyboard".to_string(),
            UsbFunctionConfig::Mouse => "Mouse".to_string(),
            #[cfg(feature = "usb-serial")]
            UsbFunctionConfig::Serial { .. } => "Serial".to_string(),
            UsbFunctionConfig::Storage { .. } => "Storage".to_string(),
        }
    }

    /// The amount of the same kind of UsbFunctionConfig that is allowed
    /// to be configured for one USB OTG device. Currently we only
    /// restrict the amount of Keyboard and Mouse configuration to 1.
    fn max_supported(&self) -> Option<usize> {
        match self.function_type().as_str() {
            "Keyboard" => Some(1),
            "Mouse" => Some(1),
            _ => None,
        }
    }

    /// Create and return the handle and function belonging to the given
    /// `UsbFunctionConfig`.
    fn usb_handle(&self) -> Result<(Handle, UsbFunction), Error> {
        match self {
            #[cfg(feature = "usb-ethernet")]
            UsbFunctionConfig::Ethernet => {
                let (net, handle) = Net::new(NetClass::Ncm);
                let function = UsbFunction::Ethernet(net);
                Ok((handle, function))
            }
            UsbFunctionConfig::Keyboard => {
                let mut builder = Hid::builder();
                builder.protocol = 1; // keyboard-specific
                builder.sub_class = 1; // use while boot process is ongoing
                builder.report_len = DEFAULT_KEYBOARD_REPORT_LENGTH;
                builder.report_desc = DEFAULT_KEYBOARD_REPORT_DESCRIPTOR.to_vec();
                let (hid, handle) = builder.build();
                let function = UsbFunction::Keyboard(hid);
                Ok((handle, function))
            }
            UsbFunctionConfig::Mouse => {
                let mut builder = Hid::builder();
                builder.protocol = 2; // mouse-specific
                builder.sub_class = 1; // use while boot process is ongoing
                builder.report_len = DEFAULT_MOUSE_REPORT_LENGTH;
                builder.report_desc = DEFAULT_MOUSE_REPORT_DESCRIPTOR.to_vec();
                let (hid, handle) = builder.build();
                let function = UsbFunction::Mouse(hid);
                Ok((handle, function))
            }
            #[cfg(feature = "usb-serial")]
            UsbFunctionConfig::Serial { serial_id } => {
                // TODO: check if Acm or Generic is better
                // (change tty path inside test image if class is changed here)
                let mut serial_builder = Serial::builder(SerialClass::Acm);
                // TODO: check if console is needed or not, use `Serial::new` if not
                serial_builder.console = Some(true);
                let (serial, handle) = serial_builder.build();
                let function = UsbFunction::Serial {
                    serial,
                    serial_id: serial_id.clone(),
                };
                Ok((handle, function))
            }
            UsbFunctionConfig::Storage { luns } => {
                let mut msd = Msd::builder();
                for StorageLunConfig {
                    path,
                    cdrom,
                    read_only,
                } in luns
                {
                    let mut lun = Lun::new(path)?;
                    lun.cdrom = *cdrom;
                    lun.read_only = *read_only;
                    msd.add_lun(lun);
                }
                let (msd, handle) = msd.build();
                let function = UsbFunction::Storage {
                    msd,
                    luns: luns.clone(),
                };
                Ok((handle, function))
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
/// List of `UsbFunctionConfig` from the API when configuring USB.
pub struct UsbConfig {
    #[serde(deserialize_with = "sort_vec")]
    pub functions: Vec<UsbFunctionConfig>,
}

fn sort_vec<'de, D>(deserializer: D) -> Result<Vec<UsbFunctionConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut vec = Vec::<UsbFunctionConfig>::deserialize(deserializer)?;
    vec.sort();
    Ok(vec)
}

impl UsbConfig {
    /// Verify if the configured functions exceed the allowed amount.
    pub fn verify(self) -> Result<(), Error> {
        let counts = &self
            .functions
            .clone()
            .into_iter()
            .map(|f| f.function_type())
            .counts();
        for function in &self.functions {
            if let Some(max) = function.max_supported() {
                let function_type = function.function_type();
                if let Some(count) = counts.get(function_type.as_str())
                    && max < *count
                {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Max amount of '{function_type}' type function is {max}, but {count} were defined."
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Make and return the USB gadget and all belonging functions
    pub fn make_gadget(self) -> Result<(Gadget, Vec<UsbFunction>), Error> {
        let mut config = Config::new("remote-hands");
        let mut functions = Vec::with_capacity(self.functions.len());
        for function in &self.functions {
            let (handle, function) = function.usb_handle()?;
            config.add_function(handle);
            functions.push(function);
        }

        // Set up gadget with class, subclass, protocol number, vendor ID, product ID,
        // manufacturer, product name and serial number.
        // Additional values are set to defaults or auto detected, like usb version,
        // max packet size or max speed.
        let gadget = Gadget::new(
            // Class, subclass and protocol number must not match any from this list:
            // https://www.usb.org/defined-class-codes. Otherwise functionality might
            // be restricted and not needed device drivers are loaded.
            Class::new(0, 0, 0),
            Id::new(
                0x1209, // Generic Vendor ID: https://devicehunt.com/all-usb-vendors
                0x0104, // Multifunction Composite Gadget
            ),
            // Manufacturer, product name and serial number are only informational.
            Strings::new("secunet", "remote-hands", "1"),
        )
        .with_config(config);
        Ok((gadget, functions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_simple() {
        let value = serde_json::to_value(UsbConfig {
            functions: vec![UsbFunctionConfig::Keyboard],
        })
        .unwrap();
        assert_eq!(value, serde_json::json!([{ "type": "keyboard" }]));
    }

    #[test]
    fn serialize_all() {
        let value = serde_json::to_value(UsbConfig {
            functions: vec![
                UsbFunctionConfig::Storage {
                    luns: vec![StorageLunConfig {
                        path: "/storage/image".to_string(),
                        cdrom: false,
                        read_only: true,
                    }],
                },
                UsbFunctionConfig::Keyboard,
                UsbFunctionConfig::Mouse,
            ],
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!([{
                "type": "storage",
                "luns": [{
                    "path": "/storage/image",
                    "cdrom": false,
                    "read_only": true,
                }]
            }, {
                "type": "keyboard",
            }, {
                "type": "mouse",
            }])
        );
    }

    #[test]
    fn deserialize_functions() {
        let cfg: UsbConfig = serde_json::from_value(serde_json::json!([
            {
                "type": "keyboard",
            },
            {
                "type": "mouse",
            },
            {
                "type": "storage",
                "luns": [{
                    "path": "/storage/image",
                    "cdrom": false,
                    "read_only": true,
                }]
            },
        ]))
        .expect("failed to deserialize usb config");

        let exp = UsbConfig {
            functions: vec![
                UsbFunctionConfig::Storage {
                    luns: vec![StorageLunConfig {
                        path: "/storage/image".to_string(),
                        cdrom: false,
                        read_only: true,
                    }],
                },
                UsbFunctionConfig::Keyboard,
                UsbFunctionConfig::Mouse,
            ],
        };

        assert_eq!(cfg.functions, exp.functions);
    }
}
