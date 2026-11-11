//! Needs the wasm: `cargo build -p tps-plugin --target wasm32-wasip1 --release` (CI does this first).

use extism::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn wasm_path() -> PathBuf {
    if let Ok(p) = std::env::var("TPS_WASM") {
        return PathBuf::from(p);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/wasm32-wasip1/release/tps_plugin.wasm")
}

struct Run {
    plugin: Plugin,
    log: Arc<Mutex<String>>,
    urls: Arc<Mutex<Vec<String>>>,
    _card: tempfile::TempDir,
    _host: tempfile::TempDir,
}

fn touch(root: &Path, rel: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, b"x").unwrap();
}

fn load(fill: impl Fn(&Path)) -> Run {
    let wasm = wasm_path();
    assert!(
        wasm.exists(),
        "build the plugin first: cargo build -p tps-plugin --target wasm32-wasip1 --release ({})",
        wasm.display()
    );
    let card = tempfile::tempdir().unwrap();
    let host = tempfile::tempdir().unwrap();
    fill(card.path());
    let log = Arc::new(Mutex::new(String::new()));
    let urls = Arc::new(Mutex::new(Vec::new()));
    // The same two mounts the real hosts give a plugin: the card as `pocket`, a private desktop folder as `host`.
    let manifest = Manifest::new([Wasm::file(wasm)])
        .with_allowed_path(card.path().to_string_lossy().to_string(), "pocket")
        .with_allowed_path(host.path().to_string_lossy().to_string(), "host");
    let plugin = PluginBuilder::new(manifest)
        .with_wasi(true)
        .with_function(
            "print_msg",
            [PTR],
            [PTR],
            UserData::new(log.clone()),
            |plugin: &mut CurrentPlugin,
             inputs: &[Val],
             out: &mut [Val],
             ud: UserData<Arc<Mutex<String>>>| {
                let s: String = plugin.memory_get_val(&inputs[0])?;
                let shared = ud.get()?;
                shared.lock().unwrap().lock().unwrap().push_str(&s);
                out[0] = Val::I64(0); // the plugin ABI declares an offset result; a print has nothing to return
                Ok(())
            },
        )
        .with_function(
            "open_url",
            [PTR],
            [PTR],
            UserData::new(urls.clone()),
            |plugin: &mut CurrentPlugin,
             inputs: &[Val],
             out: &mut [Val],
             ud: UserData<Arc<Mutex<Vec<String>>>>| {
                let s: String = plugin.memory_get_val(&inputs[0])?;
                let shared = ud.get()?;
                shared.lock().unwrap().lock().unwrap().push(s);
                out[0] = Val::I64(0);
                Ok(())
            },
        )
        .build()
        .expect("the plugin loads");
    Run {
        plugin,
        log,
        urls,
        _card: card,
        _host: host,
    }
}

fn answer(run: &mut Run, value: &str) -> String {
    let msg = serde_json::json!({"Answer": {"name": "main", "value": value}}).to_string();
    run.plugin
        .call::<&str, String>("handle_response", &msg)
        .expect("handle_response")
}

#[test]
fn start_shows_the_menu() {
    let mut run = load(|_| {});
    let out: String = run.plugin.call("start", "").unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["Choice"]["name"], "main");
    let choices: Vec<_> = v["Choice"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        choices,
        [
            "Check the card (read only)",
            "Get Tau Omega (the full tool)",
            "Exit"
        ]
    );
    assert!(run.log.lock().unwrap().contains("writes nothing"));
}

#[test]
fn check_reports_the_card_and_nudges_to_omega() {
    let mut run = load(|c| {
        touch(c, "Assets/tau/common/Album/01.mp3");
        touch(c, "Assets/tau/common/Album/02.m4a");
    });
    let out = answer(&mut run, "Check the card (read only)");
    assert!(out.contains("Choice"), "the menu comes back");
    let log = run.log.lock().unwrap().clone();
    assert!(
        log.contains("Music: 1 tracks (1 MP3, 0 FLAC) in 1 album folders"),
        "{log}"
    );
    assert!(log.contains("Library index: MISSING"), "{log}");
    assert!(
        log.contains("Not playable by TAU: 1 files (.m4a 1)"),
        "{log}"
    );
    assert!(log.contains("Tau Omega does the rest"), "{log}");
}

#[test]
fn the_card_is_never_written() {
    let mut run = load(|c| touch(c, "Assets/tau/common/A/1.mp3"));
    let before: Vec<_> = walk(run._card.path());
    answer(&mut run, "Check the card (read only)");
    assert_eq!(walk(run._card.path()), before);
}

fn walk(root: &Path) -> Vec<String> {
    let mut v = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            v.push(format!(
                "{}:{}",
                p.strip_prefix(root).unwrap().display(),
                e.metadata().unwrap().len()
            ));
            if p.is_dir() {
                stack.push(p);
            }
        }
    }
    v.sort();
    v
}

#[test]
fn omega_item_opens_the_releases_page() {
    let mut run = load(|_| {});
    answer(&mut run, "Get Tau Omega (the full tool)");
    assert_eq!(
        run.urls.lock().unwrap().as_slice(),
        ["https://github.com/alfatreze/Tau-Omega/releases/latest"]
    );
}

#[test]
fn exit_and_both_kill_encodings_end_the_run() {
    let mut run = load(|_| {});
    assert_eq!(answer(&mut run, "Exit"), "\"Exit\"");
    assert!(run.log.lock().unwrap().contains("Tau Omega does the rest"));
    for kill in ["\"Kill\"", "{\"Kill\":{}}"] {
        let out: String = run.plugin.call("handle_response", kill).unwrap();
        assert_eq!(out, "\"Exit\"", "{kill}");
    }
}

#[test]
fn an_unknown_answer_shows_the_menu_again() {
    let mut run = load(|_| {});
    assert!(answer(&mut run, "nonsense").contains("Choice"));
}
