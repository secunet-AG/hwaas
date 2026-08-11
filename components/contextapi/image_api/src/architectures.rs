//! # Boot Image Architectures
//!
//! As part of their metadata, BMR boot images can store their target platform architecture. In
//! order to avoid the proliferation of a dozen similar architecture names being used to refer to
//! the same thing, this module defines all currently supported and valid architectures instead.
//!
//! If you feel that an architecture you require isn't available here, don't hesitate to get in
//! touch with the HWaaS maintainers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Mapping between valid string representation and architecture enum discriminant.
// NOTE: The order of entries matters and selects the "default" conversion for ambiguous cases.
static ARCH_MAPPING: phf::OrderedMap<&'static str, Architecture> = phf::phf_ordered_map! {
    "x86_64" => Architecture::X86_64,
    "amd64" => Architecture::X86_64,
    "x86_32" => Architecture::X86_32,
    "aarch64" => Architecture::Aarch64,
    "arm64" => Architecture::Aarch64,
    "aarch32" => Architecture::Aarch32,
    "arm32" => Architecture::Aarch32,
    "riscv64" => Architecture::Riscv64,
    "riscv32" => Architecture::Riscv32,
};

/// Supported and known architecture names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(strum::EnumIter))]
pub enum Architecture {
    /// x86 (64-bit), also known as "amd64"
    X86_64,
    /// x86 (32-bit)
    X86_32,
    /// Aarch64 (64-bit), also known as "arm64"
    Aarch64,
    /// Aarch32 (32-bit), also known as "arm32"
    Aarch32,
    /// RISC-V (64-bit)
    Riscv64,
    /// RISC-V (32-bit)
    Riscv32,
}

#[derive(Debug, thiserror::Error, displaydoc::Display)]
/// failed to parse {arch:?} as architecture, valid choices are: {choices}
pub struct InvalidArch {
    arch: String,
    choices: String,
}

impl InvalidArch {
    /// Create a new error message.
    pub fn new<S: Into<String>>(arch: S) -> Self {
        let arch_strings = ARCH_MAPPING
            .keys()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        Self {
            arch: arch.into(),
            choices: arch_strings.join(", "),
        }
    }
}

impl std::str::FromStr for Architecture {
    type Err = InvalidArch;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ARCH_MAPPING
            .get(s.trim().to_ascii_lowercase().as_str())
            .ok_or(InvalidArch::new(s))
            .cloned()
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((word, _)) = ARCH_MAPPING.into_iter().find(|(_, arch)| arch == &self) {
            write!(f, "{}", word)
        } else {
            tracing::error!(
                "architecture {:?} has no string representation, using debug as fallback",
                self
            );
            write!(f, "{:?}", self)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use strum::IntoEnumIterator as _;

    #[test]
    fn roundtrip() {
        for variant in Architecture::iter() {
            eprintln!("testing roundtrip for variant {variant:?}");
            let variant_as_str = variant.to_string();
            let variant_from_str = Architecture::from_str(&variant_as_str)
                .expect("architecture should be parsable from stringified architecture");
            assert_eq!(variant, variant_from_str);
        }
    }

    #[test]
    fn roundtrip_with_serde() {
        for variant in Architecture::iter() {
            eprintln!("testing roundtrip for variant {variant:?}");
            let variant_as_str = variant.to_string();
            let variant_as_serde_str = serde_json::to_string(&variant)
                .expect("architecture should serialize into valid string");
            // It's JSON after all, so strings are quoted
            assert_eq!(format!("\"{}\"", variant_as_str), variant_as_serde_str);
        }
    }

    #[test]
    fn ignores_case_and_whitespace() {
        let expected = Architecture::X86_64;

        assert_eq!(expected, Architecture::from_str("x86_64").unwrap());
        assert_eq!(expected, Architecture::from_str("X86_64").unwrap());
        assert_eq!(expected, Architecture::from_str(" X86_64  ").unwrap());
        // NOTE: This one holds a tab character
        assert_eq!(expected, Architecture::from_str("	x86_64").unwrap());
    }

    #[test]
    fn makes_helpful_error_message() {
        let Err(error) = Architecture::from_str("nope") else {
            panic!("'nope' shouldn't parse as valid architecture");
        };
        let display = error.to_string();
        assert!(display.contains("valid choices are: x86_64, amd64, x86_32, aarch64,"));
    }

    #[test]
    fn ambiguous_variants() {
        assert_eq!(Architecture::X86_64.to_string(), "x86_64");
        assert_eq!(Architecture::Aarch64.to_string(), "aarch64");
        assert_eq!(Architecture::Aarch32.to_string(), "aarch32");
    }
}
