# Plan

Where the plugin is going. Today (v0.1.0-dev) only **Check the card** exists.

## How a Pocket plugin works (and what that means for us)
A plugin is a WebAssembly module (`wasm32-wasip1`, [Extism](https://extism.org)) that a host (Pocket Sync, pupdate) runs. It answers with JSON `Choice`, `Text` or `Exit` messages and can call two host functions (`print_msg`, `open_url`). The host gives it two folders: the card as `pocket/` and a private desktop folder as `host/`. There is **no folder picker and no other access to the desktop**, plugins are single-threaded, and a run cannot be cancelled cleanly, so every write must be temp file, verify, rename. The reference is [`openfpga-library/pocket-plugin`](https://github.com/openfpga-library/pocket-plugin) (MIT).

## Steps
1. **Check the card** (read only): done.
2. **Rebuild the library from the files on the card.** Scan `Assets/tau/common`, read the tags, build `tau-library.tdb` and the name sidecar (`tau-names.json` and `tau-names.bin`) the same way Tau Omega and the Tau-Alpha tools do, and replace them safely. Always show a plan first (tracks, albums, playlists, what is skipped, name problems) and ask before writing. The index builder is Tau Omega's `tau-core` (MIT); Omega made it build for WASI (tag `tau-core-wasm-2`, which also scans without threads on WASI). **Built (2026-11-11): the plan, the confirmed write, the safe swap and the name sidecar; the output is checked against the reference builder `tau_library.py` (same content) and the WebAssembly build is byte-identical to the native one.**
3. **Cover sidecars.** For each album folder without `tau-art/cover_128.pal256.timg`, make it from the folder image or the first track's embedded cover (palette-256, 128 px on the long side), so TAU stops decoding a JPEG for seconds on every track change.
4. **Small extras** (each read-only or asks first): library check (missing files, files not in the index, stale sidecar), card-safe names only where the Pocket cannot open a name, a junk sweep for `._*` and `.DS_Store`.
5. **Optional: add music from the plugin's folder** (`host/`): copy additively, skip what is already there, then rebuild. Because of the sandbox it means copying twice; copying with the file manager and then rebuilding is usually faster.

## Not planned
Converting or re-encoding audio (needs external tools that cannot run in a plugin): files TAU cannot play are listed and left alone, and the plugin points to Tau Omega, which converts them.

## Unmeasured
Time and memory on large libraries (tens of thousands of tracks) through WASI; how each host treats a long call; behaviour in pupdate and in the current Pocket Sync (the plugin has only been run under Extism with the same two mounts).
