// lib.rs
#![allow(dead_code)]
use std::{
    fmt::Display,
    io::{self, Read, Seek, SeekFrom, Write},
};

/// A single frame in an animated cursor
#[derive(Debug, Clone)]
pub struct AniFrame {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u16,
    pub hotspot_y: u16,
    pub image_data: Vec<u8>,
    pub duration: Option<u32>, // Duration in 1/60th of a second (jiffies)
}

impl AniFrame {
    pub fn new(
        width: u32,
        height: u32,
        hotspot_x: u16,
        hotspot_y: u16,
        image_data: Vec<u8>,
        duration: Option<u32>,
    ) -> Self {
        Self {
            width,
            height,
            hotspot_x,
            hotspot_y,
            image_data,
            duration,
        }
    }
}

/// Animation header information
#[derive(Debug, Clone)]
pub struct AniHeader {
    pub num_frames: u32,
    pub num_steps: u32,
    pub width: u32,
    pub height: u32,
    pub bit_count: u32,
    pub planes: u32,
    pub default_rate: u32,
    pub flags: u32,
}

impl AniHeader {
    const SIZE: usize = 36;
    
    fn new() -> Self {
        Self {
            num_frames: 0,
            num_steps: 0,
            width: 0,
            height: 0,
            bit_count: 0,
            planes: 0,
            default_rate: 6, // Default 60/6 = 10 FPS
            flags: 0,
        }
    }
}

/// An animated cursor file
#[derive(Debug, Clone)]
pub struct AniFile {
    pub header: AniHeader,
    pub frames: Vec<AniFrame>,
    pub sequence: Vec<u32>, // Frame sequence indices
    pub rates: Vec<u32>,    // Individual frame rates (optional)
}

impl AniFile {
    pub fn new(frames: Vec<AniFrame>) -> Self {
        let num_frames = frames.len() as u32;
        let sequence: Vec<u32> = (0..num_frames).collect();
        
        let mut header = AniHeader::new();
        header.num_frames = num_frames;
        header.num_steps = num_frames;
        
        if let Some(first_frame) = frames.first() {
            header.width = first_frame.width;
            header.height = first_frame.height;
            header.bit_count = 32; // Assume 32-bit
            header.planes = 1;
        }
        
        Self {
            header,
            frames,
            sequence,
            rates: Vec::new(),
        }
    }

    pub fn with_sequence(mut self, sequence: Vec<u32>) -> Self {
        self.header.num_steps = sequence.len() as u32;
        self.sequence = sequence;
        self
    }

    pub fn with_rates(mut self, rates: Vec<u32>) -> Self {
        self.rates = rates;
        self
    }

    /// Encode ANI file to writer
    pub fn encode<W: Write + Seek>(&self, mut writer: W) -> io::Result<()> {
        if self.frames.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No frames"));
        }

        // Write RIFF header
        writer.write_all(b"RIFF")?;
        let file_size_pos = writer.stream_position()?;
        writer.write_all(&[0u8; 4])?; // Placeholder for file size
        writer.write_all(b"ACON")?;

        let start_pos = writer.stream_position()?;

        // Write animation header
        writer.write_all(b"anih")?;
        writer.write_all(&(AniHeader::SIZE as u32).to_le_bytes())?;
        writer.write_all(&(AniHeader::SIZE as u32).to_le_bytes())?; // Structure size
        writer.write_all(&self.header.num_frames.to_le_bytes())?;
        writer.write_all(&self.header.num_steps.to_le_bytes())?;
        writer.write_all(&self.header.width.to_le_bytes())?;
        writer.write_all(&self.header.height.to_le_bytes())?;
        writer.write_all(&self.header.bit_count.to_le_bytes())?;
        writer.write_all(&self.header.planes.to_le_bytes())?;
        writer.write_all(&self.header.default_rate.to_le_bytes())?;
        writer.write_all(&self.header.flags.to_le_bytes())?;

