// mod codecs;

// use codecs::cursor::CursorDecoder;
mod cur;
use std::{
    fs::File,
    io::{Cursor, Write},
};

use cur::CursorFile;
use image::{DynamicImage, ImageReader};

fn main() -> std::io::Result<()> {
    let file = File::open("output.cur")?;
    let cur = CursorFile::decode(file)?;

    for png in &cur.frames {
        let path = format!("test/test {}x{}.png", png.width, png.height);
        let mut file = File::create(path)?;
        file.write_all(&png.image_data)?;
    }

    println!("{cur}");

    Ok(())
}


// fn hue_shift(img: DynamicImage, degree: u32) {
//     img.huerotate(degree)
// }

#[allow(dead_code)]
fn get_image(path: &str) -> DynamicImage {
    ImageReader::open(path)
        .unwrap_or_else(|err| panic!("error reading image: {err}"))
        .decode()
        .unwrap_or_else(|err| panic!("error decoding image: {err}"))
}

#[allow(dead_code)]
fn encode_image(img: DynamicImage) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut cur = Cursor::new(&mut buf);
    img.write_to(&mut cur, image::ImageFormat::Png)
        .unwrap_or_else(|err| panic!("error writing to buffer: {}", err));
    buf
}
