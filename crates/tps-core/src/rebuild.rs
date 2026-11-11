//! Rebuild the TAU library (`tau-library.tdb` and the name sidecar) from the music already on the card.
//!
//! The index and the name sidecar are made by Tau Omega's `tau-core` (one builder, shared; Omega message 016, D-023).
//! This module only decides which files go in, and writes safely: the new index goes to a temp file, is read back and
//! parsed, then swapped in with the old one kept until the new one is in place. Nothing is renamed or deleted among
//! the user's music: a file the Pocket cannot open (a name outside code page 437, or a path over 200 bytes) is left
//! out and listed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tau_core::name_sidecar::{BIN_NAME, JSON_NAME};
use tau_core::{card_name, name_sidecar, parse, root_prefix, scan_dir, verify};

use crate::scan::MEDIA_ROOT;

pub const INDEX_NAME: &str = "tau-library.tdb";
const TEMP_NAME: &str = ".tau-library.tdb.tmp";
const PREVIOUS_NAME: &str = ".tau-library.tdb.prev";

/// What a rebuild would do, read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub tracks: usize,
    pub albums: usize,
    pub playlists: usize,
    /// Files the Pocket cannot open (a character outside code page 437): left out.
    pub unreachable: Vec<String>,
    /// Files whose path is over the 200-byte budget: left out.
    pub too_long: Vec<String>,
    /// Tracks with no title or artist tag (the file name is used).
    pub missing_tags: usize,
    /// Warnings from the tag reader, as text (capped).
    pub notes: Vec<String>,
    /// The library on the card now: tracks in it, or `None` when there is none or it does not parse.
    pub current_tracks: Option<u16>,
}

/// What a rebuild did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub tracks: usize,
    pub albums: usize,
    pub playlists: usize,
    pub index_bytes: usize,
    pub name_records: usize,
    pub name_removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `Assets/tau/common` is not on the card.
    NoMediaRoot,
    /// Something went wrong (text for the user). The old library is untouched unless the text says otherwise.
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoMediaRoot => write!(f, "There is no {MEDIA_ROOT} folder on the card."),
            Error::Failed(m) => write!(f, "{m}"),
        }
    }
}

fn fail(e: impl std::fmt::Display) -> Error {
    Error::Failed(e.to_string())
}

fn media_root(card: &Path) -> Result<PathBuf, Error> {
    let root = card.join(MEDIA_ROOT);
    if root.is_dir() {
        Ok(root)
    } else {
        Err(Error::NoMediaRoot)
    }
}

/// The tracks that go into the index, with the files left out.
struct Selection {
    scan: tau_core::Scan,
    entries: Vec<tau_core::Entry>,
    playlists: Vec<tau_core::Playlist>,
    prefix: String,
    unreachable: Vec<String>,
    too_long: Vec<String>,
}

fn select(root: &Path) -> Result<Selection, Error> {
    let prefix = root_prefix(root).map_err(fail)?;
    let scan = scan_dir(root, true).map_err(fail)?;
    let (mut entries, mut unreachable, mut too_long) = (Vec::new(), Vec::new(), Vec::new());
    for e in &scan.entries {
        if !card_name::reachable(&e.rel) {
            unreachable.push(e.rel.clone());
        } else if prefix.len() + e.rel.len() > card_name::MAX_PATH {
            too_long.push(e.rel.clone());
        } else {
            entries.push(e.clone());
        }
    }
    let position: BTreeMap<&str, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.rel.as_str(), i))
        .collect();
    let playlists = scan
        .playlists
        .iter()
        .map(|p| {
            let mut p = p.clone();
            p.rel_ids = p
                .rel_ids
                .iter()
                .filter_map(|&i| position.get(scan.entries.get(i)?.rel.as_str()).copied())
                .collect();
            p
        })
        .collect();
    Ok(Selection {
        scan,
        entries,
        playlists,
        prefix,
        unreachable,
        too_long,
    })
}

fn current_tracks(root: &Path) -> Option<u16> {
    let bytes = fs::read(root.join(INDEX_NAME)).ok()?;
    parse(&bytes).ok().map(|ix| ix.counts.tracks)
}

/// Reads the tags and says what a rebuild would do. Writes nothing.
pub fn plan(card: &Path) -> Result<Plan, Error> {
    let root = media_root(card)?;
    let s = select(&root)?;
    let albums: BTreeSet<&str> = s.entries.iter().map(|e| e.dir.as_str()).collect();
    let missing_tags = s
        .entries
        .iter()
        .filter(|e| {
            !(e.tags.contains_key("TIT2")
                && (e.tags.contains_key("TPE1") || e.tags.contains_key("TPE2")))
        })
        .count();
    Ok(Plan {
        tracks: s.entries.len(),
        albums: albums.len(),
        playlists: s.playlists.len(),
        unreachable: s.unreachable,
        too_long: s.too_long,
        missing_tags,
        notes: s
            .scan
            .warnings
            .iter()
            .take(20)
            .map(|w| w.message.clone())
            .collect(),
        current_tracks: current_tracks(&root),
    })
}