        // Write sequence if different from default
        if self.sequence != (0..self.header.num_frames).collect::<Vec<_>>() {
            writer.write_all(b"seq ")?;
            let seq_size = (self.sequence.len() * 4) as u32;
            writer.write_all(&seq_size.to_le_bytes())?;
            for &index in &self.sequence {
                writer.write_all(&index.to_le_bytes())?;
            }
        }

        // Write rates if provided
        if !self.rates.is_empty() {
            writer.write_all(b"rate")?;
            let rates_size = (self.rates.len() * 4) as u32;
            writer.write_all(&rates_size.to_le_bytes())?;
            for &rate in &self.rates {
                writer.write_all(&rate.to_le_bytes())?;
            }
        }

        // Write LIST chunk with icons
        writer.write_all(b"LIST")?;
        let list_size_pos = writer.stream_position()?;
        writer.write_all(&[0u8; 4])?; // Placeholder for LIST size
        writer.write_all(b"fram")?;
        let list_start = writer.stream_position()?;

        // Write each frame as an icon
        for frame in &self.frames {
            writer.write_all(b"icon")?;
            writer.write_all(&(frame.image_data.len() as u32).to_le_bytes())?;
            writer.write_all(&frame.image_data)?;
            
            // Pad to even boundary
            if frame.image_data.len() % 2 != 0 {
                writer.write_all(&[0u8])?;
            }
        }

        // Update LIST size
        let list_end = writer.stream_position()?;
        let list_size = (list_end - list_start) as u32;
        writer.seek(SeekFrom::Start(list_size_pos))?;
        writer.write_all(&list_size.to_le_bytes())?;

        // Update file size
        let file_end = writer.stream_position()?;
        let file_size = (file_end - start_pos + 4) as u32; // +4 for ACON
        writer.seek(SeekFrom::Start(file_size_pos))?;
        writer.write_all(&file_size.to_le_bytes())?;

