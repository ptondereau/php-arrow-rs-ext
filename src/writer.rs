use std::collections::HashMap;
use std::fs::File;
use std::io::Cursor;
use std::sync::Arc;

use arrow::record_batch::RecordBatchWriter;
use ext_php_rs::binary::Binary;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression as ArrowCompression;
use parquet::basic::Encoding as ArrowEncoding;
use parquet::file::properties::WriterProperties;
use parquet::schema::types::ColumnPath;

use crate::compression::Compression;
use crate::encoding::Encoding;
use crate::exception::ParquetException;
use crate::schema::Schema;
use crate::types::rows_to_record_batch;

#[php_class]
#[php(name = "Parquet\\WriterOptions")]
#[derive(Debug, Clone)]
pub struct WriterOptions {
    compression: ArrowCompression,
    row_group_size: usize,
    page_size: usize,
    column_compressions: HashMap<String, ArrowCompression>,
    column_encodings: HashMap<String, ArrowEncoding>,
}

impl WriterOptions {
    pub(crate) fn to_writer_properties(&self) -> WriterProperties {
        let mut builder = WriterProperties::builder()
            .set_compression(self.compression)
            .set_max_row_group_row_count(Some(self.row_group_size))
            .set_data_page_size_limit(self.page_size);

        for (col, comp) in &self.column_compressions {
            builder = builder.set_column_compression(ColumnPath::from(col.as_str()), *comp);
        }
        for (col, enc) in &self.column_encodings {
            builder = builder.set_column_encoding(ColumnPath::from(col.as_str()), *enc);
        }

        builder.build()
    }
}

#[php_impl]
#[php(change_method_case = "none")]
impl WriterOptions {
    pub fn create() -> Self {
        Self {
            compression: ArrowCompression::UNCOMPRESSED,
            row_group_size: 128 * 1024 * 1024,
            page_size: 1024 * 1024,
            column_compressions: HashMap::new(),
            column_encodings: HashMap::new(),
        }
    }

    #[php(name = "withCompression")]
    pub fn with_compression(&self, compression: Compression) -> Self {
        Self {
            compression: compression.into(),
            ..self.clone()
        }
    }

    #[php(name = "withColumnCompression")]
    pub fn with_column_compression(&self, column: String, compression: Compression) -> Self {
        let mut opts = self.clone();
        opts.column_compressions.insert(column, compression.into());
        opts
    }

    #[php(name = "withColumnEncoding")]
    pub fn with_column_encoding(&self, column: String, encoding: Encoding) -> Self {
        let mut opts = self.clone();
        opts.column_encodings.insert(column, encoding.into());
        opts
    }

