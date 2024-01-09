// Joseph Prichard
// 1/5/2023
// File writer implementing a bit layer

use std::fs::{File, OpenOptions};
use std::io::{Write};
use std::{io, mem};
use crate::bitwise;
use crate::bitwise::{SymbolCode};
use crate::data::{FileBlock};

const BUFFER_LEN: usize = 4096;
const BUFFER_BIT_LEN: u32 = (BUFFER_LEN * 8) as u32;

pub trait BitwiseWriter {
    fn align_to_byte(&mut self) -> io::Result<()>;
    fn write_byte(&mut self, byte: u8) -> io::Result<()>;
    fn write_bits(&mut self, byte: u8, count: u8) -> io::Result<()>;
    fn write_bit(&mut self, bit: u8) -> io::Result<()>;
    fn write_symbol(&mut self, symbol: &SymbolCode) -> io::Result<()>;
    fn write_block(&mut self, block: &FileBlock) -> io::Result<()>;
    fn write_u64(&mut self, num: u64) -> io::Result<()>;
}

pub struct FileWriter {
    // the file stream to write to
    file: File,
    // a buffer storing a block to be written to the file
    buffer: [u8; BUFFER_LEN],
    // the bit position of the last write in the buffer
    bit_position: u32,
}

impl FileWriter {
    pub fn new(filepath: &str) -> io::Result<FileWriter> {
        Ok(FileWriter {
            file: OpenOptions::new()
                .write(true)
                .append(false)
                .create(true)
                .open(filepath)?,
            buffer: [0u8; BUFFER_LEN],
            bit_position: 0,
        })
    }

    fn persist_buffer(&mut self) -> io::Result<()> {
        self.file.write(&self.buffer[0..((self.bit_position / 8) as usize)])?;
        Ok(())
    }

    fn update_buffer(&mut self) -> io::Result<()> {
        // check if at end of buffer: persist current buffer and start writing on a new one
        if self.bit_position >= BUFFER_BIT_LEN {
            self.persist_buffer()?;
            self.bit_position = 0;
            self.buffer = [0u8; BUFFER_LEN];
        }
        Ok(())
    }
}

impl BitwiseWriter for FileWriter {
    fn align_to_byte(&mut self) -> io::Result<()> {
        self.bit_position = ((self.bit_position + 7) / 8) * 8;
        Ok(())
    }

    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.update_buffer()?;

        // write the byte directly into the buffer
        self.buffer[(self.bit_position / 8) as usize] = byte;
        self.bit_position += 8;

        Ok(())
    }

    fn write_bits(&mut self, byte: u8, count: u8) -> io::Result<()> {
        // write each bit individually as they might end up in different bytes in the buffer
        for i in 0..count {
            let bit = bitwise::get_bit(byte as u32, i as u32);
            self.write_bit(bit)?;
        }
        Ok(())
    }

    fn write_bit(&mut self, bit: u8) -> io::Result<()> {
        self.update_buffer()?;

        // write the bit back into the buffer
        if bit > 0 {
            let i = (self.bit_position / 8) as usize;
            self.buffer[i] = bitwise::set_bit(self.buffer[i] as u32, self.bit_position % 8);
        }

        self.bit_position += 1;
        Ok(())
    }

    fn write_symbol(&mut self, symbol: &SymbolCode) -> io::Result<()> {
        for i in 0..symbol.bit_len {
            let bit = bitwise::get_bit(symbol.encoded_symbol, i as u32);
            self.write_bit(bit)?;
        }
        Ok(())
    }

    fn write_block(&mut self, block: &FileBlock) -> io::Result<()> {
        // write string with a null terminator at the end
        for c in block.filename_rel.chars() {
            self.write_byte(c as u8)?;
        }
        self.write_byte(0)?;
        // write each u64 field into the file
        self.write_u64(block.fbs.tree_bit_size)?;
        self.write_u64(block.fbs.data_bit_size)?;
        self.write_u64(block.file_byte_offset)?;
        self.write_u64(block.og_byte_size)?;
        Ok(())
    }

    fn write_u64(&mut self, num: u64) -> io::Result<()> {
        let buffer: [u8; 8] = unsafe { mem::transmute(num) };
        for i in 0..8 {
            self.write_byte(buffer[i])?;
        }
        Ok(())
    }
}

impl Drop for FileWriter {
    fn drop(&mut self) {
        if let Err(e) = self.persist_buffer() {
            panic!("Fatal: failed to write the buffer to file when dropping: {}", e.to_string());
        }
    }
}