/// Writes the new `tau-library.tdb` and the name sidecar. The old library is kept until the new one is read back,
/// parsed and swapped in; a failure before the swap leaves it as it was.
pub fn run(card: &Path) -> Result<Done, Error> {
    let root = media_root(card)?;
    let s = select(&root)?;
    if s.entries.is_empty() {
        return Err(Error::Failed(
            "No playable track (MP3 or FLAC) was found, so nothing was written.".into(),
        ));
    }
    let (index, names) =
        tau_core::build_index_with_names(&s.entries, &s.playlists, &s.prefix, &mut Vec::new())
            .map_err(fail)?;
    let parsed = parse(&index).map_err(fail)?;
    let missing = verify(&index, Some(&root)).map_err(fail)?;
    if !missing.is_empty() {
        return Err(Error::Failed(format!(
            "The new library does not match the files ({} missing); nothing was written.",
            missing.len()
        )));
    }
    swap_in(&root, &index)?;
    let outcome = name_sidecar::write_sidecar(&root, &index, &names, &BTreeMap::new());
    let (name_records, name_removed) = match outcome {
        Ok(o) => (o.records, o.removed),
        Err(e) => {
            return Err(Error::Failed(format!(
                "The library was written, but the name sidecar was not ({}). The Pocket shows the index text.",
                e.message
            )));
        }
    };
    Ok(Done {
        tracks: usize::from(parsed.counts.tracks),
        albums: usize::from(parsed.counts.albums),
        playlists: s.playlists.len(),
        index_bytes: index.len(),
        name_records,
        name_removed,
    })
}

/// Temp file, read back, then swap: the old index is renamed to a `.prev` file first (an exFAT card cannot rename over
/// an existing file), the new one is renamed in, and the `.prev` is removed. If the second rename fails, the old
/// index is put back.
fn swap_in(root: &Path, index: &[u8]) -> Result<(), Error> {
    let (temp, live, prev) = (
        root.join(TEMP_NAME),
        root.join(INDEX_NAME),
        root.join(PREVIOUS_NAME),
    );
    fs::write(&temp, index).map_err(fail)?;
    if fs::read(&temp).map_err(fail)? != index {
        let _ = fs::remove_file(&temp);
        return Err(Error::Failed(
            "The new library did not read back the same; the old one is untouched.".into(),
        ));
    }
    let _ = fs::remove_file(&prev);
    let had_old = live.exists();
    if had_old {
        fs::rename(&live, &prev).map_err(fail)?;
    }
    if let Err(e) = fs::rename(&temp, &live) {
        if had_old {
            let _ = fs::rename(&prev, &live);
        }
        let _ = fs::remove_file(&temp);
        return Err(Error::Failed(format!(
            "Could not put the new library in place ({e}); the old one was put back."
        )));
    }
    let _ = fs::remove_file(&prev);
    Ok(())
}

/// The generated files a rebuild owns (for tests and the report).
pub const GENERATED: [&str; 3] = [INDEX_NAME, JSON_NAME, BIN_NAME];

/// Plain text for the plan.
pub fn render_plan(p: &Plan) -> String {
    let mut o = String::new();
    o.push_str(&format!(
        "Library on the card now: {}\n",
        match p.current_tracks {
            Some(n) => format!("{n} tracks"),
            None => "none (or unreadable)".into(),
        }
    ));
    o.push_str(&format!(
        "A rebuild would list: {} tracks in {} albums, {} playlists\n",
        p.tracks, p.albums, p.playlists
    ));
    if p.missing_tags > 0 {
        o.push_str(&format!(
            "  {} tracks have no title or artist tag (the file name is used)\n",
            p.missing_tags
        ));
    }
    for (title, list) in [
        (
            "left out: the Pocket cannot open these names",
            &p.unreachable,
        ),
        ("left out: path over 200 bytes", &p.too_long),
    ] {
        if !list.is_empty() {
            o.push_str(&format!("  {} {title}\n", list.len()));
            for f in list.iter().take(5) {
                o.push_str(&format!("    {f}\n"));
            }
        }
    }
    for n in &p.notes {
        o.push_str(&format!("  note: {n}\n"));
    }
    o.push_str("Your music files are never renamed, moved or deleted.\n");
    o
}

pub fn render_done(d: &Done) -> String {
    format!(
        "Library written: {} tracks, {} albums, {} playlists ({} bytes). Name sidecar: {}.\n",
        d.tracks,
        d.albums,
        d.playlists,
        d.index_bytes,
        if d.name_removed {
            "none needed (removed an old one)".to_string()
        } else if d.name_records == 0 {
            "none needed".to_string()
        } else {
            format!("{} records", d.name_records)
        }
    )
}

#[cfg(test)]
mod tests;