    #[php(name = "withRowGroupSize")]
    pub fn with_row_group_size(&self, bytes: i64) -> PhpResult<Self> {
        if bytes <= 0 {
            return Err(PhpException::from_class::<ParquetException>(format!(
                "Invalid rowGroupSize: {bytes}, must be positive"
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self {
            row_group_size: bytes as usize,
            ..self.clone()
        })
    }

    #[php(name = "withPageSize")]
    pub fn with_page_size(&self, bytes: i64) -> PhpResult<Self> {
        if bytes <= 0 {
            return Err(PhpException::from_class::<ParquetException>(format!(
                "Invalid pageSize: {bytes}, must be positive"
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self {
            page_size: bytes as usize,
            ..self.clone()
        })
    }
}

enum WriterState {
    Closed,
    FileOpen {
        writer: Box<ArrowWriter<File>>,
        schema: Arc<arrow::datatypes::Schema>,
    },
    BufferOpen {
        writer: Box<ArrowWriter<Cursor<Vec<u8>>>>,
        schema: Arc<arrow::datatypes::Schema>,
    },
}

#[php_class]
#[php(name = "Parquet\\Writer")]
pub struct Writer {
    options: WriterOptions,
    state: WriterState,
    buffer: Option<Vec<u8>>,
}

#[php_impl]
#[php(change_method_case = "none")]
impl Writer {
    pub fn __construct(options: Option<&WriterOptions>) -> Self {
        Self {
            options: options.cloned().unwrap_or_else(WriterOptions::create),
            state: WriterState::Closed,
            buffer: None,
        }
    }

    pub fn write(&mut self, path: String, schema: &Schema, rows: Vec<&Zval>) -> PhpResult<()> {
        self.open(path, schema)?;
        self.write_batch(rows)?;
        self.close()
    }

    pub fn open(&mut self, path: String, schema: &Schema) -> PhpResult<()> {
        if !matches!(self.state, WriterState::Closed) {
            return Err(PhpException::from_class::<ParquetException>(
                "Writer is already open".to_owned(),
            ));
        }

        let arrow_schema = Arc::new(schema.to_arrow_schema());
        let props = self.options.to_writer_properties();

        let file = File::create(&path).map_err(|e| {
            PhpException::from_class::<ParquetException>(format!("Failed to create file: {e}"))
        })?;

        let writer =
            ArrowWriter::try_new(file, Arc::clone(&arrow_schema), Some(props)).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!(
                    "Failed to create Parquet writer: {e}"
                ))
            })?;

        self.state = WriterState::FileOpen {
            writer: Box::new(writer),
            schema: arrow_schema,
        };
        Ok(())
    }

    #[php(name = "openBuffer")]
    pub fn open_buffer(&mut self, schema: &Schema) -> PhpResult<()> {
        if !matches!(self.state, WriterState::Closed) {
            return Err(PhpException::from_class::<ParquetException>(
                "Writer is already open".to_owned(),
            ));
        }

        let arrow_schema = Arc::new(schema.to_arrow_schema());
        let props = self.options.to_writer_properties();
        let cursor = Cursor::new(Vec::new());

        let writer =
            ArrowWriter::try_new(cursor, Arc::clone(&arrow_schema), Some(props)).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!(
                    "Failed to create Parquet writer: {e}"
                ))
            })?;

        self.state = WriterState::BufferOpen {
            writer: Box::new(writer),
            schema: arrow_schema,
        };
        self.buffer = None;
        Ok(())
    }

    #[php(name = "writeToString")]
    pub fn write_to_string(
        &mut self,
        schema: &Schema,
        rows: Vec<&Zval>,
    ) -> PhpResult<Binary<u8>> {
        self.open_buffer(schema)?;
        self.write_batch(rows)?;
        self.close()?;
        self.buffer
            .take()
            .map(Binary::from)
            .ok_or_else(|| {
                PhpException::from_class::<ParquetException>(
                    "Failed to retrieve buffer after close".to_owned(),
                )
            })
    }

    #[php(name = "getBuffer")]
    pub fn get_buffer(&self) -> PhpResult<Binary<u8>> {
        self.buffer
            .clone()
            .map(Binary::from)
            .ok_or_else(|| {
                PhpException::from_class::<ParquetException>(
                    "No buffer available. Call close() after writing to a buffer.".to_owned(),
                )
            })
    }

    #[php(name = "writeBatch")]
    pub fn write_batch(&mut self, rows: Vec<&Zval>) -> PhpResult<()> {
        if rows.is_empty() {
            return Ok(());
        }

        match &mut self.state {
            WriterState::FileOpen { writer, schema } => {
                let batch = rows_to_record_batch(&rows, schema)?;
                writer.write(&batch).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to write batch: {e}"
                    ))
                })
            }
            WriterState::BufferOpen { writer, schema } => {
                let batch = rows_to_record_batch(&rows, schema)?;
                writer.write(&batch).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to write batch: {e}"
                    ))
                })
            }
            WriterState::Closed => Err(PhpException::from_class::<ParquetException>(
                "Writer is not open".to_owned(),
            )),
        }
    }

    pub fn close(&mut self) -> PhpResult<()> {
        let state = std::mem::replace(&mut self.state, WriterState::Closed);
        match state {
            WriterState::FileOpen { writer, .. } => {
                RecordBatchWriter::close(*writer).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to close writer: {e}"
                    ))
                })?;
                Ok(())
            }
            WriterState::BufferOpen { writer, .. } => {
                let cursor = writer.into_inner().map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to close writer: {e}"
                    ))
                })?;
                self.buffer = Some(cursor.into_inner());
                Ok(())
            }
            WriterState::Closed => Err(PhpException::from_class::<ParquetException>(
                "Writer is already closed".to_owned(),
            )),
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if !matches!(self.state, WriterState::Closed) {
            let _ = self.close();
        }
    }
}
