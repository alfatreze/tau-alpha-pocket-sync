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
        text.contains("none -> 2 tracks, 2 albums") && text.contains("1 to make"),
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
    let d = run(dir.path(), &mut |i, n| seen.push((i, n)), None).unwrap();
    assert_eq!(
        (d.covers_written, d.library.as_ref().unwrap().tracks),
        (1, 2)
    );
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
        text.contains("Library   2 tracks") && text.contains("Covers    1 written"),
        "{text}"
    );
    // Nothing is left to do: a second run writes no cover and gives the same library bytes.
    let first = fs::read(c.join("tau-library.tdb")).unwrap();
    let again = run(dir.path(), &mut |_, _| {}, None).unwrap();
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
    let d = run(dir.path(), &mut |_, _| {}, None).unwrap();
    assert_eq!(
        (
            d.covers_written,
            d.covers_failed.len(),
            d.library.as_ref().unwrap().tracks
        ),
        (0, 1, 2)
    );
    assert!(render_done(&d).contains("! no cover for With Picture"));
}

#[test]
fn no_media_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(plan(dir.path()), Err(Error::NoMediaRoot));
    assert!(matches!(
        run(dir.path(), &mut |_, _| {}, None),
        Err(Error::NoMediaRoot)
    ));
}

#[test]
fn a_current_library_with_every_cover_made_means_nothing_to_do() {
    let dir = card();
    run(dir.path(), &mut |_, _| {}, None).unwrap();
    let c = dir.path().join("Assets/tau/common");
    let (index, cover) = (
        fs::read(c.join("tau-library.tdb")).unwrap(),
        fs::read(c.join("With Picture/tau-art/cover_128.pal256.timg")).unwrap(),
    );
    let p = plan(dir.path()).unwrap();
    assert!(p.nothing_to_do(), "{}", render_plan(&p));
    assert!(render_plan(&p).contains("Nothing to do"));
    let d = run(dir.path(), &mut |_, _| {}, None).unwrap();
    assert_eq!((d.covers_written, d.library.is_none()), (0, true));
    assert!(render_done(&d).contains("up to date"));
    assert_eq!(index, fs::read(c.join("tau-library.tdb")).unwrap());
    assert_eq!(
        cover,
        fs::read(c.join("With Picture/tau-art/cover_128.pal256.timg")).unwrap()
    );
}

#[test]
fn when_only_pictures_changed_the_library_is_current_before_and_rewritten_after_the_covers() {
    // The index records which albums have a cover file, so making covers changes it: the plan says the library is
    // current, the covers are missing, and the run rewrites the index so the player sees them.
    let dir = card();
    rebuild::run(dir.path()).unwrap();
    let c = dir.path().join("Assets/tau/common");
    let before = fs::read(c.join("tau-library.tdb")).unwrap();
    let p = plan(dir.path()).unwrap();
    assert!(p.library.up_to_date && !p.nothing_to_do());
    let text = render_plan(&p);
    assert!(
        text.contains("up to date") && text.contains("1 to make"),
        "{text}"
    );
    let d = run(dir.path(), &mut |_, _| {}, None).unwrap();
    assert_eq!(d.covers_written, 1);
    assert!(d.library.is_some());
    assert_ne!(before, fs::read(c.join("tau-library.tdb")).unwrap());
    assert!(plan(dir.path()).unwrap().nothing_to_do());
}

#[test]
fn a_changed_track_makes_the_library_stale_again() {
    let dir = card();
    run(dir.path(), &mut |_, _| {}, None).unwrap();
    let c = dir.path().join("Assets/tau/common");
    fs::write(c.join("No Picture/02 new.mp3"), mp3("New")).unwrap();
    let p = plan(dir.path()).unwrap();
    assert!(!p.library.up_to_date && !p.nothing_to_do());
}

/// A file whose name the Pocket cannot open (letters outside code page 437).
fn card_with_odd_name() -> tempfile::TempDir {
    let dir = card();
    let c = dir.path().join("Assets/tau/common");
    fs::write(c.join("No Picture/Łódź.mp3"), mp3("Lodz")).unwrap();
    dir
}

#[test]
fn a_name_the_pocket_cannot_open_is_offered_for_renaming_and_skipped_if_not() {
    let dir = card_with_odd_name();
    let p = plan(dir.path()).unwrap();
    let n = p.renames.as_ref().expect("a rename is offered");
    assert_eq!(n.files.len(), 1);
    assert!(
        n.files[0].0.ends_with("Łódź.mp3") && n.files[0].1.ends_with("Lódz.mp3"),
        "{:?}",
        n.files
    );
    let text = render_plan(&p);
    assert!(
        text.contains("1 file name can't be opened") && text.contains("-> "),
        "{text}"
    );
    assert!(!p.nothing_to_do());
    // Without the rename choice the file is left out and untouched.
    let d = run(dir.path(), &mut |_, _| {}, None).unwrap();
    assert_eq!((d.renamed, d.library.as_ref().unwrap().tracks), (0, 2));
    assert!(
        dir.path()
            .join("Assets/tau/common/No Picture/Łódź.mp3")
            .is_file()
    );
}

#[test]
fn renaming_backs_up_outside_the_card_keeps_the_original_and_lists_the_file() {
    let dir = card_with_odd_name();
    let backup = tempfile::tempdir().unwrap();
    let d = run(dir.path(), &mut |_, _| {}, Some(backup.path())).unwrap();
    assert_eq!(d.renamed, 1);
    let c = dir.path().join("Assets/tau/common");
    assert!(c.join("No Picture/Lódz.mp3").is_file());
    assert!(!c.join("No Picture/Łódź.mp3").exists());
    assert_eq!(d.library.as_ref().map(|l| l.tracks), Some(3));
    let sidecar = fs::read_to_string(c.join("tau-names.json")).unwrap();
    assert!(
        sidecar.contains("Łódź"),
        "the original name is kept: {sidecar}"
    );
    assert!(
        fs::read_dir(backup.path()).unwrap().count() > 0,
        "a backup was made outside the card"
    );
    assert!(render_done(&d).contains("Renamed   1 name"));
    let after = plan(dir.path()).unwrap();
    assert!(
        after.nothing_to_do(),
        "{} {:?} {:?}",
        after.library.up_to_date,
        after.covers.to_make,
        after.renames.as_ref().map(|r| r.files.clone())
    );
}
