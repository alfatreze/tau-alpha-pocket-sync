//! TAU card logic for the Pocket Sync plugin.
//!
//! Everything here works on a plain directory (the card root) and knows nothing about the plugin host, so it is
//! tested natively and then used unchanged inside the WebAssembly plugin. The plugin crate only turns it into menus.

pub mod covers;
pub mod library;
pub mod names;
pub mod rebuild;
pub mod report;
pub mod scan;

pub use scan::{Format, Kind, Limits, Report, classify, scan};
