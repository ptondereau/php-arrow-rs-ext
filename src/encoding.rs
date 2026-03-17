use ext_php_rs::prelude::*;
use parquet::basic::Encoding as ArrowEncoding;

#[php_enum]
#[php(name = "Parquet\\Encoding")]
pub enum Encoding {
    #[php(value = 0)]
    Plain,
    #[php(value = 1)]
    PlainDictionary,
    #[php(value = 2)]
    RleDictionary,
    #[php(value = 3)]
    DeltaBinaryPacked,
    #[php(value = 4)]
    DeltaLengthByteArray,
    #[php(value = 5)]
    DeltaByteArray,
    #[php(value = 6)]
    ByteStreamSplit,
}

impl From<Encoding> for ArrowEncoding {
    fn from(e: Encoding) -> Self {
        match e {
            Encoding::Plain => Self::PLAIN,
            #[allow(deprecated)]
            Encoding::PlainDictionary => Self::PLAIN_DICTIONARY,
            Encoding::RleDictionary => Self::RLE_DICTIONARY,
            Encoding::DeltaBinaryPacked => Self::DELTA_BINARY_PACKED,
            Encoding::DeltaLengthByteArray => Self::DELTA_LENGTH_BYTE_ARRAY,
            Encoding::DeltaByteArray => Self::DELTA_BYTE_ARRAY,
            Encoding::ByteStreamSplit => Self::BYTE_STREAM_SPLIT,
        }
    }
}
