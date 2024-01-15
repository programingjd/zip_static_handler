use brotli::enc::BrotliEncoderParams;
use inflate::InflateWriter;
use std::io::{Read, Write};

pub(crate) fn brotli(bytes: &[u8], len: usize) -> Vec<u8> {
    let params = BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    let mut out = Vec::with_capacity(len + 64);
    let mut reader = brotli::CompressorReader::with_params(bytes, 16_384, &params);
    reader.read_to_end(&mut out).expect("failed to compress");
    out
}

pub(crate) fn inflate(bytes: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut writer = InflateWriter::new(&mut out);
    writer.write_all(bytes).expect("failed to decompress");
    writer.finish().expect("failed to decompress");
    out
}
