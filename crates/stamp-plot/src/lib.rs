//! STAMP plots.
//!
//! Each plot builds a backend-neutral [`Figure`]: positioned shapes and text,
//! plus the data item each mark belongs to. A figure has two renderers:
//!
//! * the GUI draws it with egui's painter (in `stamp-gui`), which keeps plots
//!   sharp on screen and leaves room for interaction such as hover, selection
//!   and zoom;
//! * this crate exports it as SVG, and as PNG and PDF derived from that SVG,
//!   so exported files always agree with each other.
//!
//! Both renderers share one bundled font (DejaVu Sans, the font of STAMP v2's
//! plots) and one text-measurement routine, so that on-screen layout and
//! exported layout match on every platform.
//!
//! This crate must stay free of egui and of filesystem access.

pub mod axes;
pub mod axis;
pub mod composition;
pub mod export;
pub mod extended_error_bar;
pub mod feature_plots;
pub mod figure;
pub mod heatmap;
pub mod histogram;
pub mod pca_plot;
pub mod post_hoc;
pub mod rarefaction;
pub mod sample_plots;
pub mod scatter;
pub mod svg;
pub mod text;
pub mod volcano;

pub use figure::{Color, Figure, HAlign, Region, Shape, Stroke, TextStyle, VAlign};