        Ok(())
    }

    /// Decode ANI file from reader
    pub fn decode<R: Read + Seek>(mut reader: R) -> io::Result<Self> {
        // Read RIFF header
        let mut riff_header = [0u8; 12];
        reader.read_exact(&mut riff_header)?;

        if &riff_header[0..4] != b"RIFF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a RIFF file",
            ));
        }

        if &riff_header[8..12] != b"ACON" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not an ANI file",
            ));
        }

        let mut header = AniHeader::new();
        let mut sequence = Vec::new();
        let mut rates = Vec::new();
        let mut frames = Vec::new();

        // Read chunks
        loop {
            let mut chunk_header = [0u8; 8];
            if reader.read_exact(&mut chunk_header).is_err() {
                break;
            }

            let chunk_id = &chunk_header[0..4];
            let chunk_size = u32::from_le_bytes([
                chunk_header[4],
                chunk_header[5],
                chunk_header[6],
                chunk_header[7],
            ]);

            match chunk_id {
                b"anih" => {
                    let mut header_data = vec![0u8; chunk_size as usize];
                    reader.read_exact(&mut header_data)?;

                    if header_data.len() >= 36 {
                        header.num_frames = u32::from_le_bytes([
                            header_data[4],
                            header_data[5],
                            header_data[6],
                            header_data[7],
                        ]);
                        header.num_steps = u32::from_le_bytes([
                            header_data[8],
                            header_data[9],
                            header_data[10],
                            header_data[11],
                        ]);
                        header.width = u32::from_le_bytes([
                            header_data[12],
                            header_data[13],
                            header_data[14],
                            header_data[15],
                        ]);
                        header.height = u32::from_le_bytes([
                            header_data[16],
                            header_data[17],
                            header_data[18],
                            header_data[19],
                        ]);
                        header.bit_count = u32::from_le_bytes([
                            header_data[20],
                            header_data[21],
                            header_data[22],
                            header_data[23],
                        ]);
                        header.planes = u32::from_le_bytes([
                            header_data[24],
                            header_data[25],
                            header_data[26],
                            header_data[27],
                        ]);
                        header.default_rate = u32::from_le_bytes([
                            header_data[28],
                            header_data[29],
                            header_data[30],
                            header_data[31],
                        ]);
                        header.flags = u32::from_le_bytes([
                            header_data[32],
                            header_data[33],
                            header_data[34],
                            header_data[35],
                        ]);
                    }
                }
                b"seq " => {
                    let mut seq_data = vec![0u8; chunk_size as usize];
                    reader.read_exact(&mut seq_data)?;
                    
                    for chunk in seq_data.chunks_exact(4) {
                        sequence.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    }
                }
                b"rate" => {
                    let mut rate_data = vec![0u8; chunk_size as usize];
                    reader.read_exact(&mut rate_data)?;
                    
                    for chunk in rate_data.chunks_exact(4) {
                        rates.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    }
                }
                b"LIST" => {
                    let mut list_type = [0u8; 4];
                    reader.read_exact(&mut list_type)?;
                    
                    if &list_type == b"fram" {
                        let remaining_size = (chunk_size - 4) as u64;
                        let list_start = reader.stream_position()?;
                        
                        while reader.stream_position()? < list_start + remaining_size {
                            let mut icon_header = [0u8; 8];
                            if reader.read_exact(&mut icon_header).is_err() {
                                break;
                            }
                            
                            if &icon_header[0..4] == b"icon" {
                                let icon_size = u32::from_le_bytes([
                                    icon_header[4],
                                    icon_header[5],
                                    icon_header[6],
                                    icon_header[7],
                                ]);
                                
                                let mut icon_data = vec![0u8; icon_size as usize];
                                reader.read_exact(&mut icon_data)?;
                                
                                // Parse ICO/CUR data to get dimensions and hotspot
                                let frame = Self::parse_cursor_data(&icon_data)?;
                                frames.push(frame);
                                
                                // Skip padding
                                if icon_size % 2 != 0 {
                                    let mut pad = [0u8; 1];
                                    let _ = reader.read_exact(&mut pad);
                                }
                            }
                        }
                    } else {
                        // Skip unknown LIST
                        reader.seek(SeekFrom::Current((chunk_size - 4) as i64))?;
                    }
                }
                _ => {
                    // Skip unknown chunk
                    reader.seek(SeekFrom::Current(chunk_size as i64))?;
                }
            }

            // Handle padding
            if chunk_size % 2 != 0 {
                let mut pad = [0u8; 1];
                let _ = reader.read_exact(&mut pad);
            }
        }

        // Use default sequence if none provided
        if sequence.is_empty() {
            sequence = (0..header.num_frames).collect();
        }

        Ok(Self {
            header,
            frames,
            sequence,
            rates,
        })
    }

    fn parse_cursor_data(data: &[u8]) -> io::Result<AniFrame> {
        if data.len() < 22 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid cursor data",
            ));
        }

        // Skip ICO header (6 bytes) and read first directory entry (16 bytes)
        let width = if data[6] == 0 { 256 } else { data[6] as u32 };
        let height = if data[7] == 0 { 256 } else { data[7] as u32 };
        let hotspot_x = u16::from_le_bytes([data[10], data[11]]);
        let hotspot_y = u16::from_le_bytes([data[12], data[13]]);

        Ok(AniFrame {
            width,
            height,
            hotspot_x,
            hotspot_y,
            image_data: data.to_vec(),
            duration: None,
        })
    }
}

impl Display for AniFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Animated Cursor with {} frame(s):", self.frames.len())?;
        writeln!(f, "  Steps: {}", self.header.num_steps)?;
        writeln!(f, "  Size: {}x{}", self.header.width, self.header.height)?;
        writeln!(f, "  Default Rate: {} jiffies", self.header.default_rate)?;
        writeln!(f, "  Sequence: {:?}", self.sequence)?;
        
        if !self.rates.is_empty() {
            writeln!(f, "  Individual Rates: {:?}", self.rates)?;
        }
        
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