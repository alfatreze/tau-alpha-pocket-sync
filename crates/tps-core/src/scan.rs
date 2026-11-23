//! Read-only scan of a TAU card: what is on it, what TAU can play, what it cannot, what is missing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Where TAU keeps music and its sidecar files, relative to the card root.
pub const MEDIA_ROOT: &str = "Assets/tau/common";
/// The one album cover sidecar Player 2 reads, relative to an album folder.
pub const COVER_SIDECAR: &str = "tau-art/cover_128.pal256.timg";
/// The installed stable core's descriptor, relative to the card root.
pub const CORE_JSON: &str = "Cores/alfatreze.TAU/core.json";

/// The meter-settings file the Pocket writes for each TAU core (data slot 11, TAU 0.7.5 and later), relative to the card root. The Pocket creates it
/// at the first change, so a missing file is normal. `(core label, path)`.
pub const SETTINGS_FILES: [(&str, &str); 2] = [
    ("TAU", "Saves/tau/alfatreze.TAU/tau-meter-settings.bin"),
    (
        "TAU Diagnostics",
        "Saves/tau/alfatreze.TAU Diagnostics/tau-meter-settings.bin",
    ),
];
/// The one size the Pocket accepts for it (`size_exact` in TAU's data.json).
pub const SETTINGS_FILE_BYTES: u64 = 8192;

/// What a settings file on the card is, as far as the Pocket and TAU are concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsFile {
    /// 8,192 bytes, nothing odd in it.
    Ok,
    /// Any other size: the Pocket stops with "Error in framework file id [11] size bad" and TAU does not start until it is deleted.
    WrongSize(u64),
    /// 8,192 bytes with a format version newer than this plugin (and TAU 0.7.5) knows: TAU leaves it alone and keeps only two meters' settings.
    NewerVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp3,
    Flac,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// MP3 or FLAC: what TAU plays.
    Track(Format),
    /// `.m3u` / `.m3u8`.
    Playlist,
    /// A loose image next to the music.
    Image,
    /// macOS stubs: `._*`, `.DS_Store`, and the like. Never music, never copied.
    Junk,
    /// A file TAU itself writes (library index, name sidecar, cover sidecars, assets, presets).
    TauFile,
    /// An audio format TAU cannot play; needs conversion (Tau Omega). Holds the lower-case extension.
    Unsupported(String),
    /// Anything else.
    Other,
}

const UNSUPPORTED_AUDIO: &[&str] = &[
    "m4a", "aac", "alac", "ogg", "oga", "opus", "wav", "wave", "aif", "aiff", "ape", "wma", "wv",
    "mpc", "mka", "mp2", "mp1", "dsf", "dff", "ac3", "amr",
];
const IMAGES: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff"];

/// Classifies one file by its name and the folder names above it (relative to the media root, `/` separated).
pub fn classify(rel: &str) -> Kind {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if name.starts_with("._") || name == ".DS_Store" || name == "Thumbs.db" || name == "desktop.ini"
    {
        return Kind::Junk;
    }
    if rel.starts_with("tau-presets/") || rel.contains("/tau-art/") || rel.starts_with("tau-art/") {
        return Kind::TauFile;
    }
    if matches!(
        name,
        "tau-library.tdb" | "tau-names.json" | "tau-names.bin" | "tau-assets.bin"
    ) {
        return Kind::TauFile;
    }
    let ext = match name.rsplit_once('.') {
        Some((stem, e)) if !stem.is_empty() => e.to_ascii_lowercase(),
        _ => return Kind::Other,
    };
    match ext.as_str() {
        "mp3" => Kind::Track(Format::Mp3),
        "flac" => Kind::Track(Format::Flac),
        "m3u" | "m3u8" => Kind::Playlist,
        e if IMAGES.contains(&e) => Kind::Image,
        e if UNSUPPORTED_AUDIO.contains(&e) => Kind::Unsupported(ext),
        _ => Kind::Other,
    }
}

/// Bounds so a huge or damaged card cannot make a scan run away.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_entries: usize,
    pub max_depth: usize,
    /// How many example paths to keep per list in the report.
    pub examples: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entries: 300_000,
            max_depth: 24,
            examples: 12,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The installed stable TAU core's version, when the card has one.
    pub tau_version: Option<String>,
    /// The settings files that exist, by core label (a missing file is normal and not listed).
    pub settings_files: Vec<(String, SettingsFile)>,
    /// Whether `Assets/tau/common` exists.
    pub media_root_found: bool,
    pub mp3: u64,
    pub flac: u64,
    pub playlists: u64,
    pub images: u64,
    pub junk: u64,
    pub junk_examples: Vec<String>,
    /// Unsupported audio, by extension.
    pub unsupported: BTreeMap<String, u64>,
    pub unsupported_examples: Vec<String>,
    pub other: u64,
    /// Folders holding at least one MP3 or FLAC.
    pub albums: u64,
    pub albums_with_cover: u64,
    pub albums_missing_cover: Vec<String>,
    /// Size in bytes of `tau-library.tdb`, if present.
    pub index_bytes: Option<u64>,
    pub names_sidecar: bool,
    /// The scan stopped at a limit; the counts are lower bounds.
    pub truncated: bool,
    /// Folders or files that could not be read.
    pub errors: Vec<String>,
}

