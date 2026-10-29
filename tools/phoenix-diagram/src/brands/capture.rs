use super::{network, BrandError};
use scraper::{Html, Selector};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug)]
pub struct Captured {
    pub source_url: String,
    pub resolved_url: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

pub fn fetch(value: &str) -> Result<Captured, BrandError> {
    let source = network::normalize_url(value)?;
    let deadline = Instant::now() + Duration::from_millis(8000);
    let response = network::get(source.clone(), deadline)?;
    if response.media_type.starts_with("image/") {
        return image(response, &source);
    }
    if !matches!(
        response.media_type.as_str(),
        "text/html" | "application/xhtml+xml"
    ) {
        return Err(invalid(
            "The brand page is neither HTML nor a supported image",
        ));
    }
    let page = response.url.clone();
    let mut head = Head::new(response.media_type == "application/xhtml+xml");
    response.read_head(|byte| head.push(byte))?;
    let mut last_error = None;
    // Sequential requests use one connection at a time and one total deadline.
    for candidate in icon_candidates(&head.links, &page) {
        match network::get(candidate, deadline).and_then(|response| image(response, &source)) {
            Ok(captured) => return Ok(captured),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        BrandError::new("brand/capture-unavailable", "No usable site icon was found")
    }))
}

fn image(response: network::Response, source: &Url) -> Result<Captured, BrandError> {
    let media_type = response.media_type.clone();
    let resolved_url = network::public_url(&response.url);
    let bytes = response.read(network::MAX_IMAGE_BYTES)?;
    validate_image(&media_type, &bytes)?;
    Ok(Captured {
        source_url: network::public_url(source),
        resolved_url,
        media_type,
        bytes,
    })
}

fn invalid(message: &str) -> BrandError {
    BrandError::new("brand/invalid-image", message)
}

fn dimensions(width: u32, height: u32) -> bool {
    width > 0 && height > 0 && width <= 4096 && height <= 4096
}
fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[..4].try_into().unwrap())
}
fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[..4].try_into().unwrap())
}
fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes[..2].try_into().unwrap())
}

pub fn validate_image(media_type: &str, bytes: &[u8]) -> Result<(), BrandError> {
    if bytes.is_empty() || bytes.len() > network::MAX_IMAGE_BYTES {
        return Err(invalid(
            "The brand image is empty or exceeds its byte limit",
        ));
    }
    let valid = match media_type {
        "image/png" => valid_png(bytes),
        "image/jpeg" => valid_jpeg(bytes),
        "image/webp" => valid_webp(bytes),
        "image/x-icon" | "image/vnd.microsoft.icon" => valid_ico(bytes),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "Brand image MIME, signature, dimensions or boundaries do not match",
        ))
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn valid_png(bytes: &[u8]) -> bool {
    if bytes.len() < 45 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return false;
    }
    let mut offset = 8usize;
    let mut image = false;
    while offset + 12 <= bytes.len() {
        let length = be32(&bytes[offset..]) as usize;
        let Some(end) = offset.checked_add(12).and_then(|v| v.checked_add(length)) else {
            return false;
        };
        if end > bytes.len() {
            return false;
        }
        let tag = &bytes[offset + 4..offset + 8];
        if crc32(&bytes[offset + 4..end - 4]) != be32(&bytes[end - 4..]) {
            return false;
        }
        if offset == 8 {
            if tag != b"IHDR" || length != 13 || !dimensions(be32(&bytes[16..]), be32(&bytes[20..]))
            {
                return false;
            }
            let depth = bytes[24];
            if !match bytes[25] {
                0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
                2 | 4 | 6 => matches!(depth, 8 | 16),
                3 => matches!(depth, 1 | 2 | 4 | 8),
                _ => false,
            } || bytes[26] != 0
                || bytes[27] != 0
                || bytes[28] > 1
            {
                return false;
            }
        } else if tag == b"IHDR" {
            return false;
        }
        if tag == b"IDAT" && length > 0 {
            image = true;
        }
        if tag == b"IEND" {
            return image && length == 0 && end == bytes.len();
        }
        offset = end;
    }
    false
}

