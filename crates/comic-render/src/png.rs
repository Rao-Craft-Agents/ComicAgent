//! Pixels as a PNG, and back. An opaque strip is written as RGB; one with transparency (an icon) as RGBA. A strip's
//! script rides inside as the `comic-spec` chunk (comic-core's `spec_png`).

use crate::Raster;
use std::io::Cursor;

/// `raster` as a PNG, carrying `spec_json` when given.
pub fn encode(raster: &Raster, spec_json: Option<&str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = ::png::Encoder::new(&mut bytes, raster.width, raster.height);
        encoder.set_depth(::png::BitDepth::Eight);
        let opaque = raster.is_opaque();
        encoder.set_color(if opaque {
            ::png::ColorType::Rgb
        } else {
            ::png::ColorType::Rgba
        });
        encoder.set_compression(::png::Compression::Balanced);
        let data: Vec<u8> = if opaque {
            raster
                .rgba
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect()
        } else {
            raster.rgba.clone()
        };
        let mut writer = encoder
            .write_header()
            .expect("a PNG header writes to memory");
        writer
            .write_image_data(&data)
            .expect("PNG data writes to memory");
    }
    match spec_json {
        Some(json) => comic_core::spec_png::embed(&bytes, json),
        None => bytes,
    }
}

/// A PNG's pixels, as RGBA.
pub fn decode(bytes: &[u8]) -> Result<Raster, String> {
    let mut decoder = ::png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(::png::Transformations::EXPAND | ::png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or("the PNG is too large")?];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let data = &buffer[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        ::png::ColorType::Rgba => data.to_vec(),
        ::png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        ::png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        ::png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        ::png::ColorType::Indexed => return Err("an indexed PNG was not expanded".into()),
    };
    Ok(Raster {
        width: info.width,
        height: info.height,
        rgba,
    })
}
