//! Keyboard commands: what a key press asks the app to do, and how the
//! shortcuts window groups them. The key bindings themselves depend on the
//! front end's key types and live there.

/// Something a key press asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Open a recording.
    OpenRecording,
    /// Save the session.
    SaveSession,
    /// Save the session under a new name.
    SaveSessionAs,
    /// Close the active window.
    CloseTab,
    /// Activate the next window.
    NextTab,
    /// Activate the previous window.
    PrevTab,
    /// Select the next signal.
    NextSignal,
    /// Select the previous signal.
    PrevSignal,
    /// Open the selected signal in the view at this index of the app's view list.
    OpenView(usize),
    /// Open the workflow guide.
    Guide,
    /// Open the shortcuts window.
    ShortcutsWindow,
    /// Open the signal generator.
    SignalGenerator,
    /// Restore the markers.
    RestoreMarkers,
    /// Play the selection.
    Play,
    /// Zoom to the selection.
    ZoomToSelection,
    /// Undo the last zoom.
    ZoomBack,
    /// Show the whole signal.
    FullView,
    /// Measure the baud rate.
    MeasureBr,
    /// Measure the carrier.
    MeasureSc,
}

/// A group of rows in the shortcuts window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// General commands.
    General,
    /// Commands that open views.
    Views,
    /// Plot commands.
    Plot,
    /// Built-in interface keys.
    Interface,
}

impl Section {
    /// Every section, in window order.
    pub const ALL: [Self; 4] = [Self::General, Self::Views, Self::Plot, Self::Interface];

    /// Fluent id of the section's heading.
    pub const fn key(self) -> &'static str {
        match self {
            Self::General => "keys-section-general",
            Self::Views => "keys-section-views",
            Self::Plot => "keys-section-plot",
            Self::Interface => "keys-section-interface",
        }
    }
}

impl Command {
    /// Fluent id of what the command does.
    pub const fn key(self) -> &'static str {
        match self {
            Self::OpenRecording => "keys-open-recording",
            Self::SaveSession => "keys-save-session",
            Self::SaveSessionAs => "keys-save-session-as",
            Self::CloseTab => "keys-close-tab",
            Self::NextTab => "keys-next-tab",
            Self::PrevTab => "keys-prev-tab",
            Self::NextSignal => "keys-next-signal",
            Self::PrevSignal => "keys-prev-signal",
            Self::OpenView(_) => "keys-open-view",
            Self::Guide => "keys-guide",
            Self::ShortcutsWindow => "keys-shortcuts",
            Self::SignalGenerator => "gen-keys-open",
            Self::RestoreMarkers => "keys-restore-markers",
            Self::Play => "keys-play",
            Self::ZoomToSelection => "keys-zoom-selection",
            Self::ZoomBack => "keys-zoom-back",
            Self::FullView => "keys-full-view",
            Self::MeasureBr => "keys-measure-br",
            Self::MeasureSc => "keys-measure-sc",
        }
    }

    /// The command acts only while no text field or other control takes the
    /// keyboard: single keys and keys that edit text (Alt+Up moves the text
    /// cursor, Cmd+Z undoes typing).
    pub const fn needs_free_keys(self) -> bool {
        matches!(
            self,
            Self::NextSignal
                | Self::PrevSignal
                | Self::RestoreMarkers
                | Self::Play
                | Self::ZoomToSelection
                | Self::ZoomBack
                | Self::FullView
                | Self::MeasureBr
                | Self::MeasureSc
        )
    }

    /// The section of the shortcuts window that lists the command.
    pub const fn section(self) -> Section {
        match self {
            Self::OpenView(_) => Section::Views,
            Self::RestoreMarkers
            | Self::Play
            | Self::ZoomToSelection
            | Self::ZoomBack
            | Self::FullView
            | Self::MeasureBr
            | Self::MeasureSc => Section::Plot,
            Self::OpenRecording
            | Self::SaveSession
            | Self::SaveSessionAs
            | Self::CloseTab
            | Self::NextTab
            | Self::PrevTab
            | Self::NextSignal
            | Self::PrevSignal
            | Self::Guide
            | Self::ShortcutsWindow
            | Self::SignalGenerator => Section::General,
        }
    }
}

/// Number of views Cmd+1 and on open (the app's `TabKind::ALL`).
pub const VIEW_COUNT: usize = 6;

/// The index after (or before) `current` in a list of `len` items, wrapping
/// around. With nothing current, the first (or last) item.
pub fn step(len: usize, current: Option<usize>, forward: bool) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match (current.filter(|c| *c <= last), forward) {
        (None, true) => 0,
        (None, false) => last,
        (Some(c), true) => {
            if c >= last {
                0
            } else {
                c + 1
            }
        }
        (Some(c), false) => c.checked_sub(1).unwrap_or(last),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t;

    #[test]
    fn every_section_has_an_english_message() {
        for s in Section::ALL {
            assert_ne!(t!(s.key()), s.key());
        }
    }

    #[test]
    fn step_wraps_around() {
        assert_eq!(step(0, None, true), None);
        assert_eq!(step(0, Some(0), false), None);
        assert_eq!(step(3, None, true), Some(0));
        assert_eq!(step(3, None, false), Some(2));
        assert_eq!(step(3, Some(0), true), Some(1));
        assert_eq!(step(3, Some(2), true), Some(0));
        assert_eq!(step(3, Some(0), false), Some(2));
        assert_eq!(step(3, Some(1), false), Some(0));
        // A stale index counts as none.
        assert_eq!(step(3, Some(7), true), Some(0));
    }
}
