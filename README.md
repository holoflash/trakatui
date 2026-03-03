# PSIKAT

A lightweight cross-platform music tracker built with Rust.

![screenshot](screenshot.png)

## 🚧 UNDER DEVELOPMENT

All features are subject to change without notice before version 1.0.0

This project is inspired by, but doesn't strive to replace - the trackers of the late 90s and early 2000s.

If you're looking for a great modern alternative, check out [**Furnace**](https://github.com/tildearrow/furnace).

### TODO

- [ ] All XM effects
- [ ] MOD import
- [ ] XM export
- [x] XM import
- [x] Sample loops (forward, ping-pong)
- [x] Multi-pattern support with order table
- [x] Sampler channel
- [x] Pitch bend effect
- [x] Effects column
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
