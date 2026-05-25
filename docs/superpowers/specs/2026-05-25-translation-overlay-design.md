# Translation Overlay — Design Spec
**Date:** 2026-05-25  
**Project:** translation-konjac  
**Status:** Approved

---

## Overview

A cross-platform desktop app (macOS first, Windows next) that floats a semi-transparent always-on-top window over the screen. It continuously captures whatever is *beneath* the window, detects meaningful content changes, and uses a vision LLM to OCR + translate in a single API call — displaying the result directly inside the overlay.

---

## Architecture

Four focused units:

```
┌─────────────────────────────────────────────┐
│              Tauri Floating Window           │
│  ┌──────────────┐   ┌─────────────────────┐ │
│  │ React UI     │   │   Rust Backend       │ │
│  │              │   │                      │ │
│  │ Translation  │◄──│  1. Capture Engine   │ │
│  │ Display      │   │  2. Diff Engine      │ │
│  │              │   │  3. Translate Engine │ │
│  │ Language     │──►│                      │ │
│  │ Picker       │   │                      │ │
│  └──────────────┘   └──────────┬───────────┘ │
└─────────────────────────────────┼─────────────┘
                                  │ HTTPS
                     ┌────────────▼───────────┐
                     │  Cloudflare AI Gateway  │
                     │  → OpenAI nano / DeepSeek│
                     └────────────────────────┘
```

**Data flow:**
1. Capture Engine grabs the screen region *below* the window at ~2 fps
2. Diff Engine computes distance score vs previous frame — skips if `score < δ`
3. On change: JPEG-encode image → POST to Cloudflare AI Gateway → vision LLM OCRs + translates
4. Translated text fires Tauri event `translation-updated` → React renders it in the overlay

---

## Capture Engine

Runs a background Rust thread at ~2 fps.

On each tick:
1. Gets current window position/size from Tauri
2. Calls platform-specific capture:
   - **macOS:** `CGWindowListCreateImage(rect, onScreenBelowWindow, myWindowID)` — native window exclusion
   - **Windows:** `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` called once at startup; thereafter `screenshots` crate captures normally
3. Passes raw image to Diff Engine
4. If Diff Engine returns "go" → encode as JPEG (85% quality) → hand off to Translate Engine

The capture rect always mirrors current window bounds — dragging or resizing automatically shifts the capture zone.

**Platform abstraction:**
```rust
// capture/mod.rs
pub fn capture_below_window(rect: Rect, window_handle: WindowHandle) -> Result<Image>;

// capture/macos.rs  — #[cfg(target_os = "macos")]
// capture/windows.rs — #[cfg(target_os = "windows")]
```

---

## Diff Engine

Computes a normalized distance score (0.0–1.0) between the previous and current frame.

```
score = frame_diff(prev, current)

if score < δ  → skip (unchanged)
if score ≥ δ  → translate
```

- **Algorithm:** mean absolute pixel difference across downsampled frames (fast, good enough)
- **δ (delta threshold):** configurable, default `0.05` (5% pixel difference)
- Catches cursor blinks, minor animations, subtle redraws without triggering API calls
- δ is exposed in the settings popover so the user can tune sensitivity

---

## Translate Engine

Single HTTP POST to Cloudflare AI Gateway per changed frame.

**Endpoint:**
```
https://<cf-gateway-url>/openai/v1/chat/completions
# or /deepseek/... — switchable via config
```

**Prompt:**
```
You are a translation assistant. Extract all text visible in this image 
and translate it to {target_language}. Return ONLY the translated text, 
preserving paragraph breaks. If no text is visible, return empty string.
```

**Key behaviours:**
- Image encoded as **JPEG at 85% quality** before base64 (~60% smaller than PNG)
- Cloudflare AI Gateway caching means identical frames cost $0 after first hit
- Response fires Tauri event `translation-updated` on arrival
- If an API call is in-flight when a new change is detected, the in-flight request is **cancelled** (tokio abort handle) and a fresh one starts — prevents stale translation queue

**Config file** (stored in Tauri app data dir):
```toml
[translate]
gateway_url    = "https://..."
provider       = "openai"        # or "deepseek"
model          = "gpt-5.4-nano"
target_language = "English"
delta_threshold = 0.05
```

---

## Frontend (React)

Minimal UI — translation text dominates, controls stay out of the way.

**Layout:**
```
┌─────────────────────────────────┐
│ [EN ▾]              [⏸] [⚙]    │  ← thin toolbar (24px), drag handle
├─────────────────────────────────┤
│                                 │
│  Translated text renders here,  │
│  white on dark semi-transparent │
│  background. Scrollable if long │
│                                 │
└─────────────────────────────────┘
```

**Components:**
- `LanguagePicker` — dropdown of ~20 common languages, fires `set_target_language` Tauri command
- `TranslationDisplay` — listens on `translation-updated` event, renders text with subtle fade-in, shows spinner while in-flight
- `ToolbarControls` — pause/resume toggle (`⏸/▶`), settings gear opens popover for δ threshold + API config

**Window styling:**
- Background: `rgba(10, 10, 10, 0.82)`
- Text: white, `font-size: 14px`, relaxed line-height
- Toolbar: `data-tauri-drag-region`
- Resize: native Tauri edge handles

**State:** single `useState` + Tauri event listeners — no router, no state library.

---

## File Structure

```
translation-konjac/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app setup, window config
│   │   ├── capture/
│   │   │   ├── mod.rs           # shared interface: capture_below_window()
│   │   │   ├── macos.rs         # CGWindowListCreateImage impl
│   │   │   └── windows.rs       # SetWindowDisplayAffinity + screenshots crate
│   │   ├── diff.rs              # frame distance scoring + δ threshold
│   │   └── translate.rs         # Cloudflare AI Gateway HTTP client
│   ├── Cargo.toml
│   └── tauri.conf.json          # transparent, decorations:false, always_on_top
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── TranslationDisplay.tsx
│   │   └── LanguagePicker.tsx
│   └── main.tsx
├── package.json
└── docs/
    └── superpowers/
        └── specs/
            └── 2026-05-25-translation-overlay-design.md
```

---

## Decisions Summary

| Concern | Decision |
|---|---|
| Framework | Tauri 2.x |
| Capture | Rust, platform-specific exclusion (macOS/Windows) |
| Diff | Pixel distance score + configurable δ threshold |
| OCR + Translation | Single vision LLM call via Cloudflare AI Gateway |
| Provider | OpenAI gpt-5.4-nano or DeepSeek (switchable in config) |
| Languages | Any → user-selectable target |
| Trigger | Hybrid: continuous capture, API only on Δ ≥ δ |
| In-flight requests | Cancelled on new change (tokio abort) |
| Frontend | React, no router/state library |
| Window | Borderless, transparent, draggable toolbar, resizable |
| Cross-platform path | macOS first, Windows via cfg blocks in capture module |
