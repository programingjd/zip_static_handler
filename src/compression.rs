use brotli::enc::BrotliEncoderParams;
use brotli::BrotliDecompress;
use crc32fast::Hasher;
use inflate::InflateWriter;
use std::borrow::Cow;
use std::io::{Cursor, Read, Write};
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub(crate) fn decompress_entry(
    zip_file_header: ZipLocalFileHeader,
) -> crate::errors::Result<Cow<[u8]>> {
    match zip_file_header.compression_method {
        0u16 /* stored  */ => Ok(zip_file_header.compressed_data),
        8u16 /* deflate */ => Ok(Cow::Owned(inflate(
            zip_file_header.compressed_data.as_ref(),
            zip_file_header.uncompressed_size as usize,
        ))),
        _ => Err("unsupported compression".into()),
    }
}

pub(crate) fn compress_brotli(bytes: &[u8], len: usize) -> Vec<u8> {
    let params = BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    let mut out = Vec::with_capacity(len + 64);
    let mut reader = brotli::CompressorReader::with_params(bytes, 16_384, &params);
    reader.read_to_end(&mut out).expect("failed to compress");
    out
}

pub(crate) fn brotli_decompressed_crc32(bytes: &[u8]) -> Option<u32> {
    let mut cursor = Cursor::new(bytes);
    let mut crc32 = Crc32::default();
    BrotliDecompress(&mut cursor, &mut crc32).ok()?;
    Some(crc32.finalize())
}

fn inflate(bytes: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut writer = InflateWriter::new(&mut out);
    writer.write_all(bytes).expect("failed to decompress");
    writer.finish().expect("failed to decompress");
    out
}

#[derive(Default)]
struct Crc32 {
    hasher: Hasher,
}

impl Write for Crc32 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Crc32 {
    fn finalize(self) -> u32 {
        self.hasher.finalize()
    }
}
