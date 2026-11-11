//! The plugin: the message protocol of the Pocket plugin reference (`openfpga-library/pocket-plugin`, MIT) and menus
//! around `tps-core`. The host maps the card as `pocket/` and a private desktop folder as `host/`.

use extism_pdk::*;
use serde::Serialize;
use tps_core::{Limits, library, report, scan};

#[derive(Clone, Serialize, ToBytes)]
#[encoding(Json)]
enum PluginMessage {
    Choice {
        name: String,
        query: String,
        choices: Vec<String>,
    },
    #[allow(dead_code)]
    Text {
        name: String,
        query: String,
    },
    Exit,
}

/// The host functions of the plugin ABI: the only unsafe code in the project.
#[allow(unsafe_code)]
mod host {
    use extism_pdk::*;

    #[host_fn]
    unsafe extern "ExtismHost" {
        fn open_url(url: &str) -> ();
        fn print_msg(message: &str) -> ();
    }

    /// print_msg is a print, not a println: the plugin owns its newlines.
    pub fn say(msg: &str) {
        unsafe {
            let _ = print_msg(&format!("{msg}\n"));
        }
    }

    pub fn open(url: &str) -> Result<(), Error> {
        unsafe { open_url(url) }
    }
}
use host::{open, say};

const CARD: &str = "pocket";
const CHECK: &str = "Check the card (read only)";
const REBUILD: &str = "Rebuild the library (shows the plan first)";
const WRITE: &str = "Write now";
const SKIP: &str = "Skip those files and write";
const CANCEL: &str = "Cancel";
/// Where a rename keeps its backup: the plugin's private folder on the computer, never on the card.
const BACKUP: &str = "host/tau-backups";
const OMEGA: &str = "↗ Get Tau Omega";

fn menu(query: &str) -> PluginMessage {
    PluginMessage::Choice {
        name: "main".into(),
        query: query.into(),
        choices: vec![CHECK.into(), REBUILD.into(), OMEGA.into()],
    }
}

#[plugin_fn]
pub fn start() -> FnResult<PluginMessage> {
    say(&format!(
        "TAU Library {}. Nothing is written without your OK.",
        env!("CARGO_PKG_VERSION")
    ));
    Ok(menu("What do you want to do?"))
}

/// There is no Exit item: the host's window stays open until the user closes it, which sends the kill signal.
/// The host sends `{"Answer":{"name":..,"value":..}}` or a kill signal (the string `"Kill"` or `{"Kill":{}}`; both are accepted).
#[plugin_fn]
pub fn handle_response(Json(msg): Json<serde_json::Value>) -> FnResult<PluginMessage> {
    let killed = msg.as_str() == Some("Kill") || msg.get("Kill").is_some();
    if killed {
        return Ok(PluginMessage::Exit);
    }
    let value = msg
        .pointer("/Answer/value")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let asked = msg
        .pointer("/Answer/name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if asked == "confirm" {
        return Ok(confirmed(value));
    }
    match value {
        CHECK => {
            say("Reading the card...");
            let r = scan(std::path::Path::new(CARD), Limits::default());
            say(&report::render(&r));
            say(report::OMEGA_NUDGE);
            Ok(menu("Anything else?"))
        }
        REBUILD => Ok(rebuild_plan()),
        OMEGA => {
            open(report::TAU_OMEGA_URL)?;
            Ok(menu("Anything else?"))
        }
        _ => Ok(menu("What do you want to do?")),
    }
}

fn confirm_menu(renames: usize) -> PluginMessage {
    let mut choices = Vec::new();
    if renames > 0 {
        choices.push(format!("Rename {renames} and write"));
        choices.push(SKIP.to_string());
    } else {
        choices.push(WRITE.to_string());
    }
    choices.push(CANCEL.to_string());
    PluginMessage::Choice {
        name: "confirm".into(),
        query: if renames > 0 {
            "Rename them (originals are kept, a backup goes to the plugin's host folder)?".into()
        } else {
            "Write now?".into()
        },
        choices,
    }
}

/// Reads the tags, folders and names and shows what a rebuild would do; nothing is written.
fn rebuild_plan() -> PluginMessage {
    say("Reading the card...");
    match library::plan(std::path::Path::new(CARD)) {
        Ok(p) => {
            say(&library::render_plan(&p));
            if p.library.tracks == 0 {
                say("No MP3 or FLAC to list.");
                menu("Anything else?")
            } else if p.nothing_to_do() {
                menu("Anything else?")
            } else {
                confirm_menu(p.renames.as_ref().map_or(0, |n| n.files.len()))
            }
        }
        Err(e) => {
            say(&e.to_string());
            menu("Anything else?")
        }
    }
}

/// The answer to the confirmation. "Rename ..." renames the files the Pocket cannot open (with a backup), "Write now"
/// and "Skip those files" leave them out, anything else writes nothing.
fn confirmed(value: &str) -> PluginMessage {
    let rename = value.starts_with("Rename ");
    if !rename && value != WRITE && value != SKIP {
        say("Nothing written.");
        return menu("Anything else?");
    }
    say("Writing. Keep the card in...");
    let mut progress = |done: usize, total: usize| {
        if done == total || done % 10 == 0 {
            say(&format!("  cover {done}/{total}"));
        }
    };
    let backup = std::path::Path::new(BACKUP);
    match library::run(
        std::path::Path::new(CARD),
        &mut progress,
        rename.then_some(backup),
    ) {
        Ok(d) => {
            say(&library::render_done(&d));
            say("Eject the card before removing it.");
            say(report::OMEGA_NUDGE);
        }
        Err(e) => say(&format!("Failed: {e}")),
    }
    menu("Anything else?")
}
