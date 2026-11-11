//! The plugin's rebuild against the reference builder (`tau_library.py`): the same music tree must give an index that
//! says the same thing. See `golden/README.md` for why it compares content and not raw bytes.

use std::fs;
use std::path::Path;
use tau_core::{Index, parse, track_path};

fn ss(n: usize) -> [u8; 4] {
    [
        (n >> 21) as u8 & 127,
        (n >> 14) as u8 & 127,
        (n >> 7) as u8 & 127,
        n as u8 & 127,
    ]
}

fn frame(id: &str, text: &str) -> Vec<u8> {
    let mut body = vec![3u8];
    body.extend_from_slice(text.as_bytes());
    let mut f = id.as_bytes().to_vec();
    f.extend_from_slice(&ss(body.len()));
    f.extend_from_slice(&[0, 0]);
    f.extend(body);
    f
}

fn mp3(t: [&str; 6]) -> Vec<u8> {
    let ids = ["TIT2", "TPE1", "TALB", "TRCK", "TPOS", "TDRC"];
    let f: Vec<u8> = ids
        .iter()
        .zip(t)
        .filter(|(_, v)| !v.is_empty())
        .flat_map(|(i, v)| frame(i, v))
        .collect();
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend_from_slice(&ss(f.len()));
    out.extend(f);
    for _ in 0..60 {
        out.extend_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
        out.extend(std::iter::repeat_n(0u8, 413));
    }
    out
}

fn tree(root: &Path) {
    let albums: [(&str, Vec<[&str; 6]>); 4] = [
        (
            "The Band/First Light",
            vec![
                ["Opening", "The Band", "First Light", "1", "1/1", "1999"],
                ["Closing", "The Band", "First Light", "2", "1/1", "1999"],
            ],
        ),
        (
            "Cafe Del Mar",
            vec![
                ["Beyonce Song", "Beyoncé", "Café del Mar", "1", "", "2004"],
                ["Nino", "Niño Müller", "Café del Mar", "2", "", "2004"],
                ["Zebra", "Åsa Öberg", "Café del Mar", "3", "", ""],
            ],
        ),
        (
            "Two Discs",
            vec![
                ["D1 T1", "Various", "Two Discs", "1", "1/2", "2010"],
                ["D2 T1", "Various", "Two Discs", "1", "2/2", "2010"],
                ["D1 T2", "Various", "Two Discs", "2", "1/2", "2010"],
            ],
        ),
        ("Untagged", vec![["", "", "", "", "", ""]]),
    ];
    for (dir, tracks) in albums {
        fs::create_dir_all(root.join(dir)).unwrap();
        for (i, t) in tracks.iter().enumerate() {
            fs::write(
                root.join(dir)
                    .join(format!("{:02} file {}.mp3", i + 1, i + 1)),
                mp3(*t),
            )
            .unwrap();
        }
    }
    fs::write(
        root.join("mix.m3u"),
        "Cafe Del Mar/01 file 1.mp3\nThe Band/First Light/02 file 2.mp3\n",
    )
    .unwrap();
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Everything the index says, with strings resolved: artists, albums, tracks, playlists.
fn content(ix: &Index) -> Vec<String> {
    let (p0, plen) = ix.sections["strings"];
    let pool = &ix.data[p0..p0 + plen];
    let s = |off: u32| -> String {
        let rest = &pool[off as usize..];
        rest[..rest.iter().position(|&b| b == 0).unwrap()]
            .iter()
            .map(|&b| b as char)
            .collect()
    };
    let mut out = vec![format!("counts {:?} root {}", ix.counts, s(ix.root))];
    let (a0, _) = ix.sections["artists"];
    for i in 0..usize::from(ix.counts.artists) {
        let o = a0 + i * 8;
        out.push(format!(
            "artist {} first {} n {}",
            s(u32_at(&ix.data, o)),
            u16_at(&ix.data, o + 4),
            u16_at(&ix.data, o + 6)
        ));
    }
    let (b0, _) = ix.sections["albums"];
    for i in 0..usize::from(ix.counts.albums) {
        let o = b0 + i * 20;
        let d = &ix.data;
        out.push(format!(
            "album {} dir {} artist {} year {} first {} n {}",
            s(u32_at(d, o)),
            s(u32_at(d, o + 4)),
            u16_at(d, o + 8),
            u16_at(d, o + 10),
            u16_at(d, o + 12),
            u16_at(d, o + 14)
        ));
    }
    let (t0, _) = ix.sections["tracks"];
    for i in 0..ix.counts.tracks {
        let o = t0 + usize::from(i) * 16;
        let d = &ix.data;
        out.push(format!(
            "track {} path {} secs {} tno {} album {} fmt {}",
            s(u32_at(d, o)),
            track_path(ix, i).unwrap(),
            u16_at(d, o + 8),
            u16_at(d, o + 10),
            u16_at(d, o + 12),
            d[o + 14]
        ));
    }
    out
}

#[test]
fn rebuild_says_the_same_as_the_reference_builder() {
    let dir = tempfile::tempdir().unwrap();
    tree(&dir.path().join("Assets/tau/common"));
    tps_core::rebuild::run(dir.path()).unwrap();
    let ours = fs::read(dir.path().join("Assets/tau/common/tau-library.tdb")).unwrap();
    let golden =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/library.tdb")).unwrap();
    let (a, b) = (
        content(&parse(&ours).unwrap()),
        content(&parse(&golden).unwrap()),
    );
    assert!(a.len() > 15, "the comparison read too little: {a:?}");
    assert_eq!(a, b);
}
