//! STAMP plots.
//!
//! Each plot builds a backend-neutral *figure*: positioned shapes and text,
//! plus the data item each mark belongs to. A figure has two renderers:
//!
//! * the GUI draws it with egui's painter (in `stamp-gui`), which keeps plots
//!   sharp on screen and leaves room for interaction such as hover, selection
//!   and zoom;
//! * this crate exports it as SVG, and as PNG and PDF derived from that SVG,
//!   so exported files always agree with each other.
//!
//! Both renderers share one bundled font and one text-measurement routine so
//! that on-screen layout and exported layout match on every platform.
//!
//! This crate must stay free of egui and of filesystem access.
