//! "Rebuild the library" as the user sees it: one reviewed step that does only what is needed.
//!
//! The name sidecar is part of the library (written with the index whenever names need it). The album covers are made
//! first, because the index records which albums have a cover file. Files whose names the Pocket cannot open are
//! skipped unless the caller asks for them to be renamed (see `names`).

use std::path::Path;

use crate::rebuild::Error;
use crate::report::{header, plural};
use crate::{covers, names, rebuild};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub library: rebuild::Plan,
    pub covers: covers::Plan,
    /// Files that could be renamed so the Pocket can open them (asked, never done unasked).
    pub renames: Option<names::Plan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub renamed: usize,
    pub covers_written: usize,
    pub covers_failed: Vec<(String, String)>,
    /// `None` when the library was already what a rebuild would write and no cover was made.
    pub library: Option<rebuild::Done>,
}

impl Plan {
    /// Nothing to write at all: the library is current, every album that can have a cover has one, no name to fix.
    pub fn nothing_to_do(&self) -> bool {
        self.library.up_to_date && self.covers.to_make.is_empty() && self.renames.is_none()
    }
}

/// Reads the tags, the folders and the names and says what would be written. Writes nothing.
pub fn plan(card: &Path) -> Result<Plan, Error> {
    Ok(Plan {
        library: rebuild::plan(card)?,
        covers: covers::plan(card)?,
        renames: names::plan(card)?,
    })
}

/// Does only what is needed. With `rename_backup` (a folder that is not on the card) the files with names the Pocket
/// cannot open are renamed first, with a backup of what changes; without it they are left out. Then the missing
/// covers are made, then the library is written when it is not current or a cover was made (the index changes with
/// the covers). A cover that fails is listed and skipped. If the library fails, the covers already made stay.
pub fn run(
    card: &Path,
    progress: &mut dyn FnMut(usize, usize),
    rename_backup: Option<&Path>,
) -> Result<Done, Error> {
    let mut renamed = 0;
    if let Some(backup) = rename_backup
        && let Some(np) = names::plan(card)?
    {
        renamed = names::run(card, &np, backup)?;
    }
    let before = rebuild::plan(card)?;
    let (covers_written, covers_failed) = covers::write_all(card, progress)?;
    if before.up_to_date && covers_written == 0 {
        return Ok(Done {
            renamed,
            covers_written,
            covers_failed,
            library: None,
        });
    }
    let library = rebuild::run(card).map_err(|e| match e {
        Error::Failed(m) if covers_written > 0 => Error::Failed(format!(
            "{m} The {covers_written} covers made before it were kept."
        )),
        other => other,
    })?;
    Ok(Done {
        renamed,
        covers_written,
        covers_failed,
        library: Some(library),
    })
}

/// The plan as short lines under one heading.
pub fn render_plan(p: &Plan) -> String {
    if p.nothing_to_do() {
        return format!("{}\n  Nothing to do.\n", header("ALL GOOD"));
    }
    let mut lines = rebuild::plan_lines(&p.library, p.renames.is_some());
    lines.extend(covers::plan_lines(&p.covers));
    if let Some(n) = &p.renames {
        lines.push(format!(
            "! {} can't be opened by the Pocket:",
            plural(n.files.len() as u64, "file name", "file names")
        ));
        for (from, to) in n.files.iter().take(3) {
            lines.push(format!("    {from} -> {to}"));
        }
        if n.files.len() > 3 {
            lines.push(format!("    ... and {} more", n.files.len() - 3));
        }
    }
    let mut o = format!("{}\n", header("PLAN"));
    for l in lines {
        o.push_str(&format!("  {l}\n"));
    }
    o
}

pub fn render_done(d: &Done) -> String {
    let mut o = format!("{}\n", header("DONE"));
    if d.renamed > 0 {
        o.push_str(&format!(
            "  Renamed   {}\n",
            plural(d.renamed as u64, "name", "names")
        ));
    }
    match &d.library {
        Some(l) => {
            for line in rebuild::done_lines(l) {
                o.push_str(&format!("  {line}\n"));
            }
        }
        None => o.push_str("  Library   up to date\n"),
    }
    o.push_str(&format!("  Covers    {} written\n", d.covers_written));
    o.push_str(&covers::render_failed(&d.covers_failed));
    o
}

#[cfg(test)]
mod tests;
