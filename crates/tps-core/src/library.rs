//! "Rebuild the library" as the user sees it: the album covers and the library index in one reviewed step.
//!
//! The covers come first, because the index records which albums have a cover file; then the library is written once.

use std::path::Path;

use crate::rebuild::Error;
use crate::{covers, rebuild};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub library: rebuild::Plan,
    pub covers: covers::Plan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub covers_written: usize,
    pub covers_failed: Vec<(String, String)>,
    pub library: rebuild::Done,
}

/// Reads the tags and the folders and says what would be written. Writes nothing.
pub fn plan(card: &Path) -> Result<Plan, Error> {
    Ok(Plan {
        library: rebuild::plan(card)?,
        covers: covers::plan(card)?,
    })
}

/// Makes the missing covers, then writes the library. A cover that fails is listed and skipped; the library is still
/// written. If the library fails, the covers already made stay (they are only new files) and the error says so.
pub fn run(card: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<Done, Error> {
    let (covers_written, covers_failed) = covers::write_all(card, progress)?;
    let library = rebuild::run(card).map_err(|e| match e {
        Error::Failed(m) if covers_written > 0 => Error::Failed(format!(
            "{m} The {covers_written} cover files made before it were kept."
        )),
        other => other,
    })?;
    Ok(Done {
        covers_written,
        covers_failed,
        library,
    })
}

pub fn render_plan(p: &Plan) -> String {
    format!(
        "{}{}",
        rebuild::render_plan(&p.library),
        covers::render_plan(&p.covers)
    )
}

pub fn render_done(d: &Done) -> String {
    let mut o = rebuild::render_done(&d.library);
    o.push_str(&format!("Cover files written: {}\n", d.covers_written));
    for (f, e) in d.covers_failed.iter().take(5) {
        o.push_str(&format!("  not made for {f}: {e}\n"));
    }
    if d.covers_failed.len() > 5 {
        o.push_str(&format!(
            "  ... and {} more not made\n",
            d.covers_failed.len() - 5
        ));
    }
    o
}

#[cfg(test)]
mod tests;
