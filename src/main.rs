#![allow(unused_imports)]
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

mod ani;
mod cur;
use ani::AniFile;
use image::{open, DynamicImage, GenericImageView, ImageBuffer, ImageDecoder, ImageReader};

use crate::ani::AniFrame;
use crate::cur::CursorFrame;

const DURATION: u32= 100;
const HOTSPOT: (u16, u16) = (8, 9);

fn main() -> std::io::Result<()> {
    
    let image: DynamicImage = get_image("assets/cursor.png");
    let mut final_frames: Vec<AniFrame> = Vec::new();
    
    for i in 0..14 {
        let new_image = image.huerotate(i * 15);
        let (width, height) = new_image.dimensions();
        let image_data = encode_image(new_image);

        let (hotspot_x,  hotspot_y) = HOTSPOT;
        let duration = Some(DURATION);
        let aniframe = AniFrame::new(width, height, hotspot_x, hotspot_y, image_data, duration);
        final_frames.push(aniframe);
    }

    let anifile: AniFile = AniFile::new(final_frames);

    let file: File = File::create_new("final.ani")?; 
    anifile.encode(file)?;

    Ok(())
}


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
    img.write_to(&mut cur, image::ImageFormat::Ico)
        .unwrap_or_else(|err| panic!("error writing to buffer: {}", err));
    buf
}
