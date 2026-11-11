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
    answer_to(run, "main", value)
}

fn answer_to(run: &mut Run, name: &str, value: &str) -> String {
    let msg = serde_json::json!({"Answer": {"name": name, "value": value}}).to_string();
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
            "Rebuild the library (shows the plan first)",
            "↗ Get Tau Omega"
        ]
    );
    assert!(
        run.log
            .lock()
            .unwrap()
            .contains("Nothing is written without your OK")
    );
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
    assert!(log.contains("1 track (1 MP3, 0 FLAC), 1 album"), "{log}");
    assert!(log.contains("! No library"), "{log}");
    assert!(log.contains("! 1 file TAU can't play (.m4a)"), "{log}");
    assert!(log.contains("Get Tau Omega"), "{log}");
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
    answer(&mut run, "↗ Get Tau Omega");
    assert_eq!(
        run.urls.lock().unwrap().as_slice(),
        ["https://github.com/alfatreze/Tau-Omega/releases/latest"]
    );
}

#[test]
fn both_kill_encodings_end_the_run() {
    let mut run = load(|_| {});
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

// ---- Rebuild the library ----

fn syncsafe(n: usize) -> [u8; 4] {
    [
        (n >> 21) as u8 & 127,
        (n >> 14) as u8 & 127,
        (n >> 7) as u8 & 127,
        n as u8 & 127,
    ]
}

/// A tiny valid MP3: an ID3v2.3 tag and silent frames.
fn mp3(title: &str, artist: &str, album: &str) -> Vec<u8> {
    let mut frames = Vec::new();
    for (id, t) in [("TIT2", title), ("TPE1", artist), ("TALB", album)] {
        let mut body = vec![3u8];
        body.extend_from_slice(t.as_bytes());
        frames.extend_from_slice(id.as_bytes());
        frames.extend_from_slice(&syncsafe(body.len()));
        frames.extend_from_slice(&[0, 0]);
        frames.extend(body);
    }
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend_from_slice(&syncsafe(frames.len()));
    out.extend(frames);
    for _ in 0..40 {
        out.extend_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        out.extend(std::iter::repeat_n(0u8, 413));
    }
    out
}

fn music(card: &Path) {
    for (dir, n, title, artist) in [
        ("Album One", 2, "Song", "Ann"),
        ("Album Two", 1, "Tune", "Bob"),
    ] {
        for i in 1..=n {
            let p = card.join(format!("Assets/tau/common/{dir}/{i:02} t.mp3"));
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, mp3(&format!("{title} {i}"), artist, dir)).unwrap();
        }
    }
}

const INDEX: &str = "Assets/tau/common/tau-library.tdb";
const REBUILD: &str = "Rebuild the library (shows the plan first)";

#[test]
fn rebuild_shows_a_plan_and_writes_nothing_until_confirmed() {
    let mut run = load(music);
    let before = walk(run._card.path());
    let out = answer(&mut run, REBUILD);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["Choice"]["name"], "confirm");
    let log = run.log.lock().unwrap().clone();
    assert!(log.contains("none -> 3 tracks, 2 albums"), "{log}");
    assert_eq!(walk(run._card.path()), before, "the plan wrote something");
    let out = answer_to(&mut run, "confirm", "Cancel");
    assert!(out.contains("\"main\""));
    assert_eq!(walk(run._card.path()), before, "cancel wrote something");
}

#[test]
fn confirming_writes_the_same_library_as_the_native_build() {
    let mut run = load(music);
    answer(&mut run, REBUILD);
    answer_to(&mut run, "confirm", "Write now");
    let log = run.log.lock().unwrap().clone();
    assert!(log.contains("Library   3 tracks, 2 albums"), "{log}");
    let wasm = fs::read(run._card.path().join(INDEX)).expect("the plugin wrote the index");
    // The same music, built natively: the WebAssembly build must give the same bytes.
    let native = tempfile::tempdir().unwrap();
    music(native.path());
    tps_core::rebuild::run(native.path()).unwrap();
    assert_eq!(wasm, fs::read(native.path().join(INDEX)).unwrap());
    // The music itself was not touched.
    let tracks: Vec<_> = walk(run._card.path())
        .into_iter()
        .filter(|e| e.contains(".mp3"))
        .collect();
    assert_eq!(tracks.len(), 3);
}

