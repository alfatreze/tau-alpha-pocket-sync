use super::*;
use std::fs;

/// A small valid PNG (a colour gradient) to use as a cover.
fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        let data: Vec<u8> = (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| [(x * 4) as u8, (y * 4) as u8, 128]))
            .collect();
        writer.write_image_data(&data).unwrap();
    }
    out
}

/// A small valid baseline JPEG (the art that sits inside a track).
fn jpeg_bytes(w: u16, h: u16) -> Vec<u8> {
    let data: Vec<u8> = (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| [(x * 4) as u8, 200, (y * 4) as u8]))
        .collect();
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 90)
        .encode(&data, w, h, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

const FRAME: [u8; 4] = [0xff, 0xfb, 0x90, 0x64];

fn mp3(title: &str) -> Vec<u8> {
    let mut body = vec![3u8];
    body.extend_from_slice(title.as_bytes());
    let n = body.len();
    let mut f = b"TIT2".to_vec();
    f.extend_from_slice(&[0, 0, 0, n as u8, 0, 0]);
    f.extend(body);
    let m = f.len();
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend_from_slice(&[0, 0, (m >> 7) as u8 & 127, m as u8 & 127]);
    out.extend(f);
    for _ in 0..30 {
        out.extend_from_slice(&FRAME);
        out.extend(std::iter::repeat_n(0u8, 413));
    }
    out
}

/// Album A has a folder picture, album B has art inside its track, album C has nothing, album D already has a cover.
fn card() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("Assets/tau/common");
    for a in ["A", "B", "C", "D"] {
        fs::create_dir_all(c.join(a)).unwrap();
        fs::write(c.join(a).join("01 t.mp3"), mp3(a)).unwrap();
    }
    fs::write(c.join("A/cover.png"), png_bytes(64, 48)).unwrap();
    let pic = c.join("B/embed-source.jpg");
    fs::write(&pic, jpeg_bytes(48, 64)).unwrap();
    let with_art = cover::embed_mp3_bytes(&c.join("B/01 t.mp3"), &pic).unwrap();
    fs::write(c.join("B/01 t.mp3"), with_art).unwrap();
    fs::remove_file(pic).unwrap();
    fs::create_dir_all(c.join("D/tau-art")).unwrap();
    fs::write(c.join("D/tau-art/cover_128.pal256.timg"), b"existing").unwrap();
    dir
}

fn sidecar(card: &Path, album: &str) -> PathBuf {
    card.join(MEDIA_ROOT)
        .join(album)
        .join("tau-art/cover_128.pal256.timg")
}

#[test]
fn the_plan_names_each_album_and_writes_nothing() {
    let dir = card();
    let before = fs::read_dir(dir.path().join(MEDIA_ROOT).join("A"))
        .unwrap()
        .count();
    let p = plan(dir.path()).unwrap();
    assert_eq!((p.albums, p.have, p.to_make.len()), (4, 1, 2));
    assert_eq!(p.no_picture, ["C"]);
    assert!(
        p.to_make
            .iter()
            .any(|(f, s)| f == "A" && s.ends_with("cover.png"))
    );
    assert!(
        p.to_make
            .iter()
            .any(|(f, s)| f == "B" && s.starts_with("the art inside"))
    );
    assert!(!sidecar(dir.path(), "A").exists());
    assert_eq!(
        fs::read_dir(dir.path().join(MEDIA_ROOT).join("A"))
            .unwrap()
            .count(),
        before
    );
    let text = plan_lines(&p).join("\n");
    assert!(
        text.contains("2 to make") && text.contains("1 album has no picture"),
        "{text}"
    );
}

#[test]
fn run_writes_valid_covers_and_leaves_everything_else() {
    let dir = card();
    let c = dir.path().join(MEDIA_ROOT);
    let music = fs::read(c.join("A/01 t.mp3")).unwrap();
    let picture = fs::read(c.join("A/cover.png")).unwrap();
    let mut seen = Vec::new();
    let d = run(dir.path(), &mut |i, n| seen.push((i, n))).unwrap();
    assert_eq!((d.written, d.failed.len(), d.library_tracks), (2, 0, None));
    assert_eq!(seen, [(1, 2), (2, 2)]);
    for a in ["A", "B"] {
        let bytes = fs::read(sidecar(dir.path(), a)).unwrap();
        assert!(image::decode_tim1(&bytes).is_ok(), "{a}");
        assert!(!sidecar(dir.path(), a).with_extension("tmp").exists());
    }
    // A's cover is what Omega's encoder makes from the folder picture.
    assert_eq!(
        fs::read(sidecar(dir.path(), "A")).unwrap(),
        image::encode_pal256_bytes(&picture, 128).unwrap()
    );
    assert_eq!(
        fs::read(sidecar(dir.path(), "D")).unwrap(),
        b"existing",
        "an existing cover is never replaced"
    );
    assert!(!sidecar(dir.path(), "C").exists());
    assert_eq!(music, fs::read(c.join("A/01 t.mp3")).unwrap());
    assert_eq!(picture, fs::read(c.join("A/cover.png")).unwrap());
    // Nothing left to do, and a second run changes nothing.
    assert!(plan(dir.path()).unwrap().to_make.is_empty());
    assert_eq!(run(dir.path(), &mut |_, _| {}).unwrap().written, 0);
}

#[test]
fn a_picture_that_will_not_decode_is_listed_and_the_others_are_still_made() {
    let dir = card();
    let c = dir.path().join(MEDIA_ROOT);
    fs::write(c.join("A/cover.png"), b"not a picture at all").unwrap();
    let d = run(dir.path(), &mut |_, _| {}).unwrap();
    assert_eq!(d.written, 1);
    assert_eq!(d.failed.len(), 1);
    assert_eq!(d.failed[0].0, "A");
    assert!(sidecar(dir.path(), "B").exists());
    assert!(!sidecar(dir.path(), "A").exists());
    assert!(!sidecar(dir.path(), "A").with_extension("tmp").exists());
}

#[test]
fn an_existing_library_is_rebuilt_after_the_covers() {
    let dir = card();
    rebuild::run(dir.path()).unwrap();
    let d = run(dir.path(), &mut |_, _| {}).unwrap();
    assert_eq!(d.library_tracks, Some(4));
    assert!(render_done(&d).contains("Library   rebuilt"));
}

#[test]
fn no_media_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(plan(dir.path()), Err(Error::NoMediaRoot));
    assert_eq!(run(dir.path(), &mut |_, _| {}), Err(Error::NoMediaRoot));
}
