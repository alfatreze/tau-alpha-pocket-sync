//! Cover sidecars: the small picture file TAU reads for an album, `<album>/tau-art/cover_128.pal256.timg`.
//!
//! Without it the player decodes the album's JPEG on every track change (seconds); with it the cover is one read.
//! The picture comes from the folder's image (`cover.jpg`, `folder.png`, ...) or from the art inside the first track
//! that has some. The finding and the encoding are Tau Omega's `tau-core` (one encoder, shared); this module decides
//! which albums need one, writes each file safely (temp file, read back and decoded, then renamed) and never touches
//! an existing sidecar, the music, or the original pictures. After the covers the library is rebuilt, because the
//! index tells the player which albums have a cover file.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tau_core::sync::{CoverFilePlan, plan_cover_files};
use tau_core::{cover, image};

use crate::rebuild::{self, Error};
use crate::scan::MEDIA_ROOT;

/// What making the missing covers would do, read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Albums (folders with tracks).
    pub albums: usize,
    /// Albums that already have a cover sidecar.
    pub have: usize,
    /// Albums that will get one: the folder (relative to the media root) and where the picture comes from.
    pub to_make: Vec<(String, String)>,
    /// Albums with no cover sidecar and no picture to make one from.
    pub no_picture: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub written: usize,
    /// Albums whose cover could not be made (a picture that would not decode, a write that did not verify).
    pub failed: Vec<(String, String)>,
    /// The library was rebuilt afterwards (it exists), with this many tracks.
    pub library_tracks: Option<usize>,
}

fn media_root(card: &Path) -> Result<PathBuf, Error> {
    let root = card.join(MEDIA_ROOT);
    if root.is_dir() {
        Ok(root)
    } else {
        Err(Error::NoMediaRoot)
    }
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn describe(root: &Path, plan: &CoverFilePlan) -> Plan {
    Plan {
        albums: plan.albums,
        have: plan.have_cover_file,
        to_make: plan
            .items
            .iter()
            .map(|i| {
                let from = if i.from_embedded {
                    format!("the art inside {}", rel(root, &i.cover_source))
                } else {
                    rel(root, &i.cover_source)
                };
                (rel(root, &i.source_folder), from)
            })
            .collect(),
        no_picture: plan.no_picture.clone(),
    }
}

/// Reads the folders and says which albums would get a cover sidecar. Writes nothing.
pub fn plan(card: &Path) -> Result<Plan, Error> {
    let root = media_root(card)?;
    let p = plan_cover_files(&root).map_err(|e| Error::Failed(e.to_string()))?;
    Ok(describe(&root, &p))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// One cover: the source must still be the picture that was planned, the file is written to a temp name, read back
/// and decoded as a TIM1, and only then renamed to its final name. A card pulled halfway leaves no half file there.
fn write_one(item: &tau_core::sync::ArtSidecarItem) -> Result<(), String> {
    let source = if item.from_embedded {
        cover::extract_embedded_cover(&item.cover_source)
    } else {
        fs::read(&item.cover_source).ok()
    }
    .ok_or("the picture is gone")?;
    if sha256_hex(&source) != item.cover_sha256 {
        return Err("the picture changed since the plan".into());
    }
    let packed = image::encode_pal256_bytes(&source, 128).map_err(|e| e.to_string())?;
    if let Some(dir) = item.destination.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let temp = item.destination.with_extension("tmp");
    fs::write(&temp, &packed).map_err(|e| e.to_string())?;
    let back = fs::read(&temp).map_err(|e| e.to_string())?;
    if back != packed || image::decode_tim1(&back).is_err() {
        let _ = fs::remove_file(&temp);
        return Err("the new file did not read back the same".into());
    }
    fs::rename(&temp, &item.destination).map_err(|e| {
        let _ = fs::remove_file(&temp);
        e.to_string()
    })
}

/// Makes every missing cover sidecar and nothing else. `progress(done, total)` is called after each album. A cover that
/// fails is listed and skipped; the others are still made. Returns how many were written and the albums that failed.
pub fn write_all(
    card: &Path,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(usize, Vec<(String, String)>), Error> {
    let root = media_root(card)?;
    let p = plan_cover_files(&root).map_err(|e| Error::Failed(e.to_string()))?;
    let total = p.items.len();
    let (mut written, mut failed) = (0, Vec::new());
    for (i, item) in p.items.iter().enumerate() {
        match write_one(item) {
            Ok(()) => written += 1,
            Err(e) => failed.push((rel(&root, &item.source_folder), e)),
        }
        progress(i + 1, total);
    }
    Ok((written, failed))
}

/// Makes every missing cover sidecar, then rebuilds the library when there is one (the index records which albums
/// have a cover file).
pub fn run(card: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<Done, Error> {
    let root = media_root(card)?;
    let (written, failed) = write_all(card, progress)?;
    let library_tracks = if written > 0 && root.join(rebuild::INDEX_NAME).is_file() {
        Some(rebuild::run(card)?.tracks)
    } else {
        None
    };
    Ok(Done {
        written,
        failed,
        library_tracks,
    })
}

/// The plan as short lines (the caller adds the heading).
pub fn plan_lines(p: &Plan) -> Vec<String> {
    use crate::report::plural;
    let mut o = Vec::new();
    if p.to_make.is_empty() {
        o.push("Covers    all there".into());
    } else {
        o.push(format!("Covers    {} to make", p.to_make.len()));
    }
    if !p.no_picture.is_empty() {
        o.push(format!(
            "! {} no picture: skipped",
            plural(p.no_picture.len() as u64, "album has", "albums have")
        ));
    }
    o
}

pub fn render_done(d: &Done) -> String {
    let mut o = format!("  Covers    {} written\n", d.written);
    o.push_str(&render_failed(&d.failed));
    if let Some(n) = d.library_tracks {
        o.push_str(&format!("  Library   rebuilt ({n} tracks)\n"));
    }
    o
}

pub(crate) fn render_failed(failed: &[(String, String)]) -> String {
    let mut o = String::new();
    for (f, e) in failed.iter().take(5) {
        o.push_str(&format!("  ! no cover for {f}: {e}\n"));
    }
    if failed.len() > 5 {
        o.push_str(&format!("  ! ... and {} more\n", failed.len() - 5));
    }
    o
}

#[cfg(test)]
mod tests;
