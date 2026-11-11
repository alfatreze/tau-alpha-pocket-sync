# TAU Library: a Pocket Sync / pupdate plugin for TAU

A small plugin for [Pocket Sync](https://github.com/neil-morrison44/pocket-sync) and pupdate that looks after the music side of a card used with **TAU**, the music player core for the Analogue Pocket ([Tau-Alpha](https://github.com/alfatreze/Tau-Alpha)).

> **Status: early.** Today it does one thing: **Check the card** (read only). The library rebuild and the cover and name sidecars are the next steps (see [`docs/PLAN.md`](docs/PLAN.md)). Nothing here writes to your card yet.

## What "Check the card" tells you
* whether TAU is installed (and which version) and whether `Assets/tau/common` exists;
* how many tracks, albums and playlists TAU can see (MP3 and FLAC are what TAU plays);
* whether the library index (`tau-library.tdb`) is there; without it TAU shows an empty library;
* whether the name sidecar (`tau-names`) is there;
* which albums have no cover sidecar (`tau-art/cover_128.pal256.timg`; without one TAU decodes the embedded JPEG, which takes seconds per track);
* files TAU cannot play (m4a, ogg, opus, wav, ...): listed and left alone;
* macOS stubs (`._*`, `.DS_Store`).

## Use it
Releases are not published yet. When they are, add `https://github.com/alfatreze/tau-pocket-sync` as a plugin repository in Pocket Sync (Plugins), connect your card and run **TAU Library**. The same plugin format is meant to work in pupdate (not tested yet).

## Want more? Tau Omega
This plugin stays small on purpose (a plugin cannot run external tools). **[Tau Omega](https://github.com/alfatreze/Tau-Omega)** does the rest: converts any format to MP3 or FLAC for TAU, edits tags and covers, keeps ReplayGain right, builds playlists, installs and updates TAU and your meter presets, and checks the whole card. Free, for Mac, Windows and Linux. The plugin has a menu item that opens its download page.

## Build and test
```sh
rustup target add wasm32-wasip1
cargo test -p tps-core                                   # the card logic, natively
cargo build -p tps-plugin --target wasm32-wasip1 --release
cargo test -p tps-host-test                              # runs the real plugin.wasm under Extism with a disposable card
```
The release needs two files, `plugin.wasm` (the build above) and `plugin.json` (`crates/tps-plugin/plugin.json`); a tag `vX.Y.Z` builds and attaches them (`.github/workflows/release.yml`).

## How it is organised
* `crates/tps-core`: scan, classify and report on a card directory. Plain Rust, no host or WASM dependency, unit tested.
* `crates/tps-plugin`: the WebAssembly plugin: the plugin message protocol and the menus, nothing else. The two host calls are the only `unsafe` code.
* `crates/tps-host-test`: loads the compiled plugin in Extism with the two mounts the real hosts give it (the card as `pocket/`, a private folder as `host/`) and drives it like a host does.

The plugin depends on nothing outside this repository (no code is shared by path with Tau-Alpha or Tau Omega).

## Licence
MIT, see [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md).
