//! A Windows 95 widget kit for GPUI: classic colours and metrics, 3D borders,
//! controls, menus, MDI child windows, status bar, toolbar buttons, tooltips,
//! dialogs and message boxes.
//!
//! Every control is placed at fixed pixel coordinates (like a Delphi form) and
//! drawn on the pixel grid, so it looks the same at 1x and 2x. See `kit` for
//! how a view hosts the widgets.

pub(crate) mod bars;
pub(crate) mod button;
pub(crate) mod colors;
pub(crate) mod combo;
pub(crate) mod dialog;
pub(crate) mod edit;
pub(crate) mod geom;
pub(crate) mod glyph;
pub(crate) mod kit;
pub(crate) mod label;
pub(crate) mod marks;
pub(crate) mod mdi;
pub(crate) mod mdi_view;
pub(crate) mod menu;
pub(crate) mod menu_view;
pub(crate) mod paint;
pub(crate) mod progress;
pub(crate) mod scrollbar;
pub(crate) mod skin;
pub(crate) mod skin_aero;
pub(crate) mod skin_flat;
pub(crate) mod skin_fluent;
pub(crate) mod skin_linux;
pub(crate) mod skin_mac;
pub(crate) mod style;
pub(crate) mod text;
pub(crate) mod trackbar;
pub(crate) mod widget;
