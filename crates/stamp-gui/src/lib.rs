//! STAMP graphical application.
//!
//! The same code is built as a native window on Windows, Linux and macOS and,
//! later, as WebAssembly for the browser.

pub mod app;
pub mod figure_painter;
pub mod headless;
pub mod jobs;
pub mod platform;
pub mod plots;
pub mod session;

pub use app::StampApp;
