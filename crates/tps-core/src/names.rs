//! Names the Pocket cannot open (letters outside code page 437: accents, other alphabets, emoji).
//!
//! A rebuild leaves such files out of the library. Instead the user can let them be renamed to a plain form
//! (`Café.mp3` -> `Cafe.mp3`); the original name is kept in the name sidecar, playlists that point at the file are
//! rewritten, and what changes is backed up first. All of it is Tau Omega's `refresh` (one set of rules, shared); this
//! module only asks it for the plan and runs it with a backup folder that is not on the card.

use std::path::Path;

use tau_core::refresh::{self, RefreshPlan};

use crate::rebuild::Error;
use crate::scan::MEDIA_ROOT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Files that would be renamed: (from, to), relative to the media folder.
    pub files: Vec<(String, String)>,
    /// Folders renamed as well.
    pub folders: usize,
    refresh: RefreshPlan,
}

/// The renames a rebuild could offer, or `None` when no name needs one (or the rules refuse). Writes nothing.
pub fn plan(card: &Path) -> Result<Option<Plan>, Error> {
    let root = card.join(MEDIA_ROOT);
    if !root.is_dir() {
        return Err(Error::NoMediaRoot);
    }
    let p = refresh::plan_refresh(&root).map_err(|e| Error::Failed(e.to_string()))?;
    if p.refused.is_some() || p.renames.is_empty() {
        return Ok(None);
    }
    let files: Vec<(String, String)> = p
        .findings
        .iter()
        .filter_map(|f| f.fix.as_ref().map(|to| (f.rel.clone(), to.clone())))
        .collect();
    let folders = p.renames.len().saturating_sub(files.len());
    Ok(Some(Plan {
        files,
        folders,
        refresh: p,
    }))
}

/// Backs up what changes to `backup_root` (not on the card), renames, rewrites playlists, rebuilds the index and the
/// name sidecar and verifies; any failure puts everything back. Returns how many names were changed.
pub fn run(card: &Path, plan: &Plan, backup_root: &Path) -> Result<usize, Error> {
    let root = card.join(MEDIA_ROOT);
    refresh::execute_refresh(&root, &plan.refresh, &plan.refresh.id, backup_root)
        .map(|r| r.renamed)
        .map_err(|e| Error::Failed(e.to_string()))
}