impl Report {
    pub fn tracks(&self) -> u64 {
        self.mp3 + self.flac
    }
    pub fn unsupported_total(&self) -> u64 {
        self.unsupported.values().sum()
    }
}

fn push_example(list: &mut Vec<String>, rel: &str, cap: usize) {
    if list.len() < cap {
        list.push(rel.to_string());
    }
}

/// Looks at the meter-settings file of each TAU core: its size, and the version field of its two 4,096-byte generations (bytes 4 and 5 of each,
/// after the magic `TMSF`). Reads at most 16 bytes of it; never writes.
fn check_settings(card: &Path) -> Vec<(String, SettingsFile)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut out = Vec::new();
    for (core, rel) in SETTINGS_FILES {
        let path = card.join(rel);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        if meta.len() != SETTINGS_FILE_BYTES {
            out.push((core.to_string(), SettingsFile::WrongSize(meta.len())));
            continue;
        }
        let mut newer = false;
        if let Ok(mut f) = fs::File::open(&path) {
            for bank in [0u64, 4096] {
                let mut h = [0u8; 8];
                if f.seek(SeekFrom::Start(bank)).is_ok()
                    && f.read_exact(&mut h).is_ok()
                    && &h[0..4] == b"TMSF"
                    && u16::from_le_bytes([h[4], h[5]]) > 1
                {
                    newer = true;
                }
            }
        }
        out.push((
            core.to_string(),
            if newer {
                SettingsFile::NewerVersion
            } else {
                SettingsFile::Ok
            },
        ));
    }
    out
}

/// Scans `card` (the card root). Never writes, never follows symlinks.
pub fn scan(card: &Path, limits: Limits) -> Report {
    let mut r = Report::default();
    if let Ok(text) = fs::read_to_string(card.join(CORE_JSON)) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            r.tau_version = v
                .pointer("/core/metadata/version")
                .and_then(|x| x.as_str())
                .map(str::to_string);
        }
    }
    r.settings_files = check_settings(card);
    let root = card.join(MEDIA_ROOT);
    if !root.is_dir() {
        return r;
    }
    r.media_root_found = true;
    r.index_bytes = fs::metadata(root.join("tau-library.tdb"))
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len());
    r.names_sidecar = root.join("tau-names.json").is_file() && root.join("tau-names.bin").is_file();

    let mut entries = 0usize;
    // (directory, relative path with `/`, depth)
    let mut stack: Vec<(PathBuf, String, usize)> = vec![(root.clone(), String::new(), 0)];
    while let Some((dir, rel_dir, depth)) = stack.pop() {
        let read = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => {
                push_example(
                    &mut r.errors,
                    &format!("{}: {e}", if rel_dir.is_empty() { "." } else { &rel_dir }),
                    limits.examples,
                );
                continue;
            }
        };
        let mut items: Vec<_> = read.filter_map(Result::ok).collect();
        items.sort_by_key(|e| e.file_name());
        let mut has_track = false;
        let mut subdirs = Vec::new();
        for it in items {
            entries += 1;
            if entries > limits.max_entries {
                r.truncated = true;
                break;
            }
            let Ok(ft) = it.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let name = it.file_name().to_string_lossy().into_owned();
            let rel = if rel_dir.is_empty() {
                name.clone()
            } else {
                format!("{rel_dir}/{name}")
            };
            if ft.is_dir() {
                if depth + 1 > limits.max_depth {
                    r.truncated = true;
                } else if name == "tau-art" || name == "tau-presets" {
                    // sidecar folders: counted through their parent, never scanned for music
                } else {
                    subdirs.push((it.path(), rel, depth + 1));
                }
                continue;
            }
            match classify(&rel) {
                Kind::Track(Format::Mp3) => {
                    r.mp3 += 1;
                    has_track = true;
                }
                Kind::Track(Format::Flac) => {
                    r.flac += 1;
                    has_track = true;
                }
                Kind::Playlist => r.playlists += 1,
                Kind::Image => r.images += 1,
                Kind::Junk => {
                    r.junk += 1;
                    push_example(&mut r.junk_examples, &rel, limits.examples);
                }
                Kind::TauFile => {}
                Kind::Unsupported(ext) => {
                    *r.unsupported.entry(ext).or_insert(0) += 1;
                    push_example(&mut r.unsupported_examples, &rel, limits.examples);
                }
                Kind::Other => r.other += 1,
            }
        }
        if has_track {
            r.albums += 1;
            if dir.join(COVER_SIDECAR).is_file() {
                r.albums_with_cover += 1;
            } else {
                push_example(
                    &mut r.albums_missing_cover,
                    if rel_dir.is_empty() {
                        "(music root)"
                    } else {
                        &rel_dir
                    },
                    limits.examples,
                );
            }
        }
        if r.truncated && entries > limits.max_entries {
            break;
        }
        // keep a stable, alphabetical walk
        subdirs.reverse();
        stack.extend(subdirs);
    }
    r
}

#[cfg(test)]
mod tests;
