// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains types representing base urls for the various peripheral classes that each correspond to a standalone remote-hands service.
//! These urls are all of the form <url>/<peripheral class> which the types implemented here enforce via JSON schemas and fallible constructors.

use super::{Deserialize, InvalidUri, JsonSchema, Serialize, Uri};
use paste::paste;
use remote_client::RemoteClient;

/// Base address for a remote power endpoint
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct RemotePowerBaseUrl(#[schemars(url, regex(pattern = r".*\/power$"))] String);

/// Base address for a remote serial endpoint
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct RemoteSerialBaseUrl(#[schemars(url, regex(pattern = r".*\/serial$"))] String);

/// Base address for a remote auxiliary endpoint
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct RemoteAuxiliaryBaseUrl(#[schemars(url, regex(pattern = r".*\/auxiliaries$"))] String);

/// Base address for a remote usb endpoint
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct RemoteUsbBaseUrl(#[schemars(url, regex(pattern = r".*\/usb$"))] String);

/// Macro to implement `with_specialization` and `into_reset` methods on the Uri types in this
/// module. It takes the name of the type (e.g. `RemotePowerBaseUrl`)  and
/// a string literal that adds an example use of the `with_specialization` method.
macro_rules! base_url_manipulation {
    ($name:ident, $specialization_example:literal) => {
        impl $name {
            // The paste macro helps us inject the given $specialization_example literal into the doc string. Note that
            // the usual "///" doc comments syntax actually desugars to #[doc]
            paste!{
                #[doc = r" Append a specialization (i.e. a path extension possibly containing a query and a fragment) to the
                    base url.  The given `specialization` is expected to NOT start with a leading `/`, and must produce a
                    valid `Uri` once it is joined with this base url."
                    $specialization_example]
                pub fn with_specialization(self, specialization: &str) -> Result<Uri, InvalidUri> {
                    Uri::try_from(format!("{}/{}", self.0, specialization))
                }

                /// Convert this remote service base url into the url for resetting all interfaces of this peripheral class.
                #[allow(dead_code)]
                fn into_reset(self) -> reqwest::Url {
                    reqwest::Url::try_from(format!("{}/reset", self.0).as_str())
                        .expect("Appending /reset to a valid Uri should still return a valid Uri")
                }

            }
        }

    };
}
/// Macro that generates a method on the Uri type that sends a
/// `POST /<peripheral class>/reset` in order to reset the given
/// peripheral class using the provided client.
macro_rules! apply_reset_impl {
    ($name:ident, $reset_doc_string:literal) => {
        impl $name {
            paste!{
                #[doc = $reset_doc_string]
                #[tracing::instrument(skip_all)]
                pub async fn apply_reset(self, client: &RemoteClient) -> Result<reqwest::Response, reqwest::Error> {
                    let reset_url = self.into_reset();
                    tracing::debug!(%reset_url, "going to send reset request to remote-hands service");
                    client.client.post(reset_url).send().await
                }
            }
        }
    };
}

/// Macro to implement `TryFrom<String>`, `Into<Uri>`, `Into<String>` and Display for the Uri types in this module.
/// Also introduces a custom error type used with the `TryFrom<String>` implementation. The generated error type's
/// name starts with "Invalid" and is joined with the name of the given Uri type. In the case of [`RemotePowerBaseUrl`]
/// for example we get an error type named `InvalidRemotePowerBaseUrl`.
///
/// The generated error enum has two variants; One indicates that the string does not end with /{endpoint name}
/// which is named `DoesNotEndWith{endpoint name in camel case}` and the other variant is always named `InvalidUrl`.
///
/// The macro takes the name of the Uri type and the name of the endpoint excluding a leading "/".
macro_rules! base_url_conversion {
    ($name:ident, $endpoint:literal) => {
        paste! {
            #[doc = "Error that may occur when attempting to parse [`" $name "`]"]
            #[derive(Debug)]
            pub enum [<Invalid $name>] {
                InvalidUrl(InvalidUri),
                [<DoesNotEndWith $endpoint:camel>]
            }

            impl std::fmt::Display for [<Invalid $name>] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    use [<Invalid $name>]::*;
                    write!(f, "could not be parsed: ")?;
                    match self {
                        InvalidUrl(_) => write!(f, "not a valid uri"),
                        [<DoesNotEndWith $endpoint:camel>]  => {
                            write!(f, "the url does not end with /{}", $endpoint)
                        }
                    }
                }
            }

            impl std::error::Error for [<Invalid $name>] {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    if let Self::InvalidUrl(url_err) = self {
                        Some(url_err)
                    } else {
                        None
                    }
                }
            }

            impl TryFrom<String> for $name  {
                type Error = [<Invalid $name>];
                fn try_from(value: String) -> Result<Self, Self::Error> {
                    // check that this is a valid Uri
                    Uri::try_from(&value).map_err(Self::Error::InvalidUrl)?;
                    value
                        .ends_with(concat!("/", $endpoint))
                        .then_some(Self(value))
                        .ok_or([<Invalid $name>]::[<DoesNotEndWith $endpoint:camel>])
                }
            }

            impl From<$name> for Uri {
                fn from(value: $name) -> Self {
                    // The expect is OK, because one can only construct this value via
                    // TryFrom::<String>
                    Uri::try_from(value.0)
                        .expect(concat!("The constructors and methods available on ", stringify!($name), " should ensure that this is a valid url"))
                }
            }

            impl From<$name> for String {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
        }
    };
}

base_url_manipulation!(
    RemotePowerBaseUrl,
    "\n \n This can be used to for instance target specific power interfaces."
);

base_url_conversion!(RemotePowerBaseUrl, "power");

apply_reset_impl!(
    RemotePowerBaseUrl,
    r" Reset all power interfaces belonging to the
    associated machine."
);

base_url_manipulation!(
    RemoteSerialBaseUrl,
    "\n \n This can be used to for instance target specific serial interfaces."
);

base_url_conversion!(RemoteSerialBaseUrl, "serial");

apply_reset_impl!(
    RemoteSerialBaseUrl,
    r" Reset all serial interfaces belonging to the
        associated machine."
);

base_url_manipulation!(
    RemoteAuxiliaryBaseUrl,
    "\n \n This can be used to for instance target specific auxiliary devices"
);
base_url_conversion!(RemoteAuxiliaryBaseUrl, "auxiliaries");
apply_reset_impl!(
    RemoteAuxiliaryBaseUrl,
    r"Reset all auxiliary devices belonging to the associated machine"
);

base_url_manipulation!(
    RemoteUsbBaseUrl,
    "\n \n This can be used to for instance target specific usb functions."
);
base_url_conversion!(RemoteUsbBaseUrl, "usb");

apply_reset_impl!(
    RemoteUsbBaseUrl,
    r" Reset the usb interface belonging to the
        associated machine."
);

#[cfg(test)]
mod tests {
    use super::{
        InvalidRemoteAuxiliaryBaseUrl, InvalidRemotePowerBaseUrl, InvalidRemoteSerialBaseUrl,
        InvalidRemoteUsbBaseUrl, RemoteAuxiliaryBaseUrl, RemotePowerBaseUrl, RemoteSerialBaseUrl,
        RemoteUsbBaseUrl,
    };
    use paste::paste;

    /// Generates two tests for the given Uri type. One that checks that parsing the
    /// type has the expected behavior and one that asserts that converting the type
    /// to its string representation works as expected.
    /// The macro takes the type (name), the expected enum error variant that
    /// indicates that parsing failed and finally the end point name (i.e. "power", "serial", etc).
    macro_rules! remote_url_test {
        ($uri_type:ty, $parsing_error:path, $endpoint:literal) => {
            paste! {
                // We use paste to join substrings to name the parsing test
                // in the case of $endpoint:literal = "power" this test will be
                // named "remote_base_power_uri_parsing"
                #[test]
                fn [<remote _base_ $endpoint _uri _parsing>]() {
                    // parsing a Uri that does not end with the expected peripheral class name should fail.
                    assert!(matches!(
                        $uri_type::try_from(String::from("http://example.com")).unwrap_err(),
                        $parsing_error
                    ));
                    assert!(matches!(
                        $uri_type::try_from(String::from(concat!("http://example.com/not-", $endpoint))).unwrap_err(),
                        $parsing_error
                    ));
                    // parsing a valid remote base Uri should work: i.e. (http://example.com/power) should be ok.
                    assert!($uri_type::try_from(String::from(concat!("http://example.com/", $endpoint))).is_ok());
                }

                // We use paste to join substrings in the case of e.g. "power" the conversion test
                // will be named "remote_base_power_uri_conversion"
                #[test]
                fn [<remote _base_ $endpoint _uri _conversion>]() {
                    let string_repr = String::from(concat!("http://example.com/", $endpoint));
                    let parsed = $uri_type::try_from(string_repr.clone()).unwrap();
                    assert_eq!(string_repr, String::from(parsed));
                }
            }
        };
    }

    remote_url_test!(
        RemotePowerBaseUrl,
        InvalidRemotePowerBaseUrl::DoesNotEndWithPower,
        "power"
    );

    remote_url_test!(
        RemoteSerialBaseUrl,
        InvalidRemoteSerialBaseUrl::DoesNotEndWithSerial,
        "serial"
    );
    remote_url_test!(
        RemoteAuxiliaryBaseUrl,
        InvalidRemoteAuxiliaryBaseUrl::DoesNotEndWithAuxiliaries,
        "auxiliaries"
    );
    remote_url_test!(
        RemoteUsbBaseUrl,
        InvalidRemoteUsbBaseUrl::DoesNotEndWithUsb,
        "usb"
    );
}
