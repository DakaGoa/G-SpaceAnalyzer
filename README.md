# G-SpaceAnalyzer

[![Release](https://github.com/DakaGoa/G-SpaceAnalyzer/actions/workflows/release.yml/badge.svg)](https://github.com/DakaGoa/G-SpaceAnalyzer/actions/workflows/release.yml)

A fast, modern, free and open source disk space analyzer for Windows, written in Rust.

G-SpaceAnalyzer shows where the space on a drive actually went, then lets you do something
about it: browse a space map, sort the biggest files, find duplicates, clear caches, spot
data that has not been touched in years, and compare two saved scans to see what grew.

No account, no licence key, no telemetry, no network access — everything is measured on your
own PC.

![G-SpaceAnalyzer showing a completed scan of C:\Windows](assets/screenshot.png)

## Features

- **Space map** — treemap of any folder, coloured by file type, age or folder, with zoom and a
  size-ordered list view
- **Largest files** — sortable table with instant path inspection and filtering
- **File types** — space and count per extension and category
- **Duplicates** — content-verified identical files, single or bulk removal
- **Search** — name search across the whole scan
- **Clean up** — temp folders, browser caches and Windows Update leftovers, measured then cleared
- **Old files** — files untouched for 1, 2, 3, 5 or 10 years
- **Snapshots** — save a scan and diff it against the current one; the compact option stores
  folder totals only and takes a fraction of the disk space
- **Everywhere** — right-click any file or folder to open it, open its location, copy its path
  or delete it (Recycle Bin by default, optionally permanent)
- **Also on the command line** — `G-SpaceAnalyzer --scan C:\ --export report.csv` runs the same
  scan headless

## Install

**[Download the latest release](https://github.com/DakaGoa/G-SpaceAnalyzer/releases/latest)** and
run `GSpaceAnalyzerSetup.exe` — or use the direct link:
<https://github.com/DakaGoa/G-SpaceAnalyzer/releases/latest/download/GSpaceAnalyzerSetup.exe>

It installs per user into `%LOCALAPPDATA%\Programs\G-SpaceAnalyzer` (no administrator rights, no
UAC prompt), adds a Start menu shortcut and registers with Add or Remove Programs. Upgrades install
over the top and keep your settings and snapshots, and the same setup file can be used to uninstall.

Silent install: `GSpaceAnalyzerSetup.exe /S`

## Build

Requires a recent stable Rust toolchain on Windows.

```
cargo build --release
```

The binary lands at `target\release\G-SpaceAnalyzer.exe`.

To build the installer as well:

```
powershell -ExecutionPolicy Bypass -File installer\build-installer.ps1
```

The installer is built with the C# compiler that ships with .NET Framework, so no extra tooling
is needed, and the result is a single self-contained `dist\GSpaceAnalyzerSetup.exe`.

## Tests

```
cargo test
```

## Licence

MIT — see [LICENSE](LICENSE).
