//! Clipboard item model: text and image clippings with bounded previews, dedup hashing, and search.

use std::io::Cursor;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use uuid::Uuid;

/// Kind of content stored in a [`Clipping`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClippingKind {
    /// UTF-8 plain text (optionally with rich HTML).
    Text,
    /// Raster PNG image with bounded preview thumbnail.
    Image,
}

/// A single captured clipboard entry stored locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Clipping {
    /// Unique stable identifier for this clipping.
    pub id: Uuid,
    /// Whether this clipping is text or an image.
    pub kind: ClippingKind,
    /// Plain text content (or descriptive label for images, e.g. `"Image 1440×900"`).
    pub text: String,
    /// Optional rich HTML representation for text clippings.
    pub html: Option<String>,
    /// Optional base64-encoded PNG bytes for image clippings.
    pub image_png_base64: Option<String>,
    /// Image width in pixels when `kind == ClippingKind::Image`.
    pub image_width: Option<u32>,
    /// Image height in pixels when `kind == ClippingKind::Image`.
    pub image_height: Option<u32>,
    /// Approximate payload size in bytes.
    pub byte_size: usize,
    /// SHA-256 content digest used for deduplication.
    pub content_hash: String,
    /// Name of the source application that owned the copy, when known.
    pub source_app: Option<String>,
    /// Creation timestamp in Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Whether the user pinned this clipping so it is retained across pruning.
    pub pinned: bool,
}

impl Clipping {
    /// Constructs a new text clipping.
    #[must_use]
    pub fn from_text(text: String, html: Option<String>, source_app: Option<String>) -> Self {
        let byte_size = text.len() + html.as_ref().map_or(0, String::len);
        let content_hash = hash_bytes(b"text:", text.as_bytes());
        Self {
            id: Uuid::new_v4(),
            kind: ClippingKind::Text,
            text,
            html,
            image_png_base64: None,
            image_width: None,
            image_height: None,
            byte_size,
            content_hash,
            source_app,
            created_at_epoch_ms: now_epoch_ms(),
            pinned: false,
        }
    }

    /// Constructs a new image clipping from encoded PNG bytes.
    #[must_use]
    pub fn from_image(
        png_bytes: &[u8],
        width: u32,
        height: u32,
        source_app: Option<String>,
    ) -> Self {
        let content_hash = hash_bytes(b"img:", png_bytes);
        let encoded = BASE64.encode(png_bytes);
        Self {
            id: Uuid::new_v4(),
            kind: ClippingKind::Image,
            text: format!("Image {width}×{height}"),
            html: None,
            image_png_base64: Some(encoded),
            image_width: Some(width),
            image_height: Some(height),
            byte_size: png_bytes.len(),
            content_hash,
            source_app,
            created_at_epoch_ms: now_epoch_ms(),
            pinned: false,
        }
    }

    /// Decodes the stored PNG image bytes when `kind == ClippingKind::Image`.
    #[must_use]
    pub fn image_png_bytes(&self) -> Option<Vec<u8>> {
        self.image_png_base64
            .as_deref()
            .and_then(|b64| BASE64.decode(b64).ok())
    }

    /// Decodes a compact RGBA thumbnail (`width`, `height`, `rgba`) bounded to `max_w × max_h`
    /// so image rows stay strictly bounded in height and render quickly.
    #[must_use]
    pub fn decode_bounded_rgba(&self, max_w: u32, max_h: u32) -> Option<(u32, u32, Vec<u8>)> {
        let png_bytes = self.image_png_bytes()?;
        let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0_u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        let src_w = info.width;
        let src_h = info.height;
        if src_w == 0 || src_h == 0 {
            return None;
        }
        let rgba = to_rgba(&buf[..info.buffer_size()], info.color_type)?;

        if src_w <= max_w && src_h <= max_h {
            return Some((src_w, src_h, rgba));
        }

        let scale_x = f64::from(max_w) / f64::from(src_w);
        let scale_y = f64::from(max_h) / f64::from(src_h);
        let scale = scale_x.min(scale_y);

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "scaled dimensions are positive and clamped to max_w/max_h"
        )]
        let dst_w = ((f64::from(src_w) * scale).round() as u32).clamp(1, max_w);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "scaled dimensions are positive and clamped to max_w/max_h"
        )]
        let dst_h = ((f64::from(src_h) * scale).round() as u32).clamp(1, max_h);

        let mut out = vec![0_u8; (dst_w as usize) * (dst_h as usize) * 4];
        for dy in 0..dst_h {
            let sy =
                (u64::from(dy) * u64::from(src_h) / u64::from(dst_h)).min(u64::from(src_h - 1));
            for dx in 0..dst_w {
                let sx =
                    (u64::from(dx) * u64::from(src_w) / u64::from(dst_w)).min(u64::from(src_w - 1));
                let src_idx = (usize::try_from(sy).unwrap_or(0) * (src_w as usize)
                    + usize::try_from(sx).unwrap_or(0))
                    * 4;
                let dst_idx = ((dy as usize) * (dst_w as usize) + (dx as usize)) * 4;
                if src_idx + 4 <= rgba.len() && dst_idx + 4 <= out.len() {
                    out[dst_idx..dst_idx + 4].copy_from_slice(&rgba[src_idx..src_idx + 4]);
                }
            }
        }
        Some((dst_w, dst_h, out))
    }

    /// Returns a compact preview string normalized for a 1–2 line overlay row.
    #[must_use]
    pub fn preview_text(&self, max_chars: usize) -> String {
        let collapsed: String = self.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut chars = collapsed.chars();
        let taken: String = chars.by_ref().take(max_chars).collect();
        if chars.next().is_some() {
            format!("{taken}…")
        } else {
            taken
        }
    }

    /// Returns a human-readable size label (for example `"42 B"`, `"1.4 KB"`, `"2.1 MB"`).
    #[must_use]
    pub fn formatted_size(&self) -> String {
        const KB: usize = 1024;
        const MB: usize = 1024 * 1024;
        if self.byte_size >= MB {
            let tenths = (self.byte_size.saturating_mul(10)) / MB;
            format!("{}.{} MB", tenths / 10, tenths % 10)
        } else if self.byte_size >= KB {
            let tenths = (self.byte_size.saturating_mul(10)) / KB;
            format!("{}.{} KB", tenths / 10, tenths % 10)
        } else {
            format!("{} B", self.byte_size)
        }
    }

    /// Returns `true` when all whitespace-separated query tokens match this clipping's text or source app.
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return true;
        }
        let haystack = self.text.to_lowercase();
        let app_haystack = self
            .source_app
            .as_deref()
            .map(str::to_lowercase)
            .unwrap_or_default();
        trimmed.split_whitespace().all(|token| {
            let needle = token.to_lowercase();
            haystack.contains(&needle) || app_haystack.contains(&needle)
        })
    }
}

fn to_rgba(raw: &[u8], color_type: png::ColorType) -> Option<Vec<u8>> {
    match color_type {
        png::ColorType::Rgba => Some(raw.to_vec()),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((raw.len() / 3) * 4);
            for &[r, g, b] in raw.as_chunks::<3>().0 {
                out.extend_from_slice(&[r, g, b, 255]);
            }
            Some(out)
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((raw.len() / 2) * 4);
            for &[g, a] in raw.as_chunks::<2>().0 {
                out.extend_from_slice(&[g, g, g, a]);
            }
            Some(out)
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(raw.len() * 4);
            for &g in raw {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            Some(out)
        }
        png::ColorType::Indexed => None,
    }
}

fn hash_bytes(prefix: &[u8], payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix);
    hasher.update(payload);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
