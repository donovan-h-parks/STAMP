//! STAMP graphical application.
//!
//! A native window on Windows, Linux and macOS.

pub mod app;
pub mod assistant;
pub mod figure_painter;
pub mod headless;
pub mod icons;
pub mod jobs;
pub mod layout;
pub mod platform;
pub mod plot_config;
pub mod plots;
pub mod select_features;
pub mod session;
pub mod wizard;
pub mod wizard_ui;

pub use app::StampApp;
