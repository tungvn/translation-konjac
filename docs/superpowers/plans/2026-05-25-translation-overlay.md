# Konjac Translation Overlay — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a semi-transparent, always-on-top Tauri 2.x desktop overlay that captures the screen beneath it, detects changes via pixel diffing with a configurable δ threshold, and translates content via a vision LLM through Cloudflare AI Gateway.

**Architecture:** Rust backend handles screen capture (platform-specific window exclusion via CGWindowListCreateImage on macOS and SetWindowDisplayAffinity on Windows), image diffing (mean pixel delta score), and HTTP calls to Cloudflare AI Gateway. React frontend renders translated text and controls. State flows Rust → React via Tauri events; React → Rust via Tauri commands.

**Tech Stack:** Tauri 2.x · Rust · React 18 + TypeScript · Vite · tokio · reqwest · image crate · base64 · tokio-util · wiremock (tests) · Vitest + @testing-library/react (frontend tests)

---

## File Map

```
translation-konjac/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # entry point, Tauri setup, command registration
│   │   ├── lib.rs               # re-exports for Tauri
│   │   ├── config.rs            # AppConfig: load/save TOML
│   │   ├── diff.rs              # compute_diff_score, is_changed
│   │   ├── translate.rs         # TranslateEngine, TranslateError
│   │   └── capture/
│   │       ├── mod.rs           # CaptureRect, run_capture_loop, platform dispatch
│   │       ├── macos.rs         # CGWindowListCreateImage (cfg macos)
│   │       └── windows.rs       # SetWindowDisplayAffinity (cfg windows)
│   ├── build.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── App.css
│   └── components/
│       ├── TranslationDisplay.tsx
│       ├── TranslationDisplay.test.tsx
│       ├── LanguagePicker.tsx
│       ├── LanguagePicker.test.tsx
│       ├── ToolbarControls.tsx
│       └── ToolbarControls.test.tsx
├── index.html
├── vite.config.ts
├── package.json
└── tsconfig.json
```

---

### Task 1: Scaffold Tauri 2.x project with React + TypeScript

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `index.html`
- Create: `tsconfig.json`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/App.css`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Initialise the project**

```bash
cd /Users/tungvn/Works/Pets/translation-konjac
pnpm create tauri-app@latest . -- --template react-ts --manager pnpm --identifier com.konjac.app --name konjac
```

If prompted interactively, choose: **React**, **TypeScript**, **pnpm**.

- [ ] **Step 2: Verify the scaffold built**

```bash
pnpm install
pnpm tauri dev
```

Expected: a blank Tauri window appears. Close it before continuing.

- [ ] **Step 3: Replace `src-tauri/tauri.conf.json` with the correct window config**

```json
{
  "productName": "Konjac",
  "version": "0.1.0",
  "identifier": "com.konjac.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Konjac",
        "width": 400,
        "height": 300,
        "minWidth": 300,
        "minHeight": 150,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "resizable": true,
        "shadow": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": []
  }
}
```

- [ ] **Step 4: Add Rust dependencies to `src-tauri/Cargo.toml`**

Replace the `[dependencies]` section (keep `[package]` and `[build-dependencies]` as generated):

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
reqwest = { version = "0.12", features = ["json"] }
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
toml = "0.8"
dirs = "5"
thiserror = "2"

[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging"] }

[dev-dependencies]
wiremock = "0.6"
```

- [ ] **Step 5: Verify Rust compiles**

```bash
cd src-tauri && cargo check && cd ..
```

Expected: no errors (warnings about unused items are fine at this stage).

- [ ] **Step 6: Set up git remote and commit**

```bash
git init
git remote add origin git@github.com:tungvn/translation-konjac.git
git add .
git commit -m "feat: scaffold Tauri 2.x project with React+TS and window config"
git push -u origin main
```

---

### Task 2: Config module

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of a new file `src-tauri/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_or_default_returns_defaults_when_no_file() {
        let dir = PathBuf::from("/tmp/konjac-test-nonexistent-12345");
        let config = AppConfig::load_or_default(dir);
        assert_eq!(config.target_language, "English");
        assert!((config.delta_threshold - 0.05).abs() < f32::EPSILON);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-5.4-nano");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test config -- --nocapture 2>&1 | head -20
```

Expected: FAIL — `AppConfig` not found.

- [ ] **Step 3: Implement `AppConfig`**

Write `src-tauri/src/config.rs` in full:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gateway_url: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub target_language: String,
    pub delta_threshold: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_url: String::new(),
            provider: "openai".to_string(),
            model: "gpt-5.4-nano".to_string(),
            api_key: String::new(),
            target_language: "English".to_string(),
            delta_threshold: 0.05,
        }
    }
}

