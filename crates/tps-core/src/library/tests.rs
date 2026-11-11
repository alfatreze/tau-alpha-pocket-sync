use super::*;
use std::fs;

const FRAME: [u8; 4] = [0xff, 0xfb, 0x90, 0x64];

fn mp3(title: &str) -> Vec<u8> {
    let mut body = vec![3u8];
    body.extend_from_slice(title.as_bytes());
    let mut f = b"TIT2".to_vec();
    f.extend_from_slice(&[0, 0, 0, body.len() as u8, 0, 0]);
    f.extend(body);
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend_from_slice(&[0, 0, (f.len() >> 7) as u8 & 127, f.len() as u8 & 127]);
    out.extend(f);
    for _ in 0..30 {
        out.extend_from_slice(&FRAME);
        out.extend(std::iter::repeat_n(0u8, 413));
    }
    out
}

fn jpeg() -> Vec<u8> {
    let data: Vec<u8> = (0..48u16)
        .flat_map(|y| (0..64u16).flat_map(move |x| [(x * 4) as u8, 200, (y * 4) as u8]))
        .collect();
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 90)
        .encode(&data, 64, 48, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

/// Two albums: one with a folder picture, one without any.
fn card() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("Assets/tau/common");
    for a in ["With Picture", "No Picture"] {
        fs::create_dir_all(c.join(a)).unwrap();
        fs::write(c.join(a).join("01 t.mp3"), mp3(a)).unwrap();
    }
    fs::write(c.join("With Picture/cover.jpg"), jpeg()).unwrap();
    dir
}

#[test]
fn the_plan_shows_the_library_and_the_covers_and_writes_nothing() {
    let dir = card();
    let p = plan(dir.path()).unwrap();
    assert_eq!((p.library.tracks, p.covers.to_make.len()), (2, 1));
    let text = render_plan(&p);
    assert!(
        text.contains("2 tracks in 2 albums") && text.contains("1 would get one"),
        "{text}"
    );
    assert!(
        !dir.path()
            .join("Assets/tau/common/tau-library.tdb")
            .exists()
    );
    assert!(
        !dir.path()
            .join("Assets/tau/common/With Picture/tau-art")
            .exists()
    );
}

#[test]
fn one_run_writes_the_covers_and_the_library_and_the_library_knows_the_covers() {
    let dir = card();
    let mut seen = Vec::new();
    let d = run(dir.path(), &mut |i, n| seen.push((i, n))).unwrap();
    assert_eq!((d.covers_written, d.library.tracks), (1, 2));
    assert_eq!(seen, [(1, 1)]);
    let c = dir.path().join("Assets/tau/common");
    assert!(c.join("tau-library.tdb").is_file());
    assert!(
        c.join("With Picture/tau-art/cover_128.pal256.timg")
            .is_file()
    );
    assert!(!c.join("No Picture/tau-art").exists());
    let text = render_done(&d);
    assert!(
        text.contains("Library written: 2 tracks") && text.contains("Cover files written: 1"),
        "{text}"
    );
    // Nothing is left to do: a second run writes no cover and gives the same library bytes.
    let first = fs::read(c.join("tau-library.tdb")).unwrap();
    let again = run(dir.path(), &mut |_, _| {}).unwrap();
    assert_eq!(again.covers_written, 0);
    assert_eq!(first, fs::read(c.join("tau-library.tdb")).unwrap());
}

#[test]
fn a_bad_picture_does_not_stop_the_library() {
    let dir = card();
    fs::write(
        dir.path().join("Assets/tau/common/With Picture/cover.jpg"),
        b"not a picture",
    )
    .unwrap();
    let d = run(dir.path(), &mut |_, _| {}).unwrap();
    assert_eq!(
        (d.covers_written, d.covers_failed.len(), d.library.tracks),
        (0, 1, 2)
    );
    assert!(render_done(&d).contains("not made for With Picture"));
}

#[test]
fn no_media_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(plan(dir.path()), Err(Error::NoMediaRoot));
    assert!(matches!(
        run(dir.path(), &mut |_, _| {}),
        Err(Error::NoMediaRoot)
    ));
}