fn valid_jpeg(bytes: &[u8]) -> bool {
    if bytes.len() < 20 || bytes[..2] != [0xff, 0xd8] || bytes[bytes.len() - 2..] != [0xff, 0xd9] {
        return false;
    }
    let mut offset = 2;
    let mut frame = false;
    while offset + 3 < bytes.len() {
        if bytes[offset] != 0xff {
            return false;
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        if offset >= bytes.len() {
            return false;
        }
        let marker = bytes[offset];
        offset += 1;
        if matches!(marker, 0x00 | 0xd8 | 0xd9 | 0xd0..=0xd7) {
            return false;
        }
        if marker == 0x01 {
            continue;
        }
        if offset + 2 > bytes.len() {
            return false;
        }
        let length = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        if length < 2 || offset + length > bytes.len() {
            return false;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return false;
            }
            let height =
                u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().unwrap()) as u32;
            let width =
                u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().unwrap()) as u32;
            if !dimensions(width, height) {
                return false;
            }
            frame = true;
        }
        if marker == 0xda {
            if !frame || length < 6 {
                return false;
            }
            // Entropy data may contain byte-stuffed ff00 and restart markers.
            return offset + length < bytes.len() - 2;
        }
        offset += length;
    }
    false
}

fn valid_webp(bytes: &[u8]) -> bool {
    if bytes.len() < 20
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WEBP"
        || le32(&bytes[4..]) as usize + 8 != bytes.len()
    {
        return false;
    }
    let mut offset = 12;
    let mut image = false;
    while offset + 8 <= bytes.len() {
        let length = le32(&bytes[offset + 4..]) as usize;
        let Some(end) = offset.checked_add(8).and_then(|v| v.checked_add(length)) else {
            return false;
        };
        let padded = end + (length % 2);
        if padded > bytes.len() {
            return false;
        }
        let data = &bytes[offset + 8..end];
        match &bytes[offset..offset + 4] {
            b"VP8 " => {
                if length < 10
                    || &data[3..6] != b"\x9d\x01\x2a"
                    || !dimensions(
                        (le16(&data[6..]) & 0x3fff) as u32,
                        (le16(&data[8..]) & 0x3fff) as u32,
                    )
                {
                    return false;
                }
                image = true;
            }
            b"VP8L" => {
                if length < 5 || data[0] != 0x2f {
                    return false;
                }
                let bits = le32(&data[1..]);
                if bits >> 29 != 0 || !dimensions((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)
                {
                    return false;
                }
                image = true;
            }
            b"VP8X" => {
                if length != 10 || data[0] & 0xc1 != 0 || data[1..4] != [0, 0, 0] {
                    return false;
                }
                let width = 1 + u32::from_le_bytes([data[4], data[5], data[6], 0]);
                let height = 1 + u32::from_le_bytes([data[7], data[8], data[9], 0]);
                if !dimensions(width, height) {
                    return false;
                }
            }
            _ => {}
        }
        offset = padded;
    }
    image && offset == bytes.len()
}

fn valid_ico(bytes: &[u8]) -> bool {
    if bytes.len() < 6 || bytes[..4] != [0, 0, 1, 0] {
        return false;
    }
    let count = le16(&bytes[4..]) as usize;
    let directory_end = 6 + count * 16;
    if count == 0 || directory_end > bytes.len() {
        return false;
    }
    let mut ranges = Vec::new();
    for i in 0..count {
        let entry = &bytes[6 + i * 16..6 + (i + 1) * 16];
        let size = le32(&entry[8..]) as usize;
        let offset = le32(&entry[12..]) as usize;
        let Some(end) = offset.checked_add(size) else {
            return false;
        };
        if entry[3] != 0
            || size == 0
            || offset < directory_end
            || end > bytes.len()
            || ranges
                .iter()
                .any(|&(start, end)| offset < end && start < offset + size)
        {
            return false;
        }
        let image = &bytes[offset..end];
        if image.starts_with(b"\x89PNG\r\n\x1a\n") {
            if !valid_png(image) {
                return false;
            }
            let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
            let height = if entry[1] == 0 { 256 } else { entry[1] as u32 };
            if be32(&image[16..]) != width || be32(&image[20..]) != height {
                return false;
            }
        } else {
            if size < 40 || le32(image) < 40 || le32(image) as usize > size {
                return false;
            }
            let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
            let height = if entry[1] == 0 { 256 } else { entry[1] as u32 };
            if le32(&image[4..]) != width
                || le32(&image[8..]) != height * 2
                || le16(&image[12..]) != 1
                || !matches!(le16(&image[14..]), 1 | 4 | 8 | 16 | 24 | 32)
                || !matches!(le32(&image[16..]), 0 | 3)
            {
                return false;
            }
            let depth = le16(&image[14..]) as u32;
            let palette = if depth <= 8 { (1usize << depth) * 4 } else { 0 };
            let raster = ((width * depth).div_ceil(32) * 4 * height) as usize;
            let mask = (width.div_ceil(32) * 4 * height) as usize;
            if size < le32(image) as usize + palette + raster + mask {
                return false;
            }
        }
        ranges.push((offset, end));
    }
    true
}

// A one-pass head scanner keeps comments, raw text, quoted attributes and
// template contents from becoming icon declarations or a premature head end.
struct Head {
    bytes: Vec<u8>,
    tag_start: Option<usize>,
    quote: u8,
    comment: bool,
    raw_closing: Vec<u8>,
    matched: usize,
    template_depth: usize,
    xhtml: bool,
    links: Vec<String>,
}
impl Head {
    fn new(xhtml: bool) -> Self {
        Self {
            bytes: vec![],
            tag_start: None,
            quote: 0,
            comment: false,
            raw_closing: vec![],
            matched: 0,
            template_depth: 0,
            xhtml,
            links: vec![],
        }
    }
    fn push(&mut self, byte: u8) -> bool {
        self.bytes.push(byte);
        let position = self.bytes.len() - 1;
        if self.comment {
            if self.bytes.ends_with(b"-->") {
                self.comment = false;
            }
            return false;
        }
        if !self.raw_closing.is_empty() {
            if self.matched == self.raw_closing.len()
                && (byte.is_ascii_whitespace() || matches!(byte, b'/' | b'>'))
            {
                self.tag_start = Some(position - self.matched);
                self.raw_closing.clear();
                self.matched = 0;
            } else {
                self.matched =
                    if self.raw_closing.get(self.matched) == Some(&byte.to_ascii_lowercase()) {
                        self.matched + 1
                    } else {
                        usize::from(byte == b'<')
                    };
                return false;
            }
        }
        let Some(start) = self.tag_start else {
            if byte == b'<' {
                self.tag_start = Some(position);
            }
            return false;
        };
        if position == start + 1
            && !byte.is_ascii_alphabetic()
            && !matches!(byte, b'!' | b'/' | b'?')
        {
            self.tag_start = if byte == b'<' { Some(position) } else { None };
            return false;
        }
        if position == start + 3 && &self.bytes[start..=position] == b"<!--" {
            self.comment = true;
            self.tag_start = None;
            return false;
        }
        if self.quote != 0 {
            if byte == self.quote {
                self.quote = 0;
            }
            return false;
        }
        if matches!(byte, b'\'' | b'"') {
            self.quote = byte;
            return false;
        }
        if byte != b'>' {
            return false;
        }
        let tag = String::from_utf8_lossy(&self.bytes[start..=position]).into_owned();
        let lower = tag.to_ascii_lowercase();
        let name = lower
            .trim_start_matches('<')
            .trim_start_matches('/')
            .split(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or("");
        let closing = lower.starts_with("</");
        let empty = self.xhtml && lower.ends_with("/>");
        self.tag_start = None;
        if name == "head" && closing && self.template_depth == 0 {
            return true;
        }
        if name == "body" && !closing && self.template_depth == 0 {
            return true;
        }
        if name == "template" {
            if closing {
                self.template_depth = self.template_depth.saturating_sub(1);
            } else if !empty {
                self.template_depth += 1;
            }
        } else if name == "link" && !closing && self.template_depth == 0 {
            self.links.push(tag);
        }
        if !closing
            && !empty
            && (matches!(
                name,
                "script"
                    | "style"
                    | "title"
                    | "textarea"
                    | "xmp"
                    | "iframe"
                    | "noembed"
                    | "noframes"
            ) || (!self.xhtml && name == "noscript"))
        {
            self.raw_closing = format!("</{name}").into_bytes();
            self.matched = 0;
        }
        false
    }
}

fn icon_candidates(links: &[String], page: &Url) -> Vec<Url> {
    let selector = Selector::parse("link").expect("fixed selector");
    let mut candidates = Vec::new();
    for tag in links {
        let fragment = Html::parse_fragment(tag);
        let Some(link) = fragment.select(&selector).next() else {
            continue;
        };
        let rel: Vec<_> = link
            .attr("rel")
            .unwrap_or("")
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        if !rel
            .iter()
            .any(|value| matches!(value.as_str(), "icon" | "apple-touch-icon" | "mask-icon"))
        {
            continue;
        }
        let Some(href) = link.attr("href") else {
            continue;
        };
        let Ok(url) = page.join(href) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }
        let sizes = link.attr("sizes").unwrap_or("");
        let area = sizes
            .split_ascii_whitespace()
            .filter_map(|s| s.split_once('x').or_else(|| s.split_once('X')))
            .filter_map(|(width, height)| {
                Some(
                    width
                        .parse::<u64>()
                        .ok()?
                        .saturating_mul(height.parse::<u64>().ok()?),
                )
            })
            .max()
            .unwrap_or(0)
            .min(499_999);
        let score = area
            + if rel.iter().any(|r| r == "apple-touch-icon") {
                500_000
            } else {
                0
            };
        candidates.push((score, url));
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    let fallback = page.join("/favicon.ico").expect("HTTP origin has a root");
    let mut urls = Vec::new();
    for (_, url) in candidates {
        if url != fallback && !urls.contains(&url) && urls.len() < 5 {
            urls.push(url);
        }
    }
    urls.push(fallback);
    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    const PNG: &[u8] = include_bytes!("../../tests/fixtures/brands/official-test.png");
    // Encoded from the fixed synthetic PNG with ImageMagick and cwebp;
    // these exercise real image layouts instead of invented signatures.
    const JPEG: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgICAgMCAgIDAwMDBAYEBAQEBAgGBgUGCQgKCgkICQkKDA8MCgsOCwkJDRENDg8QEBEQCgwSExIQEw8QEBD/2wBDAQMDAwQDBAgEBAgQCwkLEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBD/wAARCAAgACADAREAAhEBAxEB/8QAFgABAQEAAAAAAAAAAAAAAAAACQAI/8QAFBABAAAAAAAAAAAAAAAAAAAAAP/EABcBAQEBAQAAAAAAAAAAAAAAAAgJAAf/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwDO7uDl5HU3zDHEpAHhHU3zDTMOJSAPCOpvmGOJSAPEzEdTfMMcSkAeEdTfMNMw4lIA8I6m+YY4lIA8f//Z";
    const WEBP: &str = "UklGRi4AAABXRUJQVlA4TCEAAAAvH8AHAA9wUtjD0H+oav5jCWLBZP7S3RlE9D+SJ73kSQ8A";

    #[test]
    fn accepts_real_raster_formats_and_ico_and_rejects_type_confusion() {
        let jpeg = STANDARD.decode(JPEG).unwrap();
        let webp = STANDARD.decode(WEBP).unwrap();
        let mut ico = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        ico.extend_from_slice(&(PNG.len() as u32).to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(PNG);
        for (mime, bytes) in [
            ("image/png", PNG),
            ("image/jpeg", jpeg.as_slice()),
            ("image/webp", webp.as_slice()),
            ("image/x-icon", ico.as_slice()),
        ] {
            assert!(validate_image(mime, bytes).is_ok(), "rejected {mime}");
            assert!(
                validate_image(mime, b"<!doctype html><html><body>image</body></html>").is_err()
            );
            assert!(
                validate_image(mime, &bytes[..bytes.len() - 1]).is_err(),
                "accepted truncated {mime}"
            );
        }
        assert!(validate_image(
            "image/svg+xml",
            b"<svg xmlns='http://www.w3.org/2000/svg'/>"
        )
        .is_err());
        assert!(validate_image("image/jpeg", PNG).is_err());
        assert!(validate_image("image/png", &vec![0; network::MAX_IMAGE_BYTES + 1]).is_err());
        // ICO entries cannot point into their own directory or beyond the body.
        ico[18..22].copy_from_slice(&6u32.to_le_bytes());
        assert!(validate_image("image/x-icon", &ico).is_err());
    }

    #[test]
    fn png_rejects_bad_crc_dimensions_and_hidden_trailing_bytes() {
        let mut damaged = PNG.to_vec();
        damaged[50] ^= 1;
        assert!(validate_image("image/png", &damaged).is_err());
        let mut huge = PNG.to_vec();
        huge[16..20].copy_from_slice(&100000u32.to_be_bytes());
        let crc = crc32(&huge[12..29]);
        huge[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(validate_image("image/png", &huge).is_err());
        let mut trailing = PNG.to_vec();
        trailing.extend_from_slice(b"secret");
        assert!(validate_image("image/png", &trailing).is_err());
    }

    #[test]
    fn head_scanner_ignores_raw_text_comments_templates_and_body() {
        let html = r#"<html><head><!-- <link rel=icon href='/comment.png'> -->
            <script>const x="</head><link rel=icon href='/script.png'>";</script>
            <title>Example </head><link rel=icon href='/title.png'></title>
            <style>/* </head><link rel=icon href='/style.png'> */</style>
            <noscript><link rel=icon href='/noscript.png'></noscript>
            <template><link rel=icon href='/template.png'></template>
            <link title="</head>" rel='shortcut icon' href='/real.png?x=1&amp;token=2'>
            <link rel=icon data-href='/false.png' href='/second.png' href='/duplicate.png'>
            </head><body><link rel=icon href='/body.png'></body></html>"#;
        let mut head = Head::new(false);
        for &byte in html.as_bytes() {
            if head.push(byte) {
                break;
            }
        }
        let candidates = icon_candidates(
            &head.links,
            &Url::parse("https://brand.example/page").unwrap(),
        );
        assert_eq!(
            candidates.iter().map(Url::as_str).collect::<Vec<_>>(),
            vec![
                "https://brand.example/real.png?x=1&token=2",
                "https://brand.example/second.png",
                "https://brand.example/favicon.ico",
            ]
        );
    }

    #[test]
    fn discovers_declared_web_icons_including_cdns_with_bounded_fallback() {
        let html = r#"<head><link rel=icon href='https://other.example/icon.png'>
            <link rel=icon href='javascript:alert(1)'><link rel=icon href='/small.png' sizes='16x16'>
            <link rel=apple-touch-icon href='/apple.png'><link rel=icon href='/large.png' sizes='64x64'>
            <link rel=icon href='/large.png' sizes='64x64'></head>"#;
        let mut head = Head::new(false);
        for &byte in html.as_bytes() {
            if head.push(byte) {
                break;
            }
        }
        let page = Url::parse("https://brand.example/page").unwrap();
        let candidates = icon_candidates(&head.links, &page);
        assert_eq!(
            candidates.iter().map(Url::as_str).collect::<Vec<_>>(),
            vec![
                "https://brand.example/apple.png",
                "https://brand.example/large.png",
                "https://brand.example/small.png",
                "https://other.example/icon.png",
                "https://brand.example/favicon.ico",
            ]
        );
    }
}
