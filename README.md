# PSIKAT

A lightweight cross-platform music tracker built with Rust.

![screenshot](screenshot.png)

## 🚧 UNDER DEVELOPMENT

### Features

**Playback & Audio**

- Real-time playback with play-from-cursor support
- Stereo WAV export (44.1kHz, 16-bit)
- Master volume control with peak meter
- Per-channel muting/soloing

**Pattern Editor**

- Note, instrument, volume, and effect columns
- Multi-pattern support with order table
- Selection, copy/paste, fill, and transpose
- Configurable edit step and subdivision

**Instruments & Synthesis**

- 5 built-in waveforms (sine, triangle, square, saw, noise)
- Sample import with drag-and-drop
- Sample loops and volume envelopes
- Auto-vibrato
- Envelope editor

**UI & Input**

- Chromatic keyboard input (Z-P)
- Musical scales for input transposition
- Keyboard shortcut reference

**.psikat format**

- Save/load native .psikat project files
- Undo/redo (100 steps)

> _All features are subject to change without notice before version 1.0.0_

---

Psikat is a fresh take on the music tracker.

While working with the .xm and .mod formats initially was a great starting point, there are already excellent trackers like Furnace, Renoise, OpenMPT, and MilkyTracker that handle legacy formats well and are made by people who have deep knowledge of the tracker format and scene. My background being purely in music production in traditional DAWs, I feel like my efforts are better spent on building something new instead, that aligns more with my own personal preferences. I just really love the tracker workflow, but some habits from traditional DAWs are too ingrained in me to let go of and I'd like to see if I can bridge that gap.

## Install

> _If **Releases** is empty, I'm doing some big changes and wiped out the old releases because they no longer represent what psikat is about. New release coming soon! In the meanwhile you can build psikat from source following the instructions below._

Download the latest release for your platform from [**Releases**](https://github.com/holoflash/psikat/releases/latest):

| Platform              | File                         |
| --------------------- | ---------------------------- |
| macOS (Apple Silicon) | `psikat-macos-aarch64.dmg`   |
| macOS (Intel)         | `psikat-macos-x86_64.dmg`    |
| Linux                 | `psikat-linux-x86_64.tar.gz` |
| Windows               | `psikat-windows-x86_64.zip`  |

**macOS:** Open the `.dmg` and drag Psikat to Applications. If macOS says the app "is damaged and can't be opened", run this once in Terminal:

```sh
xattr -cr /Applications/Psikat.app
```

## Build from Source

Requires [Rust](https://rustup.rs/).

**macOS** — creates a `Psikat.app` bundle (requires Python + Pillow for icon generation):

```sh
./scripts/bundle_macos.sh
open target/Psikat.app
```

**Linux** — requires ALSA and X11/Wayland dev libraries:

```sh
sudo apt install libasound2-dev libgl1-mesa-dev libxkbcommon-dev libwayland-dev
./scripts/build_linux.sh
```

**Windows:**

```bat
scripts\build_windows.bat
```

## License

MIT
