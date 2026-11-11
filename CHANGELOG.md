# Changelog

## 0.2.0
- **Easier to read:** the Check text, the plans and the results are now laid out in sections (CARD, NEEDS DOING, OPTIONAL; LIBRARY, ALBUM COVERS; DONE) with short lines, `!` for a problem, and the menu item that fixes each problem named under it.
- The Tau Omega item is now **↗ Get Tau Omega**.
- **Rebuild the library now also makes the album covers**: one plan, one confirmation; the covers are made first, then the library is written once. **Make album covers only** stays for when only the covers are missing.
- The **Exit** item is gone (the host's window stays open until you close it).

## 0.1.0 (first release)
- A plugin icon (`assets/plugin-icon.png`, a square crop of the TAU platform artwork), shown by the hosts from `logo_url`.
- **Check the card** (read only): is TAU installed, how many tracks, albums and playlists it can see, whether the library index, the name sidecar and the album cover files are there, which files TAU cannot play, macOS stubs.
- **Rebuild the library**: reads the tags of the MP3 and FLAC files under `Assets/tau/common` and writes `tau-library.tdb` and, when names need it, the name sidecar. Shows a plan first and asks. The old library stays until the new one is checked. Files the Pocket cannot open are left out and listed. The index is made by Tau Omega's `tau-core`.
- **Make album covers**: writes `<album>/tau-art/cover_128.pal256.timg` for albums that have none, from the folder picture or the art inside a track. Shows a plan first and asks. Never replaces an existing cover file; rebuilds the library afterwards.
- A one-line pointer to **Tau Omega** (converting, tag editing, meter presets, installs).
- Your music files are never renamed, moved or deleted.

Not tried yet in the real Pocket Sync or pupdate; not measured on very large libraries (the work runs one file at a time inside WebAssembly).
