#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;
use ext_php_rs::zend::ModuleEntry;
use ext_php_rs::{info_table_end, info_table_row, info_table_start};

mod column;
mod compression;
mod encoding;
mod exception;
mod metadata;
mod reader;
mod schema;
mod types;
mod writer;

#[allow(clippy::must_use_candidate)]
pub fn startup(_ty: i32, _mod_num: i32) -> i32 {
    0
}

pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
    info_table_start!();
    info_table_row!("ext-parquet", "enabled");
    info_table_row!("parquet version", env!("CARGO_PKG_VERSION"));
    info_table_end!();
}

#[php_module]
#[php(startup = "startup")]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .info_function(php_module_info)
        .enumeration::<compression::Compression>()
        .enumeration::<encoding::Encoding>()
        .class::<exception::ParquetException>()
        .class::<column::Column>()
        .class::<schema::Schema>()
        .class::<writer::WriterOptions>()
        .class::<writer::Writer>()
        .class::<metadata::Metadata>()
        .class::<reader::ReaderOptions>()
        .class::<reader::Reader>()
        .class::<reader::ParquetFile>()
}