#[test]
fn rebuild_without_a_tau_folder_says_so_and_writes_nothing() {
    let mut run = load(|_| {});
    let out = answer(&mut run, REBUILD);
    assert!(out.contains("\"main\""), "back to the menu");
    assert!(
        run.log
            .lock()
            .unwrap()
            .contains("There is no Assets/tau/common folder")
    );
    assert!(walk(run._card.path()).is_empty());
}

// ---- Album covers ----

fn jpeg(w: u16, h: u16) -> Vec<u8> {
    let data: Vec<u8> = (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| [(x * 4) as u8, 200, (y * 4) as u8]))
        .collect();
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 90)
        .encode(&data, w, h, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

fn music_with_covers(card: &Path) {
    music(card);
    fs::write(
        card.join("Assets/tau/common/Album One/cover.jpg"),
        jpeg(64, 48),
    )
    .unwrap();
}

const SIDECAR: &str = "Assets/tau/common/Album One/tau-art/cover_128.pal256.timg";

#[test]
fn rebuild_includes_the_covers_and_matches_the_native_build() {
    let mut run = load(music_with_covers);
    answer(&mut run, REBUILD);
    let plan_log = run.log.lock().unwrap().clone();
    assert!(
        plan_log.contains("1 to make"),
        "the plan names the covers: {plan_log}"
    );
    answer_to(&mut run, "confirm", "Write now");
    let log = run.log.lock().unwrap().clone();
    assert!(log.contains("Library   3 tracks"), "{log}");
    assert!(log.contains("Covers    1 written"), "{log}");
    let (wasm_cover, wasm_index) = (
        fs::read(run._card.path().join(SIDECAR)).unwrap(),
        fs::read(run._card.path().join(INDEX)).unwrap(),
    );
    let native = tempfile::tempdir().unwrap();
    music_with_covers(native.path());
    tps_core::library::run(native.path(), &mut |_, _| {}, None).unwrap();
    assert_eq!(wasm_cover, fs::read(native.path().join(SIDECAR)).unwrap());
    assert_eq!(wasm_index, fs::read(native.path().join(INDEX)).unwrap());
}

#[test]
fn a_name_the_pocket_cannot_open_can_be_renamed_with_a_backup_in_the_host_folder() {
    let mut run = load(|c| {
        music(c);
        fs::write(
            c.join("Assets/tau/common/Album One/Łódź.mp3"),
            mp3("Lodz", "Ann", "Album One"),
        )
        .unwrap();
    });
    let out = answer(&mut run, REBUILD);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let choices: Vec<String> = v["Choice"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        choices,
        ["Rename 1 and write", "Skip those files and write", "Cancel"]
    );
    let log = run.log.lock().unwrap().clone();
    assert!(log.contains("1 file name can't be opened"), "{log}");
    answer_to(&mut run, "confirm", "Rename 1 and write");
    let log = run.log.lock().unwrap().clone();
    assert!(log.contains("Renamed   1 name"), "{log}");
    let card = run._card.path().join("Assets/tau/common/Album One");
    assert!(card.join("Lódz.mp3").is_file() && !card.join("Łódź.mp3").exists());
    assert!(
        fs::read_dir(run._host.path().join("tau-backups"))
            .unwrap()
            .count()
            > 0,
        "the backup is in the host folder"
    );
    assert!(
        !walk(run._card.path())
            .iter()
            .any(|e| e.contains("tau-backups")),
        "no backup on the card"
    );
}

#[test]
fn skipping_leaves_the_file_alone() {
    let mut run = load(|c| {
        music(c);
        fs::write(
            c.join("Assets/tau/common/Album One/Łódź.mp3"),
            mp3("Lodz", "Ann", "Album One"),
        )
        .unwrap();
    });
    answer(&mut run, REBUILD);
    answer_to(&mut run, "confirm", "Skip those files and write");
    let card = run._card.path().join("Assets/tau/common/Album One");
    assert!(card.join("Łódź.mp3").is_file());
    assert!(fs::read(run._card.path().join(INDEX)).is_ok());
    assert!(run.log.lock().unwrap().contains("Library   3 tracks"));
}
