//! The user interface core of SigNexora, shared by the desktop front ends.
//!
//! Everything here is independent of the UI toolkit:
//! - [`i18n`]: the Fluent translations and the [`t!`] macro.
//! - [`prefs`]: the settings remembered across runs.
//! - [`theme`], [`colormap`] and [`color`]: theme choices, plot palettes and
//!   colour maps as plain RGBA data.
//! - [`titles`], [`format`](mod@format), [`markers`], [`view`] and [`lineage`]: window
//!   names, number formatting, the marker selection, the zoom and pan mapping,
//!   and how derived signals relate to their parents.
//! - [`correlation`]: SA's question and limits before an ACF or CCF.
//! - [`edits`], [`generator`], [`sono_settings`] and [`shortcuts`]: the
//!   parameters, validation and commands behind the menus and dialogs.
//! - [`sono_tools`] and [`sono_cache`]: the sonogram work window's toolbar,
//!   popup menu, Info overlay and marker rules, and its image.
//! - [`phase`], [`phase_plot`] and [`harmonics`]: the PhasePlane module with
//!   its ISD corrector, and the harmonics (power-of-N) module.
//! - [`demod`] and [`ofdm`]: the state, jobs and texts of the FSK/MFSK
//!   demodulator and the `SA-OFDM` module windows.
//! - [`wf_tools`], [`wf_plot`] and [`vmw_tools`]: the WF window's toolbar,
//!   popup menu, Info overlay and trace image, and its VMW mode's line period,
//!   colours and raster.
//! - [`app_glue`]: the questions, lists and choices of the application glue.
//! - [`jobs`], [`playback`], [`loader`], [`loader_files`] and [`loader_info`]:
//!   job error mapping, audio playback and the `SA Open file` logic.

#![warn(missing_docs)]

pub mod app_glue;
pub mod color;
pub mod colormap;
pub mod correlation;
pub mod demod;
pub mod edits;
pub mod format;
pub mod generator;
pub mod harmonics;
pub mod i18n;
pub mod jobs;
pub mod lineage;
pub mod loader;
pub mod loader_files;
pub mod loader_info;
pub mod markers;
pub mod ofdm;
pub mod phase;
pub mod phase_plot;
pub mod playback;
pub mod prefs;
pub mod shortcuts;
pub mod sono_cache;
pub mod sono_settings;
pub mod sono_tools;
pub mod theme;
pub mod titles;
pub mod view;
pub mod vmw_tools;
pub mod wf_plot;
pub mod wf_tools;
pub mod winstyle;

/// The Fluent crate, for the [`t!`] macro.
#[doc(hidden)]
pub use fluent_bundle;
