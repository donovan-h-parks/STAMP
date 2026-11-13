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

/// Version of the STAMP port.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
