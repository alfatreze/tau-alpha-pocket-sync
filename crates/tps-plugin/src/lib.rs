//! The plugin: the message protocol of the Pocket plugin reference (`openfpga-library/pocket-plugin`, MIT) and menus
//! around `tps-core`. The host maps the card as `pocket/` and a private desktop folder as `host/`.

use extism_pdk::*;
use serde::Serialize;
use tps_core::{Limits, report, scan};

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
const OMEGA: &str = "Get Tau Omega (the full tool)";
const EXIT: &str = "Exit";

fn menu(query: &str) -> PluginMessage {
    PluginMessage::Choice {
        name: "main".into(),
        query: query.into(),
        choices: vec![CHECK.into(), OMEGA.into(), EXIT.into()],
    }
}

#[plugin_fn]
pub fn start() -> FnResult<PluginMessage> {
    say(&format!(
        "TAU Library {}: reads the card, writes nothing.",
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
    match value {
        CHECK => {
            say("Scanning the card...");
            let r = scan(std::path::Path::new(CARD), Limits::default());
            say(&report::render(&r));
            say(report::OMEGA_NUDGE);
            Ok(menu("Anything else?"))
        }
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
