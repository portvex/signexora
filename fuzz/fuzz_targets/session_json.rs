//! Parse arbitrary text as a session file, write it back, and parse it again.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sn_session::SessionFile;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(session) = SessionFile::from_json(text) else {
        return;
    };
    let _ = session.measurement_context();
    if let Ok(again) = session.to_json() {
        let _ = SessionFile::from_json(&again);
    }
});
