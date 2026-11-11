//! Plain-text rendering of a scan for the plugin's log window (one line per fact, no colour, no markup).

use crate::scan::Report;

pub const TAU_OMEGA_URL: &str = "https://github.com/alfatreze/Tau-Omega/releases/latest";

/// One line that points to Tau Omega. Short on purpose; the plugin never blocks on it.
pub const OMEGA_NUDGE: &str = "More (convert, tags, presets): ↗ Get Tau Omega";

/// A section heading as a visible rule (the log window of the hosts drops blank lines, so the spacing has to be drawn):
/// `── CARD ──────────────────`.
pub fn header(title: &str) -> String {
    let used = title.chars().count() + 4;
    format!("── {title} {}", "─".repeat(34usize.saturating_sub(used)))
}

/// "1 track", "3 tracks".
pub fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The Check text, as short as it can be: the facts, then only what needs doing. Plain text for a log window (the
/// plugin protocol has no bold, colour or style): `!` marks a problem, `-` a list item, `->` the menu item that fixes it.
pub fn render(r: &Report) -> String {
    let mut card: Vec<String> = Vec::new();
    let mut todo: Vec<String> = Vec::new();
    match &r.tau_version {
        Some(v) => card.push(format!("TAU core  {v}")),
        None => todo.push("! TAU not installed (Tau Omega can do it)".into()),
    }
    if !r.media_root_found {
        todo.push("! No music folder: copy MP3 and FLAC albums to Assets/tau/common".into());
    } else {
        card.push(format!(
            "Music     {} ({} MP3, {} FLAC), {}",
            plural(r.tracks(), "track", "tracks"),
            r.mp3,
            r.flac,
            plural(r.albums, "album", "albums")
        ));
        let mut fix = false;
        match r.index_bytes {
            Some(_) => card.push("Library   there".into()),
            None => {
                card.push("Library   missing".into());
                todo.push("! No library: TAU shows nothing".into());
                fix = true;
            }
        }
        if r.albums > 0 {
            if r.albums_with_cover >= r.albums {
                card.push("Covers    all there".into());
            } else {
                card.push(format!("Covers    {} of {}", r.albums_with_cover, r.albums));
                todo.push(format!(
                    "! {} no cover: slow track changes",
                    plural(r.albums - r.albums_with_cover, "album has", "albums have")
                ));
                fix = true;
            }
        }
        if fix {
            todo.push("  -> Rebuild the library".into());
        }
        if r.unsupported_total() > 0 {
            let kinds: Vec<String> = r.unsupported.keys().map(|e| format!(".{e}")).collect();
            todo.push(format!(
                "! {} TAU can't play ({}); Tau Omega converts",
                plural(r.unsupported_total(), "file", "files"),
                kinds.join(" ")
            ));
        }
    }
    if r.junk > 0 {
        card.push(format!("Stubs     {} macOS junk files", r.junk));
    }
    if r.truncated {
        todo.push("! Scan stopped at its limit: counts are lower bounds".into());
    }
    for e in &r.errors {
        todo.push(format!("! Could not read: {e}"));
    }
    let mut o = String::new();
    if !card.is_empty() {
        o.push_str(&format!("{}\n", header("CARD")));
        for l in &card {
            o.push_str(&format!("  {l}\n"));
        }
    }
    if todo.is_empty() {
        o.push_str(&format!("{}\n  Nothing to do.\n", header("ALL GOOD")));
    } else {
        o.push_str(&format!("{}\n", header("TO DO")));
        for l in &todo {
            o.push_str(&format!("  {l}\n"));
        }
    }
    o
}