impl AppConfig {
    pub fn load_or_default(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("config.toml");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&app_data_dir)?;
        let path = app_data_dir.join("config.toml");
        std::fs::write(path, toml::to_string(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_or_default_returns_defaults_when_no_file() {
        let dir = PathBuf::from("/tmp/konjac-test-nonexistent-12345");
        let config = AppConfig::load_or_default(dir);
        assert_eq!(config.target_language, "English");
        assert!((config.delta_threshold - 0.05).abs() < f32::EPSILON);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-5.4-nano");
    }

    #[test]
    fn save_and_reload_round_trips() {
        let dir = PathBuf::from("/tmp/konjac-test-config-roundtrip");
        let mut cfg = AppConfig::default();
        cfg.target_language = "Vietnamese".to_string();
        cfg.delta_threshold = 0.1;
        cfg.save(dir.clone()).unwrap();

        let loaded = AppConfig::load_or_default(dir.clone());
        assert_eq!(loaded.target_language, "Vietnamese");
        assert!((loaded.delta_threshold - 0.1).abs() < f32::EPSILON);

        std::fs::remove_dir_all(dir).ok();
    }
}
```

- [ ] **Step 4: Add `mod config;` to `src-tauri/src/lib.rs`**

```rust
pub mod config;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd src-tauri && cargo test config -- --nocapture
```

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat: add AppConfig with TOML load/save and defaults"
```

---

### Task 3: Diff engine

**Files:**
- Create: `src-tauri/src/diff.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/diff.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage, Rgb};

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |_, _| Rgb(rgb)))
    }

    #[test]
    fn identical_images_score_near_zero() {
        let img = solid(100, 100, [128, 128, 128]);
        let score = compute_diff_score(&img, &img);
        assert!(score < 0.001, "expected ~0, got {score}");
    }

    #[test]
    fn black_vs_white_score_near_one() {
        let black = solid(100, 100, [0, 0, 0]);
        let white = solid(100, 100, [255, 255, 255]);
        let score = compute_diff_score(&black, &white);
        assert!(score > 0.99, "expected ~1, got {score}");
    }

    #[test]
    fn is_changed_respects_threshold() {
        assert!(is_changed(0.10, 0.05));
        assert!(!is_changed(0.03, 0.05));
        assert!(!is_changed(0.05, 0.05)); // equal is NOT changed
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test diff -- --nocapture 2>&1 | head -20
```

Expected: FAIL — `compute_diff_score` not found.

- [ ] **Step 3: Implement the diff engine**

Replace `src-tauri/src/diff.rs` with the full implementation:

```rust
use image::DynamicImage;

/// Returns a normalised difference score in [0.0, 1.0].
/// Both images are downsampled to 64×64 before comparison for speed.
pub fn compute_diff_score(prev: &DynamicImage, current: &DynamicImage) -> f32 {
    let prev_thumb = prev.thumbnail(64, 64).to_rgb8();
    let curr_thumb = current.thumbnail(64, 64).to_rgb8();

    let total: u64 = prev_thumb
        .pixels()
        .zip(curr_thumb.pixels())
        .map(|(p, c)| {
            (p[0] as i64 - c[0] as i64).unsigned_abs()
                + (p[1] as i64 - c[1] as i64).unsigned_abs()
                + (p[2] as i64 - c[2] as i64).unsigned_abs()
        })
        .sum();

    let w = prev_thumb.width() as u64;
    let h = prev_thumb.height() as u64;
    let max_diff = w * h * 3 * 255;
    total as f32 / max_diff as f32
}

/// Returns true only when score strictly exceeds the threshold.
pub fn is_changed(score: f32, threshold: f32) -> bool {
    score > threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgb, RgbImage};

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |_, _| Rgb(rgb)))
    }

    #[test]
    fn identical_images_score_near_zero() {
        let img = solid(100, 100, [128, 128, 128]);
        let score = compute_diff_score(&img, &img);
        assert!(score < 0.001, "expected ~0, got {score}");
    }

    #[test]
    fn black_vs_white_score_near_one() {
        let black = solid(100, 100, [0, 0, 0]);
        let white = solid(100, 100, [255, 255, 255]);
        let score = compute_diff_score(&black, &white);
        assert!(score > 0.99, "expected ~1, got {score}");
    }

    #[test]
    fn is_changed_respects_threshold() {
        assert!(is_changed(0.10, 0.05));
        assert!(!is_changed(0.03, 0.05));
        assert!(!is_changed(0.05, 0.05));
    }
}
```

- [ ] **Step 4: Add `mod diff;` to `src-tauri/src/lib.rs`**

```rust
pub mod config;
pub mod diff;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd src-tauri && cargo test diff -- --nocapture
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/diff.rs src-tauri/src/lib.rs
git commit -m "feat: add diff engine with configurable delta threshold"
```

---

### Task 4: Translate engine

**Files:**
- Create: `src-tauri/src/translate.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/translate.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn translate_returns_text_from_api() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "content": "Hello world" } }]
            })))
            .mount(&server)
            .await;

        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let result = engine.translate(&img, "English", CancellationToken::new()).await;
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn translate_returns_cancelled_when_token_is_pre_cancelled() {
        let server = MockServer::start().await;
        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let token = CancellationToken::new();
        token.cancel();
        let result = engine.translate(&img, "English", token).await;
        assert!(matches!(result, Err(TranslateError::Cancelled)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test translate -- --nocapture 2>&1 | head -20
```

Expected: FAIL — `TranslateEngine` not found.

- [ ] **Step 3: Implement the translate engine**

Replace `src-tauri/src/translate.rs` with the full implementation:

```rust
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::DynamicImage;
use reqwest::Client;
use std::io::Cursor;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("request cancelled")]
    Cancelled,
    #[error("image encoding failed: {0}")]
    ImageEncode(#[from] image::ImageError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected api response")]
    BadResponse,
}

pub struct TranslateEngine {
    client: Client,
    gateway_url: String,
    model: String,
    api_key: String,
}

impl TranslateEngine {
    pub fn new(gateway_url: String, model: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            gateway_url,
            model,
            api_key,
        }
    }

    pub async fn translate(
        &self,
        image: &DynamicImage,
        target_language: &str,
        cancel: CancellationToken,
    ) -> Result<String, TranslateError> {
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let mut buf = Cursor::new(Vec::new());
        image.write_to(&mut buf, image::ImageFormat::Jpeg)?;
        let b64 = BASE64.encode(buf.get_ref());

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": format!("data:image/jpeg;base64,{}", b64) }
                    },
                    {
                        "type": "text",
                        "text": format!(
                            "You are a translation assistant. Extract all text visible in this image and translate it to {}. Return ONLY the translated text, preserving paragraph breaks. If no text is visible, return empty string.",
                            target_language
                        )
                    }
                ]
            }],
            "max_tokens": 1024
        });

        let request = self
            .client
            .post(format!("{}/chat/completions", self.gateway_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send();

        let response = tokio::select! {
            res = request => res.map_err(TranslateError::Http)?,
            _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
        };

        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(TranslateError::BadResponse)?
            .to_string();

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn translate_returns_text_from_api() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "content": "Hello world" } }]
            })))
            .mount(&server)
            .await;

        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let result = engine.translate(&img, "English", CancellationToken::new()).await;
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn translate_returns_cancelled_when_token_is_pre_cancelled() {
        let server = MockServer::start().await;
        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let token = CancellationToken::new();
        token.cancel();
        let result = engine.translate(&img, "English", token).await;
        assert!(matches!(result, Err(TranslateError::Cancelled)));
    }
}
```

- [ ] **Step 4: Add `mod translate;` to `src-tauri/src/lib.rs`**

```rust
pub mod config;
pub mod diff;
pub mod translate;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd src-tauri && cargo test translate -- --nocapture
```

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/translate.rs src-tauri/src/lib.rs
git commit -m "feat: add translate engine with Cloudflare AI Gateway and cancellation support"
```

---

### Task 5: Capture module — shared types + macOS implementation

**Files:**
- Create: `src-tauri/src/capture/mod.rs`
- Create: `src-tauri/src/capture/macos.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `src-tauri/src/capture/mod.rs`**

```rust
use image::DynamicImage;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct CaptureRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture returned null image")]
    NullImage,
    #[error("image conversion failed")]
    ConversionError,
    #[error("platform error: {0}")]
    Platform(String),
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn capture_below_window(rect: CaptureRect, window_id: u32) -> Result<DynamicImage, CaptureError> {
    #[cfg(target_os = "macos")]
    return macos::capture_below_window(rect, window_id);

    #[cfg(target_os = "windows")]
    return windows::capture_below_window(rect, window_id);

    #[allow(unreachable_code)]
    Err(CaptureError::Platform("unsupported platform".to_string()))
}
```

- [ ] **Step 2: Create `src-tauri/src/capture/macos.rs`**

```rust
use super::{CaptureError, CaptureRect};
use image::{DynamicImage, RgbaImage};
use std::ffi::c_void;

// CoreGraphics types
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint { x: f64, y: f64 }

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize { width: f64, height: f64 }

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect { origin: CGPoint, size: CGSize }

// 1 << 4: capture windows below the given window ID
const CG_WINDOW_LIST_OPTION_ON_SCREEN_BELOW_WINDOW: u32 = 1 << 4;
const CG_WINDOW_IMAGE_DEFAULT: u32 = 0;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCreateImage(
        screen_bounds: CGRect,
        list_option: u32,
        window_id: u32,
        image_option: u32,
    ) -> *mut c_void;

    fn CGImageGetWidth(image: *const c_void) -> usize;
    fn CGImageGetHeight(image: *const c_void) -> usize;
    fn CGImageGetBytesPerRow(image: *const c_void) -> usize;
    fn CGImageGetDataProvider(image: *const c_void) -> *mut c_void;
    fn CGDataProviderCopyData(provider: *const c_void) -> *const c_void;
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFDataGetLength(data: *const c_void) -> isize;
    fn CFRelease(cf: *const c_void);
    fn CGImageRelease(image: *mut c_void);
}

pub fn capture_below_window(rect: CaptureRect, window_id: u32) -> Result<DynamicImage, CaptureError> {
    unsafe {
        let cg_rect = CGRect {
            origin: CGPoint { x: rect.x as f64, y: rect.y as f64 },
            size: CGSize { width: rect.width as f64, height: rect.height as f64 },
        };

        let image_ref = CGWindowListCreateImage(
            cg_rect,
            CG_WINDOW_LIST_OPTION_ON_SCREEN_BELOW_WINDOW,
            window_id,
            CG_WINDOW_IMAGE_DEFAULT,
        );

        if image_ref.is_null() {
            return Err(CaptureError::NullImage);
        }

        let width = CGImageGetWidth(image_ref) as u32;
        let height = CGImageGetHeight(image_ref) as u32;
        let provider = CGImageGetDataProvider(image_ref);
        let data_ref = CGDataProviderCopyData(provider);

        if data_ref.is_null() {
            CGImageRelease(image_ref);
            return Err(CaptureError::ConversionError);
        }

        let len = CFDataGetLength(data_ref) as usize;
        let ptr = CFDataGetBytePtr(data_ref);
        let bytes = std::slice::from_raw_parts(ptr, len).to_vec();

        CFRelease(data_ref);
        CGImageRelease(image_ref);

        // CoreGraphics returns BGRA; convert to RGBA
        let rgba_bytes: Vec<u8> = bytes
            .chunks(4)
            .flat_map(|px| [px[2], px[1], px[0], px[3]])
            .collect();

        RgbaImage::from_raw(width, height, rgba_bytes)
            .map(DynamicImage::ImageRgba8)
            .ok_or(CaptureError::ConversionError)
    }
}
```

- [ ] **Step 3: Add `pub mod capture;` to `src-tauri/src/lib.rs`**

```rust
pub mod capture;
pub mod config;
pub mod diff;
pub mod translate;
```

- [ ] **Step 4: Verify it compiles on macOS**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/capture/
git add src-tauri/src/lib.rs
git commit -m "feat: add capture module with macOS CGWindowListCreateImage exclusion"
```

---

### Task 6: Windows capture

**Files:**
- Create: `src-tauri/src/capture/windows.rs`

- [ ] **Step 1: Create `src-tauri/src/capture/windows.rs`**

```rust
use super::{CaptureError, CaptureRect};
use image::DynamicImage;

// Called once at startup via init_window_exclusion; capture uses screenshots crate normally.
pub fn capture_below_window(rect: CaptureRect, _window_id: u32) -> Result<DynamicImage, CaptureError> {
    use screenshots::Screen;

    let screens = Screen::all().map_err(|e| CaptureError::Platform(e.to_string()))?;
    let screen = screens.first().ok_or(CaptureError::Platform("no screens".to_string()))?;

    let capture = screen
        .capture_area(rect.x, rect.y, rect.width, rect.height)
        .map_err(|e| CaptureError::Platform(e.to_string()))?;

    let rgba = image::RgbaImage::from_raw(
        capture.width(),
        capture.height(),
        capture.rgba().to_vec(),
    )
    .ok_or(CaptureError::ConversionError)?;

    Ok(DynamicImage::ImageRgba8(rgba))
}
```

- [ ] **Step 2: Add `screenshots` to Windows-only deps in `src-tauri/Cargo.toml`**

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging"] }
screenshots = "0.8"
```

- [ ] **Step 3: Add `SetWindowDisplayAffinity` helper to `src-tauri/src/capture/windows.rs`**

Append to the file:

```rust
/// Call once at Tauri startup to exclude our window from all screen captures.
#[cfg(target_os = "windows")]
pub fn init_window_exclusion(hwnd: isize) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
    use windows::Win32::Foundation::HWND;
    unsafe {
        let _ = SetWindowDisplayAffinity(HWND(hwnd as *mut _), WDA_EXCLUDEFROMCAPTURE);
    }
}
```

- [ ] **Step 4: Verify Windows target compiles (cross-check)**

```bash
cd src-tauri && cargo check --target x86_64-pc-windows-msvc 2>&1 | grep -E "^error" | head -10
```

Expected: no `error:` lines (install the target first if needed: `rustup target add x86_64-pc-windows-msvc`).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/capture/windows.rs src-tauri/Cargo.toml
git commit -m "feat: add Windows capture with SetWindowDisplayAffinity exclusion"
```

---

### Task 7: Capture loop + Tauri commands + app wiring

**Files:**
- Modify: `src-tauri/src/capture/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add `run_capture_loop` to `src-tauri/src/capture/mod.rs`**

Append to the existing `src-tauri/src/capture/mod.rs`:

```rust
use crate::{
    config::AppConfig,
    diff,
    translate::{TranslateEngine, TranslateError},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub config: AppConfig,
    pub is_capturing: bool,
}

pub async fn run_capture_loop(app: tauri::AppHandle, state: Arc<Mutex<AppState>>) {
    let mut prev_image: Option<DynamicImage> = None;
    let mut inflight_cancel: Option<CancellationToken> = None;

    loop {
        sleep(Duration::from_millis(500)).await; // 2 fps

        let (is_capturing, config) = {
            let s = state.lock().await;
            (s.is_capturing, s.config.clone())
        };

        if !is_capturing {
            continue;
        }

        let window = match app.get_webview_window("main") {
            Some(w) => w,
            None => continue,
        };

        let pos = match window.outer_position() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let size = match window.outer_size() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let rect = CaptureRect {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        };

        #[cfg(target_os = "macos")]
        let window_id = {
            use objc::{msg_send, runtime::Object};
            let ns_win = match window.ns_window() {
                Ok(ptr) => ptr as *const Object,
                Err(_) => continue,
            };
            let n: i32 = unsafe { msg_send![ns_win, windowNumber] };
            n as u32
        };

        #[cfg(not(target_os = "macos"))]
        let window_id: u32 = 0;

        let current = match capture_below_window(rect, window_id) {
            Ok(img) => img,
            Err(_) => continue,
        };

        if let Some(ref prev) = prev_image {
            let score = diff::compute_diff_score(prev, &current);
            if !diff::is_changed(score, config.delta_threshold) {
                continue;
            }
        }

        prev_image = Some(current.clone());

        // Cancel any in-flight API call
        if let Some(token) = inflight_cancel.take() {
            token.cancel();
        }

        let token = CancellationToken::new();
        inflight_cancel = Some(token.clone());

        let app_clone = app.clone();
        let lang = config.target_language.clone();
        let engine = TranslateEngine::new(
            config.gateway_url.clone(),
            config.model.clone(),
            config.api_key.clone(),
        );

        tauri::async_runtime::spawn(async move {
            let _ = app_clone.emit("translation-loading", ());
            match engine.translate(&current, &lang, token).await {
                Ok(text) => {
                    let _ = app_clone.emit("translation-updated", text);
                }
                Err(TranslateError::Cancelled) => {}
                Err(e) => {
                    let _ = app_clone.emit("translation-error", e.to_string());
                }
            }
        });
    }
}
```

- [ ] **Step 2: Replace `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

use translation_konjac_lib::{
    capture::{run_capture_loop, AppState},
    config::AppConfig,
};

#[tauri::command]
async fn set_target_language(
    language: String,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    state.lock().await.config.target_language = language;
    Ok(())
}

#[tauri::command]
async fn set_delta_threshold(
    threshold: f32,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    state.lock().await.config.delta_threshold = threshold;
    Ok(())
}

#[tauri::command]
async fn pause_capture(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    state.lock().await.is_capturing = false;
    Ok(())
}

#[tauri::command]
async fn resume_capture(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    state.lock().await.is_capturing = true;
    Ok(())
}

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<translation_konjac_lib::config::AppConfig, String> {
    Ok(state.lock().await.config.clone())
}

#[tauri::command]
async fn save_config(
    config: AppConfig,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    config.save(dir).map_err(|e| e.to_string())?;
    state.lock().await.config = config;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let config = AppConfig::load_or_default(dir);
            let state = Arc::new(Mutex::new(AppState {
                config,
                is_capturing: true,
            }));
            app.manage(state.clone());

            // Windows: exclude overlay from screen capture at startup
            #[cfg(target_os = "windows")]
            {
                use translation_konjac_lib::capture::windows::init_window_exclusion;
                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(hwnd) = win.hwnd() {
                        init_window_exclusion(hwnd.0 as isize);
                    }
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                run_capture_loop(app_handle, state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_target_language,
            set_delta_threshold,
            pause_capture,
            resume_capture,
            get_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/capture/mod.rs src-tauri/src/main.rs
git commit -m "feat: add capture loop, app state, and Tauri commands"
```

---

### Task 8: Frontend — TranslationDisplay component

**Files:**
- Create: `src/components/TranslationDisplay.tsx`
- Create: `src/components/TranslationDisplay.test.tsx`

- [ ] **Step 1: Install Vitest + React Testing Library**

```bash
pnpm add -D vitest @vitejs/plugin-react jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event
```

- [ ] **Step 2: Add Vitest config to `vite.config.ts`**

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
  },
});
```

- [ ] **Step 3: Create `src/test-setup.ts`**

```typescript
import "@testing-library/jest-dom";

