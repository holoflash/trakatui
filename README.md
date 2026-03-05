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

**XM Format**

- Support for all XM effects and commands
- XM module loading and playback

**UI & Input**

- Chromatic keyboard input (Z-P)
- Musical scales for input transposition
- Keyboard shortcut reference

**TODO**

- Make the settings panel more compact
- Undo
- MOD import
- MOD export
- XM export
- Native project save/load

> _All features are subject to change without notice before version 1.0.0_

---

This project is inspired by the trackers of the late 90s and early 2000s and builds upon the .xm format as documented [here](https://github.com/milkytracker/MilkyTracker/blob/master/resources/reference/xm-form.txt).

If you're looking for a great fully featured modern alternative, check out [**Furnace**](https://github.com/tildearrow/furnace).

## Install

Download the latest archive for your platform from [**Releases**](https://github.com/holoflash/psikat/releases/latest) and extract it:

| Platform              | File                          |
| --------------------- | ----------------------------- |
| macOS (Apple Silicon) | `psikat-macos-aarch64.tar.gz` |
| macOS (Intel)         | `psikat-macos-x86_64.tar.gz`  |
| Linux                 | `psikat-linux-x86_64.tar.gz`  |
| Windows               | `psikat-windows-x86_64.zip`   |

```sh
tar xzf psikat-*.tar.gz
./psikat
```

**macOS users:** If you see "Apple could not verify", run this once before launching:

```sh
xattr -d com.apple.quarantine psikat
```

## License

MIT
