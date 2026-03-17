use ext_php_rs::prelude::*;
use parquet::basic::{BrotliLevel, Compression as ArrowCompression, GzipLevel, ZstdLevel};

#[php_enum]
#[php(name = "Parquet\\Compression")]
pub enum Compression {
    #[php(value = 0)]
    Uncompressed,
    #[php(value = 1)]
    Snappy,
    #[php(value = 2)]
    Gzip,
    #[php(value = 3)]
    Brotli,
    #[php(value = 4)]
    Lz4Raw,
    #[php(value = 5)]
    Zstd,
}

impl From<Compression> for ArrowCompression {
    fn from(c: Compression) -> Self {
        match c {
            Compression::Uncompressed => Self::UNCOMPRESSED,
            Compression::Snappy => Self::SNAPPY,
            Compression::Gzip => Self::GZIP(GzipLevel::default()),
            Compression::Brotli => Self::BROTLI(BrotliLevel::default()),
            Compression::Lz4Raw => Self::LZ4_RAW,
            Compression::Zstd => Self::ZSTD(ZstdLevel::default()),
        }
    }
}
