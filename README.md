# PSIKAT

A lightweight cross-platform music tracker built with Rust.

![screenshot](screenshot.png)

## 🚧 UNDER DEVELOPMENT

All features are subject to change without notice before version 1.0.0

### TODO

- [x] Pattern editor
- [x] Built-in synthesizer
- [x] Real-time playback
- [x] WAV export (44.1kHz, 16-bit)
- [x] Settings panel
- [x] Chromatic keyboard note input
- [x] Scale selection
- [x] Octave control
- [x] BPM control
- [x] Pattern length control
- [x] Transpose control
- [x] Instrument/Synth editing
- [ ] Multiple color themes
- [ ] More sensible default settings
- [ ] Stable audio playback
- [ ] Effects channels
- [ ] Patterns
- [ ] Keybinding settings
- [ ] Save/load from file
- [ ] Sampler channel

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
