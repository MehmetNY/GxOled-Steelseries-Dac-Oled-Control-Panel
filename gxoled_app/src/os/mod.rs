pub const IDLE_TIMEOUT_MS: u32 = 60_000;

const MAX_MEDIA_TITLE_CHARS: usize = 160;
const MAX_MEDIA_ARTIST_CHARS: usize = 96;

#[derive(PartialEq)]
pub struct Media {
    pub title: String,
    pub artist: String,
}

impl Media {
    /// Limit and sanitize metadata from external media sessions before sending it to the
    /// font renderer. Ordinary metadata is unchanged; control characters become spaces so
    /// a title cannot inject additional OLED text lines or produce an oversized bitmap.
    pub fn sanitize(self) -> Self {
        Self {
            title: sanitize_media_field(&self.title, MAX_MEDIA_TITLE_CHARS),
            artist: sanitize_media_field(&self.artist, MAX_MEDIA_ARTIST_CHARS),
        }
    }
}

fn sanitize_media_field(value: &str, max_chars: usize) -> String {
    let mut result = String::with_capacity(value.len().min(max_chars.saturating_mul(4)) + 1);
    let mut chars = value.chars();
    for ch in chars.by_ref().take(max_chars) {
        result.push(if ch.is_control() { ' ' } else { ch });
    }
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{MAX_MEDIA_ARTIST_CHARS, MAX_MEDIA_TITLE_CHARS, Media};

    #[test]
    fn media_metadata_is_bounded_and_control_characters_are_sanitized() {
        let media = Media {
            title: format!("{}\nInjected line", "T".repeat(MAX_MEDIA_TITLE_CHARS + 4)),
            artist: "Artist\r\nName".to_owned(),
        }
        .sanitize();

        assert_eq!(media.title.chars().count(), MAX_MEDIA_TITLE_CHARS + 1);
        assert!(media.title.ends_with('…'));
        assert_eq!(media.artist, "Artist  Name");
        assert!(media.artist.chars().count() <= MAX_MEDIA_ARTIST_CHARS);
    }
}

pub trait OSFeatures {
    fn new() -> Self
    where
        Self: Sized;

    fn supports_media(&self) -> bool;
    fn get_media(&mut self) -> Option<Media>;

    fn supports_idle(&self) -> bool;
    fn is_idle(&mut self) -> bool;
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::OSImpl;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::OSImpl;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::OSImpl;
