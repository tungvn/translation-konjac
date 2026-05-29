# Font Size Control — Design Spec

**Date:** 2026-05-29  
**Status:** Approved

## Overview

Add +/− toolbar buttons that let the user increase or decrease the translation text font size (range 10–20px, step 1px, default 13px). The preference persists across app relaunches via the existing `config.toml` mechanism.

## Architecture

The feature extends the existing Rust `AppConfig` + Tauri command pipeline. No new persistence mechanism is introduced.

### Rust (`src-tauri/src/`)

**`config.rs`**
- Add `font_size: u8` field to `AppConfig` and `StoredConfig`.
- Add `default_font_size() -> u8` returning `13`.
- `StoredConfig` uses `#[serde(default = "default_font_size")]` so existing config files without the field load cleanly.

**`lib.rs`**
- Add Tauri command `set_font_size(size: u8, state, app_handle)` that:
  1. Clamps `size` to `10..=20`.
  2. Updates the in-memory `AppConfig` in the Tauri managed state.
  3. Calls `config.save(app_data_dir)` to persist.
- Register the command in `tauri::Builder`.

### Frontend (`src/`)

**`App.tsx`**
- Add `fontSize` state (default `13`), initialized from `config.font_size` when `get_config` resolves.
- Add `handleFontSizeChange(delta: -1 | 1)` callback that clamps within 10–20, updates local state, and calls `invoke("set_font_size", { size })`.
- Pass `fontSize` to `<TranslationDisplay>` and `handleFontSizeChange` to `<ToolbarControls>`.

**`ToolbarControls.tsx`**
- Accept `onFontSizeChange: (delta: -1 | 1) => void` prop.
- Render two `icon-btn` buttons: `A−` and `A+` that call `onFontSizeChange(-1)` / `onFontSizeChange(1)`.

**`TranslationDisplay.tsx`**
- Accept `fontSize: number` prop.
- Apply `style={{ fontSize }}` inline on the `<pre className="translation-text">` element.
- The inline style overrides the CSS rule's `font-size: 13px`.

### CSS

No changes needed. The inline style applied to `.translation-text` takes precedence over the class rule.

## Data Flow

```
User clicks A+ / A−
  → handleFontSizeChange(±1) [App.tsx]
    → clamps to [10, 20]
    → setFontSize(newSize)           ← re-renders TranslationDisplay
    → invoke("set_font_size", size)  ← Rust: clamp + update state + save config.toml
```

On relaunch:
```
get_config → AppConfig { font_size: N, ... }
  → setFontSize(N)  → TranslationDisplay renders at N px
```

## Constraints

- Range: 10–20px inclusive, step 1px.
- Default: 13px (matches current hardcoded value).
- Clamping happens on both frontend (UX) and backend (safety).
- `api_key` continues to be stored in keyring, not config.toml (no change).

## Out of Scope

- Font family selection.
- Line-height control.
- Per-panel font sizes (history, settings).
