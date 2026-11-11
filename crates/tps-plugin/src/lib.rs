//! The plugin: the message protocol of the Pocket plugin reference (`openfpga-library/pocket-plugin`, MIT) and menus
//! around `tps-core`. The host maps the card as `pocket/` and a private desktop folder as `host/`.

use extism_pdk::*;
use serde::Serialize;
use tps_core::{Limits, covers, library, report, scan};

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
const COVERS: &str = "Make album covers only (shows the plan first)";
const WRITE_COVERS: &str = "Make the covers now";
const WRITE: &str = "Write the library and covers now";
const CANCEL: &str = "Cancel, write nothing";
const OMEGA: &str = "Get Tau Omega (the full tool)";

fn menu(query: &str) -> PluginMessage {
    PluginMessage::Choice {
        name: "main".into(),
        query: query.into(),
        choices: vec![CHECK.into(), REBUILD.into(), COVERS.into(), OMEGA.into()],
    }
}

#[plugin_fn]
pub fn start() -> FnResult<PluginMessage> {
    say(&format!(
        "TAU Library {}: reads the card; writes only when you say so, and only the library files.",
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
    if asked == "confirm-covers" {
        return Ok(covers_confirmed(value));
    }
    match value {
        CHECK => {
            say("Scanning the card...");
            let r = scan(std::path::Path::new(CARD), Limits::default());
            say(&report::render(&r));
            say(report::OMEGA_NUDGE);
            Ok(menu("Anything else?"))
        }
        REBUILD => Ok(rebuild_plan()),
        COVERS => Ok(covers_plan()),
        OMEGA => {
            open(report::TAU_OMEGA_URL)?;
            Ok(menu("Anything else?"))
        }
        _ => Ok(menu("What do you want to do?")),
    }
}

fn confirm_menu() -> PluginMessage {
    PluginMessage::Choice {
        name: "confirm".into(),
        query: "Write the library now?".into(),
        choices: vec![WRITE.into(), CANCEL.into()],
    }
}

/// Reads the tags and the folders and shows what a rebuild would do (the library and the album covers); nothing is written.
fn rebuild_plan() -> PluginMessage {
    say("Reading the music, its tags and the album folders (nothing is written yet)...");
    match library::plan(std::path::Path::new(CARD)) {
        Ok(p) => {
            say(&library::render_plan(&p));
            if p.library.tracks == 0 {
                say(
                    "There is no playable track (MP3 or FLAC) to list, so there is nothing to write.",
                );
                menu("Anything else?")
            } else {
                confirm_menu()
            }
        }
        Err(e) => {
            say(&e.to_string());
            menu("Anything else?")
        }
    }
}

/// The answer to the confirmation: make the covers and write the library, or leave everything as it is.
fn confirmed(value: &str) -> PluginMessage {
    if value != WRITE {
        say("Nothing was written.");
        return menu("Anything else?");
    }
    say("Writing the covers and the library. Please keep the card in place...");
    let mut progress = |done: usize, total: usize| {
        if done == total || done % 10 == 0 {
            say(&format!("  cover {done} of {total}"));
        }
    };
    match library::run(std::path::Path::new(CARD), &mut progress) {
        Ok(d) => {
            say(&library::render_done(&d));
            say("Eject the card from your computer before you take it out.");
            say(report::OMEGA_NUDGE);
        }
        Err(e) => say(&format!("The library was not changed by this step: {e}")),
    }
    menu("Anything else?")
}

fn covers_plan() -> PluginMessage {
    say("Looking for albums without a cover file (nothing is written yet)...");
    match covers::plan(std::path::Path::new(CARD)) {
        Ok(p) => {
            say(&covers::render_plan(&p));
            if p.to_make.is_empty() {
                say("Nothing to make.");
                menu("Anything else?")
            } else {
                PluginMessage::Choice {
                    name: "confirm-covers".into(),
                    query: "Make these cover files now?".into(),
                    choices: vec![WRITE_COVERS.into(), CANCEL.into()],
                }
            }
        }
        Err(e) => {
            say(&e.to_string());
            menu("Anything else?")
        }
    }
}

fn covers_confirmed(value: &str) -> PluginMessage {
    if value != WRITE_COVERS {
        say("Nothing was written.");
        return menu("Anything else?");
    }
    say("Making the cover files. Please keep the card in place...");
    let mut progress = |done: usize, total: usize| {
        if done == total || done % 10 == 0 {
            say(&format!("  {done} of {total}"));
        }
    };
    match covers::run(std::path::Path::new(CARD), &mut progress) {
        Ok(d) => {
            say(&covers::render_done(&d));
            say("Eject the card from your computer before you take it out.");
            say(report::OMEGA_NUDGE);
        }
        Err(e) => say(&format!("The covers were not changed by this step: {e}")),
    }
    menu("Anything else?")
}
