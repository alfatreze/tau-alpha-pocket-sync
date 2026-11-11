use super::*;
use std::fs;

fn touch(root: &Path, rel: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, b"x").unwrap();
}

#[test]
fn classify_by_name() {
    assert_eq!(classify("A/b.MP3"), Kind::Track(Format::Mp3));
    assert_eq!(classify("A/b.flac"), Kind::Track(Format::Flac));
    assert_eq!(classify("list.m3u8"), Kind::Playlist);
    assert_eq!(classify("A/cover.JPG"), Kind::Image);
    assert_eq!(classify("A/._b.mp3"), Kind::Junk);
    assert_eq!(classify(".DS_Store"), Kind::Junk);
    assert_eq!(classify("A/b.m4a"), Kind::Unsupported("m4a".into()));
    assert_eq!(classify("A/b.OPUS"), Kind::Unsupported("opus".into()));
    assert_eq!(classify("tau-library.tdb"), Kind::TauFile);
    assert_eq!(classify("A/tau-art/cover_128.pal256.timg"), Kind::TauFile);
    assert_eq!(classify("tau-presets/12/p1.tpr"), Kind::TauFile);
    assert_eq!(classify("A/notes.txt"), Kind::Other);
    assert_eq!(classify("A/noext"), Kind::Other);
    assert_eq!(
        classify("A/.mp3"),
        Kind::Other,
        "a dot file named like an extension is not a track"
    );
}

#[test]
fn empty_card_and_missing_root() {
    let d = tempfile::tempdir().unwrap();
    let r = scan(d.path(), Limits::default());
    assert!(!r.media_root_found && r.tau_version.is_none() && r.tracks() == 0);
}

#[test]
fn counts_albums_covers_unsupported_and_junk() {
    let d = tempfile::tempdir().unwrap();
    let c = d.path();
    touch(c, "Cores/alfatreze.TAU/core.json");
    fs::write(
        c.join("Cores/alfatreze.TAU/core.json"),
        br#"{"core":{"metadata":{"version":"0.7.0"}}}"#,
    )
    .unwrap();
    touch(c, "Assets/tau/common/tau-library.tdb");
    touch(c, "Assets/tau/common/Album One/01.mp3");
    touch(c, "Assets/tau/common/Album One/02.flac");
    touch(
        c,
        "Assets/tau/common/Album One/tau-art/cover_128.pal256.timg",
    );
    touch(c, "Assets/tau/common/Album One/._01.mp3");
    touch(c, "Assets/tau/common/Album Two/a.mp3");
    touch(c, "Assets/tau/common/Album Two/b.m4a");
    touch(c, "Assets/tau/common/Album Two/c.OGG");
    touch(c, "Assets/tau/common/Mixed.m3u");
    touch(c, "Assets/tau/common/Album Three/only.wav");
    let r = scan(c, Limits::default());
    assert_eq!(r.tau_version.as_deref(), Some("0.7.0"));
    assert!(r.media_root_found && r.index_bytes == Some(1) && !r.names_sidecar);
    assert_eq!((r.mp3, r.flac, r.playlists, r.junk), (2, 1, 1, 1));
    assert_eq!(r.albums, 2, "Album Three has no playable track");
    assert_eq!(r.albums_with_cover, 1);
    assert_eq!(r.albums_missing_cover, vec!["Album Two".to_string()]);
    assert_eq!(r.unsupported.get("m4a"), Some(&1));
    assert_eq!(r.unsupported.get("ogg"), Some(&1));
    assert_eq!(r.unsupported_total(), 3);
    assert!(!r.truncated && r.errors.is_empty());
}

#[test]
fn sidecar_folders_are_not_scanned_for_music() {
    let d = tempfile::tempdir().unwrap();
    touch(d.path(), "Assets/tau/common/tau-presets/12/leftover.mp3");
    let r = scan(d.path(), Limits::default());
    assert_eq!(r.tracks(), 0);
}

#[test]
fn limits_stop_the_scan_and_say_so() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..30 {
        touch(d.path(), &format!("Assets/tau/common/A/{i:02}.mp3"));
    }
    let r = scan(
        d.path(),
        Limits {
            max_entries: 10,
            max_depth: 24,
            examples: 5,
        },
    );
    assert!(r.truncated && r.tracks() <= 10);
    let r = scan(
        d.path(),
        Limits {
            max_entries: 1000,
            max_depth: 0,
            examples: 5,
        },
    );
    assert!(
        r.truncated,
        "a folder below the depth limit is reported as truncated"
    );
}

#[test]
fn report_text_covers_the_important_cases() {
    let d = tempfile::tempdir().unwrap();
    let c = d.path();
    touch(c, "Assets/tau/common/X/1.mp3");
    touch(c, "Assets/tau/common/X/2.wma");
    let t = crate::report::render(&scan(c, Limits::default()));
    assert!(t.contains("! TAU core not found"));
    assert!(t.contains("! Library index missing"));
    assert!(t.contains("! Album covers: 0 of 1 have one"));
    assert!(t.contains("! Not playable by TAU: 1 file (.wma 1)"));
    assert!(
        crate::report::render(&Report::default())
            .contains("Music folder missing: Assets/tau/common")
    );
}

#[cfg(unix)]
#[test]
fn symlinks_are_never_followed() {
    let d = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    touch(outside.path(), "x/1.mp3");
    fs::create_dir_all(d.path().join("Assets/tau/common")).unwrap();
    std::os::unix::fs::symlink(outside.path(), d.path().join("Assets/tau/common/link")).unwrap();
    assert_eq!(scan(d.path(), Limits::default()).tracks(), 0);
}
