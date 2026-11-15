//! STAMP plots.
//!
//! Each plot builds a backend-neutral [`Figure`]: positioned shapes and text,
//! plus the data item each mark belongs to. A figure has two renderers:
//!
//! * the GUI draws it with egui's painter (in `stamp-gui`), which keeps plots
//!   sharp on screen and leaves room for interaction such as hover, selection
//!   and zoom;
//! * this crate exports it as SVG and PDF, each written directly from the
//!   figure, and as PNG drawn from the SVG (with resvg, the `png` feature;
//!   the browser build has the browser draw the SVG instead).
//!
//! Both renderers share one bundled font (DejaVu Sans, the font of STAMP v2's
//! plots) and one text-measurement routine, so that on-screen layout and
//! exported layout match on every platform.
//!
//! This crate must stay free of egui and of filesystem access.

pub mod ancom_plot;
pub mod axes;
pub mod axis;
pub mod co_occurrence_plot;
pub mod community_plots;
pub mod composition;
pub mod correlation_plot;
pub mod export;
pub mod extended_error_bar;
pub mod feature_plots;
pub mod figure;
pub mod geo;
pub mod group_heatmap;
pub mod heatmap;
pub mod histogram;
pub mod map_plot;
pub mod multi_box;
pub mod ordination_plot;
pub mod ordination_style;
pub mod p_values;
pub mod pdf;
pub mod post_hoc;
pub mod prevalence_plot;
pub mod rank_abundance;
pub mod rarefaction;
pub mod sample_plots;
pub mod scatter;
pub mod shared_features;
pub mod svg;
pub mod text;
pub mod volcano;

pub use figure::{Color, Figure, HAlign, Region, Shape, Stroke, TextStyle, VAlign};
