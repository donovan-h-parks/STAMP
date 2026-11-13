//! Core of STAMP: profiles, metadata, file parsers, statistical tests,
//! multiple-test corrections and filtering.
//!
//! Rules for this crate, so that it compiles unchanged for desktop and for
//! WebAssembly:
//!
//! * no filesystem access: parsers take bytes or strings, never paths;
//! * no threads: callers decide how to schedule work;
//! * no UI or plotting types.
//!
//! Results are checked against the reference outputs in `tests/golden`,
//! generated from the original Python implementation (see `legacy/golden`).

pub mod error;
pub mod import;
pub mod metadata;
pub mod profile;
pub mod profile_tree;
pub mod text;

pub use error::{Result, StampError};
pub use metadata::{Groups, Metadata};
pub use profile::{
    Feature, GroupProfile, MultiGroupProfile, Profile, SampleProfile, SecondGroup, Unclassified,
    ALL_OTHER_SAMPLES,
};
pub use profile_tree::{ProfileTree, ENTIRE_SAMPLE};

/// Version of the STAMP port.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
