//! The plugin: the message protocol of the Pocket plugin reference (`openfpga-library/pocket-plugin`, MIT) and menus
//! around `tps-core`. The host maps the card as `pocket/` and a private desktop folder as `host/`.

use extism_pdk::*;
use serde::Serialize;
use tps_core::{Limits, rebuild, report, scan};

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
const WRITE: &str = "Write the library now";
const CANCEL: &str = "Cancel, write nothing";
const OMEGA: &str = "Get Tau Omega (the full tool)";
const EXIT: &str = "Exit";

fn menu(query: &str) -> PluginMessage {
    PluginMessage::Choice {
        name: "main".into(),
        query: query.into(),
        choices: vec![CHECK.into(), REBUILD.into(), OMEGA.into(), EXIT.into()],
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
            say("Scanning the card...");
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
        EXIT => {
            say(report::OMEGA_NUDGE);
            Ok(PluginMessage::Exit)
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

/// Reads the tags and shows what a rebuild would do; nothing is written.
fn rebuild_plan() -> PluginMessage {
    say("Reading the music and its tags (nothing is written yet)...");
    match rebuild::plan(std::path::Path::new(CARD)) {
        Ok(p) => {
            say(&rebuild::render_plan(&p));
            if p.tracks == 0 {
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

/// The answer to the confirmation: write the library, or leave everything as it is.
fn confirmed(value: &str) -> PluginMessage {
    if value != WRITE {
        say("Nothing was written.");
        return menu("Anything else?");
    }
    say("Writing the library. Please keep the card in place...");
    match rebuild::run(std::path::Path::new(CARD)) {
        Ok(d) => {
            say(&rebuild::render_done(&d));
            say("Eject the card from your computer before you take it out.");
            say(report::OMEGA_NUDGE);
        }
        Err(e) => say(&format!("The library was not changed by this step: {e}")),
    }
    menu("Anything else?")
}
