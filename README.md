# TRAKATUI

A music tracker for the terminal, built with Rust.

![screenshot](screenshot.png)

[Audio exported from the pattern seen in the screenshot](demo.wav)

## 🚧 WORK IN PROGRESS

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
- [ ] More sensible default settings
- [ ] Stable audio playback
- [ ] Instrument/Synth editing
- [ ] Effects channels
- [ ] Patterns
- [ ] Keybinding settings
- [ ] Save/load from file
- [ ] Sampler channel

## Install

Download the latest archive for your platform from [**Releases**](https://github.com/holoflash/trakatui/releases/latest) and extract it:

| Platform              | File                            |
| --------------------- | ------------------------------- |
| macOS (Apple Silicon) | `trakatui-macos-aarch64.tar.gz` |
| macOS (Intel)         | `trakatui-macos-x86_64.tar.gz`  |
| Linux                 | `trakatui-linux-x86_64.tar.gz`  |
| Windows               | `trakatui-windows-x86_64.zip`   |

```sh
tar xzf trakatui-*.tar.gz
./trakatui
```

**macOS users:** If you see "Apple could not verify", run this once before launching:

```sh
xattr -d com.apple.quarantine trakatui
```

## License

MIT
