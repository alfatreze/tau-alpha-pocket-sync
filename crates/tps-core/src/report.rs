//! Plain-text rendering of a scan for the plugin's log window (one line per fact, no colour, no markup).

use crate::scan::Report;

pub const TAU_OMEGA_URL: &str = "https://github.com/alfatreze/Tau-Omega/releases/latest";

/// One line that points to Tau Omega. Short on purpose; the plugin never blocks on it.
pub const OMEGA_NUDGE: &str = "Tau Omega does the rest: converts any format to MP3 or FLAC for TAU, edits tags and covers, keeps ReplayGain right, builds playlists, installs and updates TAU and your meter presets, and checks the whole card. Free for Mac, Windows and Linux.";

/// "1 track", "3 tracks".
pub fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The Check text: what is on the card, then what needs doing (each with the menu item that fixes it), then optional
/// things. Plain text for a log window: short lines, blank lines between sections, `!` for a problem, `-` for a list
/// item or a note. No colour, no markup (the plugin protocol has none).
pub fn render(r: &Report) -> String {
    let mut o = String::new();
    let mut card: Vec<String> = Vec::new();
    let mut todo: Vec<Vec<String>> = Vec::new();
    let mut optional: Vec<String> = Vec::new();

    match &r.tau_version {
        Some(v) => card.push(format!("TAU core:  {v}")),
        None => todo.push(vec![
            "! TAU core not found (Cores/alfatreze.TAU)".into(),
            "    Install TAU first (Tau Omega can do it).".into(),
        ]),
    }
    if !r.media_root_found {
        todo.push(vec![
            "! Music folder missing: Assets/tau/common".into(),
            "    TAU has no music here yet. Copy your MP3 and FLAC albums into it,".into(),
            "    then run Rebuild the library.".into(),
        ]);
    } else {
        card.push(format!(
            "Music:     {} ({} MP3, {} FLAC)",
            plural(r.tracks(), "track", "tracks"),
            r.mp3,
            r.flac
        ));
        card.push(format!(
            "           {}, {}",
            plural(r.albums, "album", "albums"),
            plural(r.playlists, "playlist", "playlists")
        ));
        match r.index_bytes {
            Some(n) => card.push(format!("Library:   present ({n} bytes)")),
            None => todo.push(vec![
                "! Library index missing".into(),
                "    TAU shows an empty library until it is rebuilt.".into(),
                "    -> Rebuild the library".into(),
            ]),
        }
        if r.albums > 0 {
            if r.albums_with_cover >= r.albums {
                card.push(format!(
                    "Covers:    all {} have one",
                    plural(r.albums, "album", "albums")
                ));
            } else {
                let mut v = vec![
                    format!(
                        "! Album covers: {} of {} have one",
                        r.albums_with_cover, r.albums
                    ),
                    "    Without one, TAU decodes the album JPEG on every track change (seconds)."
                        .into(),
                    "    -> Rebuild the library (or Make album covers only)".into(),
                ];
                for a in &r.albums_missing_cover {
                    v.push(format!("    - {a}"));
                }
                let more = (r.albums - r.albums_with_cover) as usize;
                if more > r.albums_missing_cover.len() {
                    v.push(format!(
                        "    - ... and {} more",
                        more - r.albums_missing_cover.len()
                    ));
                }
                todo.push(v);
            }
        }
        if r.unsupported_total() > 0 {
            let kinds: Vec<String> = r
                .unsupported
                .iter()
                .map(|(e, n)| format!(".{e} {n}"))
                .collect();
            let mut v = vec![
                format!(
                    "! Not playable by TAU: {} ({})",
                    plural(r.unsupported_total(), "file", "files"),
                    kinds.join(", ")
                ),
                "    Left alone. Tau Omega converts them to MP3 or FLAC.".into(),
            ];
            for f in &r.unsupported_examples {
                v.push(format!("    - {f}"));
            }
            todo.push(v);
        }
        if !r.names_sidecar {
            optional.push("- Name sidecar not present. It keeps the original names".into());
            optional
                .push("  with accents or other alphabets (made by Rebuild the library).".into());
        }
        if r.junk > 0 {
            optional.push(format!("- macOS stubs (._ files, .DS_Store): {}", r.junk));
        }
    }
    if r.truncated {
        optional.push(
            "- The scan stopped at its safety limit: the numbers above are lower bounds.".into(),
        );
    }
    for e in &r.errors {
        optional.push(format!("- Could not read: {e}"));
    }

    if !card.is_empty() {
        o.push_str("CARD\n");
        for l in &card {
            o.push_str(&format!("  {l}\n"));
        }
    }
    if todo.is_empty() {
        o.push_str("\nEVERYTHING TAU NEEDS IS IN PLACE\n");
    } else {
        o.push_str("\nNEEDS DOING\n");
        for block in &todo {
            for l in block {
                o.push_str(&format!("  {l}\n"));
            }
        }
    }
    if !optional.is_empty() {
        o.push_str("\nOPTIONAL\n");
        for l in &optional {
            o.push_str(&format!("  {l}\n"));
        }
    }
    o
}
