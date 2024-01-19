use crate::errors::Error;
use brotli::enc::BrotliEncoderParams;
use inflate::InflateWriter;
use std::borrow::Cow;
use std::io::{Read, Write};
use zip_structs::zip_local_file_header::ZipLocalFileHeader;

pub(crate) fn decompress(zip_file_header: ZipLocalFileHeader) -> crate::errors::Result<Cow<[u8]>> {
    match zip_file_header.compression_method {
        0u16 /* stored  */ => Ok(zip_file_header.compressed_data),
        8u16 /* deflate */ => Ok(Cow::Owned(inflate(
            zip_file_header.compressed_data.as_ref(),
            zip_file_header.uncompressed_size as usize,
        ))),
        _ => Err(Error::Message("unsupported compression")),
    }
}

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

fn inflate(bytes: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut writer = InflateWriter::new(&mut out);
    writer.write_all(bytes).expect("failed to decompress");
    writer.finish().expect("failed to decompress");
    out
}
