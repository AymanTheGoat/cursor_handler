// lib.rs
#![allow(dead_code)]
use std::{
    fmt::Display,
    io::{self, Read, Seek, SeekFrom, Write},
};

/// A cursor frame with image data and hotspot
#[derive(Debug, Clone)]
pub struct CursorFrame {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u16,
    pub hotspot_y: u16,
    pub image_data: Vec<u8>,
}

impl CursorFrame {
    pub fn new(
        width: u32,
        height: u32,
        hotspot_x: u16,
        hotspot_y: u16,
        image_data: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            hotspot_x,
            hotspot_y,
            image_data,
        }
    }
}

impl Display for CursorFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cursor with {} frame(s):", self.frames.len())?;
        for (i, frame) in self.frames.iter().enumerate() {
            writeln!(
                f,
                "  Frame {i}:\n    Size:    {}x{}\n    Hotspot: ({}, {})",
                frame.width, frame.height, frame.hotspot_x, frame.hotspot_y
            )?;
        }
        Ok(())
    }
}

/// A cursor file containing one or more frames
#[derive(Debug, Clone)]
pub struct CursorFile {
    pub frames: Vec<CursorFrame>,
}

impl CursorFile {
    pub fn new(frames: Vec<CursorFrame>) -> Self {
        Self { frames }
    }

    pub fn single(frame: CursorFrame) -> Self {
        Self {
            frames: vec![frame],
        }
    }

    /// Encode cursor to writer
    pub fn encode<W: Write>(&self, mut writer: W) -> io::Result<()> {
        if self.frames.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No frames"));
        }

        // Write header
        writer.write_all(&[0, 0, 2, 0])?; // reserved=0, type=2 (cursor)
        writer.write_all(&(self.frames.len() as u16).to_le_bytes())?;

        // Calculate directory size
        let dir_size = 6 + (self.frames.len() * 16);
        let mut offset = dir_size as u32;

        // Write directory entries
        for frame in &self.frames {
            let width_byte = if frame.width == 256 {
                0
            } else {
                frame.width as u8
            };
            let height_byte = if frame.height == 256 {
                0
            } else {
                frame.height as u8
            };

            writer.write_all(&[width_byte, height_byte, 0, 0])?; // width, height, colors, reserved
            writer.write_all(&frame.hotspot_x.to_le_bytes())?;
            writer.write_all(&frame.hotspot_y.to_le_bytes())?;
            writer.write_all(&(frame.image_data.len() as u32).to_le_bytes())?;
            writer.write_all(&offset.to_le_bytes())?;

            offset += frame.image_data.len() as u32;
        }

        // Write image data
        for frame in &self.frames {
            writer.write_all(&frame.image_data)?;
        }

        Ok(())
    }

    /// Decode cursor from reader
    pub fn decode<R: Read + Seek>(mut reader: R) -> io::Result<Self> {
        // Read header
        let mut header = [0u8; 6];
        reader.read_exact(&mut header)?;

        if u16::from_le_bytes([header[2], header[3]]) != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a cursor file",
            ));
        }

        let count = u16::from_le_bytes([header[4], header[5]]) as usize;
        if count == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "No frames"));
        }

        // Read directory entries
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let mut entry = [0u8; 16];
            reader.read_exact(&mut entry)?;

            let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
            let height = if entry[1] == 0 { 256 } else { entry[1] as u32 };
            let hotspot_x = u16::from_le_bytes([entry[4], entry[5]]);
            let hotspot_y = u16::from_le_bytes([entry[6], entry[7]]);
            let size = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
            let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);

            entries.push((width, height, hotspot_x, hotspot_y, size, offset));
        }

        // Read image data
        let mut frames = Vec::with_capacity(count);
        for (width, height, hotspot_x, hotspot_y, size, offset) in entries {
            reader.seek(SeekFrom::Start(offset as u64))?;
            let mut image_data = vec![0u8; size as usize];
            reader.read_exact(&mut image_data)?;

            frames.push(CursorFrame {
                width,
                height,
                hotspot_x,
                hotspot_y,
                image_data,
            });
        }

        Ok(Self { frames })
    }
}
