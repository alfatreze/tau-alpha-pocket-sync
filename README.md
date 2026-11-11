# TAU Library: a Pocket Sync / pupdate plugin for TAU

<img src="assets/plugin-icon.png" alt="TAU Library plugin icon" width="120" align="right">

A small plugin for [Pocket Sync](https://github.com/neil-morrison44/pocket-sync) and pupdate that looks after the music side of a card used with **TAU**, the music player core for the Analogue Pocket ([Tau-Alpha](https://github.com/alfatreze/Tau-Alpha)).

> **Status: first release, early.** It has three functions: **Check the card** (read only), **Rebuild the library** (the library and the album covers, in one step) and **Make album covers only** (both show a plan first and write only after you say yes). See [`docs/PLAN.md`](docs/PLAN.md) for what comes next. Tested on disposable cards under the plugin host; not tried yet in the real Pocket Sync or pupdate.

## What "Check the card" tells you
* whether TAU is installed (and which version) and whether `Assets/tau/common` exists;
* how many tracks, albums and playlists TAU can see (MP3 and FLAC are what TAU plays);
* whether the library index (`tau-library.tdb`) is there; without it TAU shows an empty library;
* whether the name sidecar (`tau-names`) is there;
* which albums have no cover sidecar (`tau-art/cover_128.pal256.timg`; without one TAU decodes the embedded JPEG, which takes seconds per track);
* files TAU cannot play (m4a, ogg, opus, wav, ...): listed and left alone;
* macOS stubs (`._*`, `.DS_Store`).

## What "Rebuild the library" does
It makes the missing album covers (see below) and then writes the library, in one step with one plan.

It reads the tags of the MP3 and FLAC files under `Assets/tau/common` and writes `tau-library.tdb` (the index TAU lists) and, when names need it, the name sidecar (`tau-names.json` and `tau-names.bin`, the original text behind the Pocket-safe names). It shows the plan first (tracks, albums, playlists, files it must leave out) and asks. The old library stays until the new one has been written, read back and checked. **Your music files are never renamed, moved or deleted**; a file the Pocket cannot open (a character outside code page 437, or a path over 200 bytes) is left out and listed. The index and the sidecar are made by [Tau Omega](https://github.com/alfatreze/Tau-Omega)'s `tau-core`, so the plugin and Tau Omega write the same format.

## What "Make album covers" does
TAU shows an album's cover from a small file, `<album>/tau-art/cover_128.pal256.timg`. Without it the player decodes the album's JPEG on every track change, which takes seconds. The plugin finds the albums that have no such file, shows which picture each would be made from (the folder's `cover.jpg` / `folder.png` / similar, or the art inside the first track that has some), and after you confirm writes the files: a temp file, read back and checked, then renamed. It never replaces an existing cover file and never changes your pictures or your music. Albums with no picture at all are listed and left alone. Afterwards it rebuilds the library (if there is one), because the index tells the player which albums have a cover file.

## Use it
In Pocket Sync open **Plugins**, add the repository `alfatreze/tau-alpha-pocket-sync`, connect your card and run **TAU Library**. The first release is 0.1.0 (see [`CHANGELOG.md`](CHANGELOG.md)); it has not been tried in every version of Pocket Sync, and the same plugin format is meant to work in pupdate (not tested yet). Eject the card from your computer before you take it out of the reader.

## Want more? Tau Omega
This plugin stays small on purpose (a plugin cannot run external tools). **[Tau Omega](https://github.com/alfatreze/Tau-Omega)** does the rest: converts any format to MP3 or FLAC for TAU, edits tags and covers, keeps ReplayGain right, builds playlists, installs and updates TAU and your meter presets, and checks the whole card. Free, for Mac, Windows and Linux. The plugin has a menu item that opens its download page.

## Build and test
```sh
rustup target add wasm32-wasip1
cargo test -p tps-core                                   # the card logic and the golden comparison with tau_library.py, natively
cargo build -p tps-plugin --target wasm32-wasip1 --release
cargo test -p tps-host-test                              # runs the real plugin.wasm under Extism with a disposable card
```
The release needs two files, `plugin.wasm` (the build above) and `plugin.json` (`crates/tps-plugin/plugin.json`); a tag `vX.Y.Z` builds and attaches them (`.github/workflows/release.yml`).

## How it is organised
* `crates/tps-core`: scan, classify and report on a card directory. Plain Rust, no host or WASM dependency, unit tested.
* `crates/tps-plugin`: the WebAssembly plugin: the plugin message protocol and the menus, nothing else. The two host calls are the only `unsafe` code.
* `crates/tps-host-test`: loads the compiled plugin in Extism with the two mounts the real hosts give it (the card as `pocket/`, a private folder as `host/`) and drives it like a host does.

The plugin shares no code with Tau-Alpha by path. It depends on Tau Omega's `tau-core` as an ordinary library pinned to a tag (see `Cargo.toml`).

## Licence
MIT, see [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md).
