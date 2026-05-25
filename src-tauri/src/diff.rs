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
