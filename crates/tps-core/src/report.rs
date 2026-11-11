//! Plain-text rendering of a scan for the plugin's log window (one line per fact, no colour, no markup).

use crate::scan::Report;

pub const TAU_OMEGA_URL: &str = "https://github.com/alfatreze/Tau-Omega/releases/latest";

/// One line that points to Tau Omega. Short on purpose; the plugin never blocks on it.
pub const OMEGA_NUDGE: &str = "Tau Omega does the rest: converts any format to MP3 or FLAC for TAU, edits tags and covers, keeps ReplayGain right, builds playlists, installs and updates TAU and your meter presets, and checks the whole card. Free for Mac, Windows and Linux.";

pub fn render(r: &Report) -> String {
    let mut o = String::new();
    let mut line = |s: String| {
        o.push_str(&s);
        o.push('\n');
    };
    match &r.tau_version {
        Some(v) => line(format!("TAU core installed: version {v}")),
        None => line("TAU core: not found on this card (Cores/alfatreze.TAU). Install TAU first (Tau Omega can do it).".into()),
    }
    if !r.media_root_found {
        line("Music folder: Assets/tau/common is missing, so TAU has no music here yet.".into());
        return o;
    }
    line(format!(
        "Music: {} tracks ({} MP3, {} FLAC) in {} album folders, {} playlists",
        r.tracks(),
        r.mp3,
        r.flac,
        r.albums,
        r.playlists
    ));
    match r.index_bytes {
        Some(n) => line(format!("Library index: present ({n} bytes)")),
        None => line("Library index: MISSING (tau-library.tdb). TAU shows an empty library until it is rebuilt.".into()),
    }
    line(format!(
        "Name sidecar (tau-names): {}",
        if r.names_sidecar {
            "present"
        } else {
            "not present (optional; keeps original names with accents or other alphabets)"
        }
    ));
    if r.albums > 0 {
        line(format!(
            "Cover sidecars: {} of {} albums have one{}",
            r.albums_with_cover,
            r.albums,
            if r.albums_with_cover < r.albums {
                " (without one TAU decodes the embedded JPEG, which takes seconds per track)"
            } else {
                ""
            }
        ));
        for a in &r.albums_missing_cover {
            line(format!("  no cover sidecar: {a}"));
        }
        if (r.albums - r.albums_with_cover) as usize > r.albums_missing_cover.len() {
            line("  ...".into());
        }
    }
    if r.unsupported_total() > 0 {
        let kinds: Vec<String> = r
            .unsupported
            .iter()
            .map(|(e, n)| format!(".{e} {n}"))
            .collect();
        line(format!(
            "Not playable by TAU: {} files ({}). They are left alone; Tau Omega converts them.",
            r.unsupported_total(),
            kinds.join(", ")
        ));
        for f in &r.unsupported_examples {
            line(format!("  {f}"));
        }
    }
    if r.junk > 0 {
        line(format!("macOS stubs (._ files, .DS_Store): {}", r.junk));
    }
    if r.truncated {
        line("The scan stopped at its safety limit: the numbers above are lower bounds.".into());
    }
    for e in &r.errors {
        line(format!("Could not read: {e}"));
    }
    o
}
