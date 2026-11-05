//! The script a strip was drawn from, carried inside its PNG: an uncompressed `iTXt` chunk keyed `comic-spec`, before
//! `IEND`. One file is the picture and what draws it again, so a project gains a PNG and nothing a build might read.
//! Read without decoding the picture: the chunks are walked.

/// The `iTXt` keyword.
pub const KEYWORD: &str = "comic-spec";

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// `png` with `json` in an `iTXt` chunk before its end; `png` unchanged when it is not a PNG.
pub fn embed(png: &[u8], json: &str) -> Vec<u8> {
    let Some(end) = chunks(png).find(|c| &c.kind == b"IEND").map(|c| c.offset) else {
        return png.to_vec();
    };
    let mut body = KEYWORD.as_bytes().to_vec();
    body.extend_from_slice(&[0, 0, 0, 0, 0]); // keyword's end, not compressed, method, no language, no translation
    body.extend_from_slice(json.as_bytes());
    let mut typed = b"iTXt".to_vec();
    typed.extend_from_slice(&body);
    let mut out = Vec::with_capacity(png.len() + typed.len() + 8);
    out.extend_from_slice(&png[..end]);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&typed);
    out.extend_from_slice(&crc32(&typed).to_be_bytes());
    out.extend_from_slice(&png[end..]);
    out
}

/// The script inside a PNG, when it carries one.
pub fn embedded(png: &[u8]) -> Option<String> {
    chunks(png).filter(|c| &c.kind == b"iTXt").find_map(|c| {
        let data = &png[c.offset + 8..c.offset + 8 + c.length];
        let rest = data.strip_prefix(KEYWORD.as_bytes())?.strip_prefix(&[0])?;
        let (&compressed, rest) = rest.split_first()?;
        let rest = rest.get(1..)?; // the method
        let language_end = rest.iter().position(|&b| b == 0)?;
        let rest = &rest[language_end + 1..];
        let translated_end = rest.iter().position(|&b| b == 0)?;
        let text = &rest[translated_end + 1..];
        (compressed == 0)
            .then(|| String::from_utf8(text.to_vec()).ok())
            .flatten()
    })
}

/// Whether the bytes start as a PNG does.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&SIGNATURE)
}

struct Chunk {
    offset: usize,
    length: usize,
    kind: [u8; 4],
}

fn chunks(png: &[u8]) -> impl Iterator<Item = Chunk> + '_ {
    let mut offset = if is_png(png) {
        SIGNATURE.len()
    } else {
        png.len()
    };
    std::iter::from_fn(move || {
        let header = png.get(offset..offset + 8)?;
        let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = [header[4], header[5], header[6], header[7]];
        let end = offset.checked_add(12)?.checked_add(length)?;
        if end > png.len() {
            return None;
        }
        let chunk = Chunk {
            offset,
            length,
            kind,
        };
        offset = end;
        Some(chunk)
    })
}

/// CRC-32 as PNG uses it (polynomial 0xEDB88320).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
