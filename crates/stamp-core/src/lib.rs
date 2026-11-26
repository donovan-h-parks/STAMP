//! Core of STAMP: profiles, metadata, file parsers, statistical tests,
//! multiple-test corrections and filtering.
//!
//! Rules for this crate, which keep it easy to test and let the GUI and the
//! command line share it:
//!
//! * no filesystem access: parsers take bytes or strings, never paths;
//! * no threads: callers decide how to schedule work;
//! * no UI or plotting types.
//!
//! Results are checked against the reference outputs in `tests/golden`,
//! generated from the original Python implementation (see `legacy/golden`).

// Reference values in tests are copied verbatim from their sources.
#![cfg_attr(test, allow(clippy::excessive_precision))]

pub mod error;
pub mod import;
pub mod metadata;
pub mod profile;
pub mod profile_tree;
pub mod selection;
pub mod stats;
pub mod text;

pub use error::{Result, StampError};
pub use metadata::{Groups, Metadata, Pairing};
pub use profile::{
    CorrelationProfile, Feature, GroupProfile, MultiGroupProfile, Profile, SampleProfile,
    SecondGroup, Unclassified, ALL_OTHER_SAMPLES,
};
pub use profile_tree::{ProfileTree, ENTIRE_SAMPLE};
pub use selection::{Comparison, Condition, SampleSelection};

/// Version of the STAMP port.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
