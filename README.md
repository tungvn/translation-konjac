# Konjac

A macOS screen translation overlay. Capture any region of your screen and get instant translations via an AI gateway API — always on top, transparent, draggable.

## Features

- Transparent, always-on-top overlay window
- Screen region capture with automatic change detection
- AI-powered translation via configurable gateway URL (OpenAI-compatible)
- Translation history with copy support
- Hides to menu bar tray when minimized
- Dark/light mode support

## Requirements

- macOS (primary target)
- Rust + Cargo
- Node.js + pnpm
- Screen Recording permission (prompted on first launch)

## Development

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

The `.app` bundle is output to `src-tauri/target/release/bundle/macos/`.

## Configuration

Settings are stored at:

```
~/Library/Application Support/com.ordinarist.konjac-translation/config.toml
```

Configure the API gateway URL, API key, target language, and change-detection sensitivity from the in-app settings panel (⚙).

## Stack

- [Tauri 2](https://tauri.app) — Rust backend, native shell
- React + TypeScript — frontend UI
- Vite — frontend bundler
