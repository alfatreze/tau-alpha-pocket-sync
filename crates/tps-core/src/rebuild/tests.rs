use super::*;
use std::fs;

const FRAME: [u8; 4] = [0xff, 0xfb, 0x90, 0x64];

fn syncsafe(n: usize) -> [u8; 4] {
    [
        (n >> 21) as u8 & 127,
        (n >> 14) as u8 & 127,
        (n >> 7) as u8 & 127,
        n as u8 & 127,
    ]
}

fn text_frame(id: &str, text: &str) -> Vec<u8> {
    let mut body = vec![3u8];
    body.extend_from_slice(text.as_bytes());
    let mut f = id.as_bytes().to_vec();
    f.extend_from_slice(&syncsafe(body.len()));
    f.extend_from_slice(&[0, 0]);
    f.extend(body);
    f
}

/// A tiny valid MP3: an ID3v2.3 tag (UTF-8 text frames) and silent frames.
fn mp3(title: &str, artist: &str, album: &str, track: &str) -> Vec<u8> {
    let mut frames = Vec::new();
    for (id, t) in [
        ("TIT2", title),
        ("TPE1", artist),
        ("TALB", album),
        ("TRCK", track),
    ] {
        if !t.is_empty() {
            frames.extend(text_frame(id, t));
        }
    }
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend_from_slice(&syncsafe(frames.len()));
    out.extend(frames);
    for _ in 0..40 {
        out.extend_from_slice(&FRAME);
        out.extend(std::iter::repeat_n(0u8, 413));
    }
    out
}

fn card() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let common = dir.path().join("Assets/tau/common");
    fs::create_dir_all(common.join("Album One")).unwrap();
    fs::create_dir_all(common.join("Album Two")).unwrap();
    fs::write(
        common.join("Album One/01 First.mp3"),
        mp3("First", "Ann", "Album One", "1"),
    )
    .unwrap();
    fs::write(
        common.join("Album One/02 Second.mp3"),
        mp3("Second", "Ann", "Album One", "2"),
    )
    .unwrap();
    fs::write(
        common.join("Album Two/01 Third.mp3"),
        mp3("Third", "Bob", "Album Two", "1"),
    )
    .unwrap();
    dir
}

fn index_of(card: &Path) -> Vec<u8> {
    fs::read(card.join(MEDIA_ROOT).join(INDEX_NAME)).unwrap()
}

#[test]
fn plan_counts_and_writes_nothing() {
    let dir = card();
    let p = plan(dir.path()).unwrap();
    assert_eq!((p.tracks, p.albums, p.current_tracks), (3, 2, None));
    assert!(!dir.path().join(MEDIA_ROOT).join(INDEX_NAME).exists());
    assert!(render_plan(&p).contains("3 tracks in 2 albums"));
}

#[test]
fn run_writes_a_valid_library_that_matches_the_files() {
    let dir = card();
    let d = run(dir.path()).unwrap();
    assert_eq!((d.tracks, d.albums), (3, 2));
    let bytes = index_of(dir.path());
    let ix = parse(&bytes).unwrap();
    assert_eq!(ix.counts.tracks, 3);
    let root = dir.path().join(MEDIA_ROOT);
    assert!(verify(&bytes, Some(&root)).unwrap().is_empty());
    let leftovers: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    assert_eq!(plan(dir.path()).unwrap().current_tracks, Some(3));
}

#[test]
fn a_second_run_gives_the_same_bytes() {
    let dir = card();
    run(dir.path()).unwrap();
    let first = index_of(dir.path());
    run(dir.path()).unwrap();
    assert_eq!(first, index_of(dir.path()));
}

#[test]
fn names_the_pocket_cannot_open_are_left_out_and_listed() {
    let dir = card();
    let common = dir.path().join(MEDIA_ROOT);
    fs::write(
        common.join("Album Two/日本語.mp3"),
        mp3("Nihon", "Bob", "Album Two", "2"),
    )
    .unwrap();
    let p = plan(dir.path()).unwrap();
    assert_eq!(p.tracks, 3);
    assert_eq!(p.unreachable.len(), 1);
    assert!(render_plan(&p).contains("cannot open"));
    assert_eq!(run(dir.path()).unwrap().tracks, 3);
}

#[test]
fn a_run_with_no_tracks_leaves_the_old_library_alone() {
    let dir = card();
    run(dir.path()).unwrap();
    let before = index_of(dir.path());
    let common = dir.path().join(MEDIA_ROOT);
    fs::remove_dir_all(common.join("Album One")).unwrap();
    fs::remove_dir_all(common.join("Album Two")).unwrap();
    assert!(matches!(run(dir.path()), Err(Error::Failed(_))));
    assert_eq!(before, index_of(dir.path()));
}

#[test]
fn no_media_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(plan(dir.path()), Err(Error::NoMediaRoot));
    assert_eq!(run(dir.path()), Err(Error::NoMediaRoot));
}

#[test]
fn music_files_are_never_changed() {
    let dir = card();
    let common = dir.path().join(MEDIA_ROOT);
    let before = fs::read(common.join("Album One/01 First.mp3")).unwrap();
    run(dir.path()).unwrap();
    assert_eq!(
        before,
        fs::read(common.join("Album One/01 First.mp3")).unwrap()
    );
}