// Mock Tauri event API
(globalThis as any).__TAURI_INTERNALS__ = {};
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));
```

- [ ] **Step 4: Write the failing test**

Create `src/components/TranslationDisplay.test.tsx`:

```typescript
import { render, screen, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import TranslationDisplay from "./TranslationDisplay";

describe("TranslationDisplay", () => {
  it("shows spinner when loading", () => {
    render(<TranslationDisplay loading={true} text="" error={null} />);
    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("renders translated text", () => {
    render(<TranslationDisplay loading={false} text="Bonjour" error={null} />);
    expect(screen.getByText("Bonjour")).toBeInTheDocument();
  });

  it("renders error message", () => {
    render(<TranslationDisplay loading={false} text="" error="API error" />);
    expect(screen.getByText(/API error/i)).toBeInTheDocument();
  });

  it("shows placeholder when idle with no text", () => {
    render(<TranslationDisplay loading={false} text="" error={null} />);
    expect(screen.getByText(/move the window/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 5: Run test to verify it fails**

```bash
pnpm vitest run src/components/TranslationDisplay.test.tsx
```

Expected: FAIL — `TranslationDisplay` not found.

- [ ] **Step 6: Implement `TranslationDisplay`**

Create `src/components/TranslationDisplay.tsx`:

```typescript
interface Props {
  loading: boolean;
  text: string;
  error: string | null;
}

export default function TranslationDisplay({ loading, text, error }: Props) {
  if (loading) {
    return (
      <div className="translation-body center">
        <div className="spinner" role="status" aria-label="Translating" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="translation-body center error">
        <span>{error}</span>
      </div>
    );
  }

  if (!text) {
    return (
      <div className="translation-body center placeholder">
        <span>Move the window over text to translate</span>
      </div>
    );
  }

  return (
    <div className="translation-body">
      <p className="translation-text">{text}</p>
    </div>
  );
}
```

- [ ] **Step 7: Run tests to verify they pass**

```bash
pnpm vitest run src/components/TranslationDisplay.test.tsx
```

Expected: 4 tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/components/TranslationDisplay.tsx src/components/TranslationDisplay.test.tsx src/test-setup.ts vite.config.ts
git commit -m "feat: add TranslationDisplay component with loading, error, and idle states"
```

---

### Task 9: Frontend — LanguagePicker component

**Files:**
- Create: `src/components/LanguagePicker.tsx`
- Create: `src/components/LanguagePicker.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `src/components/LanguagePicker.test.tsx`:

```typescript
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import LanguagePicker from "./LanguagePicker";

describe("LanguagePicker", () => {
  it("renders the current language", () => {
    render(<LanguagePicker value="English" onChange={vi.fn()} />);
    expect(screen.getByDisplayValue("English")).toBeInTheDocument();
  });

  it("calls onChange when selection changes", () => {
    const onChange = vi.fn();
    render(<LanguagePicker value="English" onChange={onChange} />);
    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: "Vietnamese" },
    });
    expect(onChange).toHaveBeenCalledWith("Vietnamese");
  });

  it("includes common languages in the list", () => {
    render(<LanguagePicker value="English" onChange={vi.fn()} />);
    const options = screen.getAllByRole("option").map((o) => o.textContent);
    expect(options).toContain("Japanese");
    expect(options).toContain("Chinese (Simplified)");
    expect(options).toContain("Vietnamese");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm vitest run src/components/LanguagePicker.test.tsx
```

Expected: FAIL — `LanguagePicker` not found.

- [ ] **Step 3: Implement `LanguagePicker`**

Create `src/components/LanguagePicker.tsx`:

```typescript
const LANGUAGES = [
  "English", "Vietnamese", "Japanese", "Chinese (Simplified)",
  "Chinese (Traditional)", "Korean", "French", "German", "Spanish",
  "Portuguese", "Italian", "Russian", "Arabic", "Hindi", "Thai",
  "Indonesian", "Dutch", "Polish", "Turkish", "Swedish",
];

interface Props {
  value: string;
  onChange: (lang: string) => void;
}

export default function LanguagePicker({ value, onChange }: Props) {
  return (
    <select
      className="language-picker"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {LANGUAGES.map((lang) => (
        <option key={lang} value={lang}>
          {lang}
        </option>
      ))}
    </select>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
pnpm vitest run src/components/LanguagePicker.test.tsx
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/LanguagePicker.tsx src/components/LanguagePicker.test.tsx
git commit -m "feat: add LanguagePicker with 20 common languages"
```

---

### Task 10: Frontend — ToolbarControls component

**Files:**
- Create: `src/components/ToolbarControls.tsx`
- Create: `src/components/ToolbarControls.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `src/components/ToolbarControls.test.tsx`:

```typescript
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import ToolbarControls from "./ToolbarControls";

describe("ToolbarControls", () => {
  it("shows pause button when capturing", () => {
    render(
      <ToolbarControls capturing={true} onPause={vi.fn()} onResume={vi.fn()} onOpenSettings={vi.fn()} />
    );
    expect(screen.getByLabelText("Pause")).toBeInTheDocument();
  });

  it("shows resume button when paused", () => {
    render(
      <ToolbarControls capturing={false} onPause={vi.fn()} onResume={vi.fn()} onOpenSettings={vi.fn()} />
    );
    expect(screen.getByLabelText("Resume")).toBeInTheDocument();
  });

  it("calls onPause when pause clicked", () => {
    const onPause = vi.fn();
    render(
      <ToolbarControls capturing={true} onPause={onPause} onResume={vi.fn()} onOpenSettings={vi.fn()} />
    );
    fireEvent.click(screen.getByLabelText("Pause"));
    expect(onPause).toHaveBeenCalled();
  });

  it("shows settings button", () => {
    render(
      <ToolbarControls capturing={true} onPause={vi.fn()} onResume={vi.fn()} onOpenSettings={vi.fn()} />
    );
    expect(screen.getByLabelText("Settings")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm vitest run src/components/ToolbarControls.test.tsx
```

Expected: FAIL — `ToolbarControls` not found.

- [ ] **Step 3: Implement `ToolbarControls`**

Create `src/components/ToolbarControls.tsx`:

```typescript
interface Props {
  capturing: boolean;
  onPause: () => void;
  onResume: () => void;
  onOpenSettings: () => void;
}

export default function ToolbarControls({ capturing, onPause, onResume, onOpenSettings }: Props) {
  return (
    <div className="toolbar-controls">
      <button
        className="icon-btn"
        aria-label={capturing ? "Pause" : "Resume"}
        onClick={capturing ? onPause : onResume}
      >
        {capturing ? "⏸" : "▶"}
      </button>
      <button
        className="icon-btn"
        aria-label="Settings"
        onClick={onOpenSettings}
      >
        ⚙
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
pnpm vitest run src/components/ToolbarControls.test.tsx
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/ToolbarControls.tsx src/components/ToolbarControls.test.tsx
git commit -m "feat: add ToolbarControls with pause/resume and settings button"
```

---

### Task 11: Frontend — App.tsx, styles, and settings popover

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: Replace `src/App.tsx`**

```typescript
import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import TranslationDisplay from "./components/TranslationDisplay";
import LanguagePicker from "./components/LanguagePicker";
import ToolbarControls from "./components/ToolbarControls";

interface AppConfig {
  gateway_url: string;
  provider: string;
  model: string;
  api_key: string;
  target_language: string;
  delta_threshold: number;
}

export default function App() {
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [capturing, setCapturing] = useState(true);
  const [language, setLanguage] = useState("English");
  const [showSettings, setShowSettings] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    invoke<AppConfig>("get_config").then((c) => {
      setConfig(c);
      setLanguage(c.target_language);
    });

    const unlisteners = [
      listen("translation-loading", () => {
        setLoading(true);
        setError(null);
      }),
      listen<string>("translation-updated", (e) => {
        setLoading(false);
        setText(e.payload);
      }),
      listen<string>("translation-error", (e) => {
        setLoading(false);
        setError(e.payload);
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  const handleLanguageChange = useCallback((lang: string) => {
    setLanguage(lang);
    invoke("set_target_language", { language: lang });
  }, []);

  const handlePause = useCallback(() => {
    setCapturing(false);
    invoke("pause_capture");
  }, []);

  const handleResume = useCallback(() => {
    setCapturing(true);
    invoke("resume_capture");
  }, []);

  const handleSaveSettings = useCallback((updated: AppConfig) => {
    invoke("save_config", { config: updated }).then(() => {
      setConfig(updated);
      setLanguage(updated.target_language);
      setShowSettings(false);
    });
  }, []);

  return (
    <div className="app">
      <div className="toolbar" data-tauri-drag-region>
        <LanguagePicker value={language} onChange={handleLanguageChange} />
        <ToolbarControls
          capturing={capturing}
          onPause={handlePause}
          onResume={handleResume}
          onOpenSettings={() => setShowSettings((v) => !v)}
        />
      </div>

      {showSettings && config ? (
        <SettingsPopover config={config} onSave={handleSaveSettings} onClose={() => setShowSettings(false)} />
      ) : (
        <TranslationDisplay loading={loading} text={text} error={error} />
      )}
    </div>
  );
}

function SettingsPopover({
  config,
  onSave,
  onClose,
}: {
  config: AppConfig;
  onSave: (c: AppConfig) => void;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(config);

  return (
    <div className="settings">
      <label>
        Gateway URL
        <input value={draft.gateway_url} onChange={(e) => setDraft({ ...draft, gateway_url: e.target.value })} />
      </label>
      <label>
        API Key
        <input type="password" value={draft.api_key} onChange={(e) => setDraft({ ...draft, api_key: e.target.value })} />
      </label>
      <label>
        Model
        <input value={draft.model} onChange={(e) => setDraft({ ...draft, model: e.target.value })} />
      </label>
      <label>
        Delta threshold ({draft.delta_threshold.toFixed(2)})
        <input
          type="range" min="0.01" max="0.5" step="0.01"
          value={draft.delta_threshold}
          onChange={(e) => setDraft({ ...draft, delta_threshold: parseFloat(e.target.value) })}
        />
      </label>
      <div className="settings-actions">
        <button onClick={() => onSave(draft)}>Save</button>
        <button onClick={onClose}>Cancel</button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Replace `src/App.css`**

```css
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  background: transparent;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #fff;
  overflow: hidden;
  user-select: none;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: rgba(10, 10, 10, 0.82);
  border-radius: 8px;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  height: 32px;
  background: rgba(255, 255, 255, 0.06);
  cursor: grab;
  flex-shrink: 0;
}

.toolbar:active { cursor: grabbing; }

.language-picker {
  background: transparent;
  color: #fff;
  border: none;
  font-size: 12px;
  cursor: pointer;
  outline: none;
}

.language-picker option { background: #1a1a1a; }

.toolbar-controls { display: flex; gap: 4px; }

.icon-btn {
  background: transparent;
  border: none;
  color: rgba(255,255,255,0.7);
  font-size: 14px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
}

.icon-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }

.translation-body {
  flex: 1;
  padding: 10px 12px;
  overflow-y: auto;
}

.translation-body.center {
  display: flex;
  align-items: center;
  justify-content: center;
}

.translation-text { font-size: 14px; line-height: 1.6; }

.placeholder { color: rgba(255,255,255,0.35); font-size: 12px; }

.error { color: #ff6b6b; font-size: 12px; }

@keyframes spin { to { transform: rotate(360deg); } }

.spinner {
  width: 20px; height: 20px;
  border: 2px solid rgba(255,255,255,0.2);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

.settings {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  overflow-y: auto;
}

.settings label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  color: rgba(255,255,255,0.6);
}

.settings input[type="text"],
.settings input[type="password"] {
  background: rgba(255,255,255,0.08);
  border: 1px solid rgba(255,255,255,0.15);
  color: #fff;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  outline: none;
}

.settings input[type="range"] { accent-color: #fff; }

.settings-actions {
  display: flex;
  gap: 8px;
  margin-top: auto;
}

.settings-actions button {
  flex: 1;
  padding: 6px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.2);
  background: rgba(255,255,255,0.08);
  color: #fff;
  cursor: pointer;
  font-size: 12px;
}

.settings-actions button:hover { background: rgba(255,255,255,0.15); }
```

- [ ] **Step 3: Run all frontend tests**

```bash
pnpm vitest run
```

Expected: all tests pass.


- [ ] **Step 4: Commit**

```bash
git add src/App.tsx src/App.css
git commit -m "feat: wire up App with all components, settings popover, and Tauri events"
```

---

### Task 12: Full smoke test + macOS screen recording permission

**Files:** No new files — verification only.

- [ ] **Step 1: Run all Rust tests**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass (config ×2, diff ×3, translate ×2 = 7 total).

- [ ] **Step 2: Run all frontend tests**

```bash
pnpm vitest run
```

Expected: all tests pass (TranslationDisplay ×4, LanguagePicker ×3, ToolbarControls ×4 = 11 total).

- [ ] **Step 3: Launch in dev mode**

```bash
pnpm tauri dev
```

Expected: a dark semi-transparent borderless window appears, always on top.

- [ ] **Step 4: Grant screen recording permission on macOS**

When the app first tries to capture, macOS will prompt for screen recording permission. If the prompt doesn't appear automatically:

```
System Settings → Privacy & Security → Screen Recording → enable Konjac
```

After granting, restart the dev server.

- [ ] **Step 5: Manual smoke test**

1. Open a page with visible text (e.g., a Terminal window with `man ls`)
2. Move the Konjac overlay on top of the text
3. Confirm after ~1 second a translated result appears
4. Change target language from the picker — confirm new translation fires
5. Click ⏸ — confirm translations stop
6. Click ▶ — confirm translations resume
7. Open ⚙ → adjust delta slider → save → verify the threshold change persists (check `~/Library/Application Support/com.konjac.app/config.toml`)

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "chore: verified full smoke test on macOS — capture, diff, translate, settings all working"
```
