use std::fs::File;

use bytes::Bytes;
use ext_php_rs::binary::Binary;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};

use crate::exception::ParquetException;
use crate::metadata::Metadata;
use crate::schema::Schema;
use crate::types::record_batch_to_rows;

type BatchRows = Vec<Vec<(String, Zval)>>;

#[php_class]
#[php(name = "Parquet\\ReaderOptions")]
#[derive(Debug, Clone)]
pub struct ReaderOptions {
    batch_size: usize,
    columns: Option<Vec<String>>,
}

#[php_impl]
#[php(change_method_case = "none")]
impl ReaderOptions {
    pub fn create() -> Self {
        Self {
            batch_size: 8192,
            columns: None,
        }
    }

    #[php(name = "withBatchSize")]
    pub fn with_batch_size(&self, size: i64) -> PhpResult<Self> {
        if size <= 0 {
            return Err(PhpException::from_class::<ParquetException>(format!(
                "Invalid batchSize: {size}, must be positive"
            )));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self {
            batch_size: size as usize,
            ..self.clone()
        })
    }

    #[php(name = "withColumns")]
    pub fn with_columns(&self, columns: Vec<String>) -> Self {
        Self {
            columns: Some(columns),
            ..self.clone()
        }
    }
}

#[php_class]
#[php(name = "Parquet\\Reader")]
pub struct Reader {
    options: ReaderOptions,
}

impl Reader {
    #[allow(clippy::cast_possible_wrap)]
    fn build_parquet_file<T: parquet::file::reader::ChunkReader + 'static>(
        &self,
        builder: ParquetRecordBatchReaderBuilder<T>,
    ) -> PhpResult<ParquetFile> {
        let file_metadata = builder.metadata().file_metadata();
        let metadata = Metadata::new(
            file_metadata.num_rows(),
            builder.metadata().num_row_groups() as i64,
            file_metadata.created_by().map(String::from),
            i64::from(file_metadata.version()),
        );

        let arrow_schema = builder.schema().clone();
        let schema = Schema::from_arrow_schema(&arrow_schema);

        let builder = if let Some(ref columns) = self.options.columns {
            let parquet_schema = builder.parquet_schema();
            let indices: Vec<usize> = columns
                .iter()
                .filter_map(|name| arrow_schema.fields().iter().position(|f| f.name() == name))
                .collect();
            let mask = ProjectionMask::roots(parquet_schema, indices);
            builder.with_projection(mask)
        } else {
            builder
        };

        let reader = builder
            .with_batch_size(self.options.batch_size)
            .build()
            .map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to build reader: {e}"))
            })?;

        Ok(ParquetFile {
            reader: Some(reader),
            schema,
            metadata,
        })
    }
}

#[php_impl]
#[php(change_method_case = "none")]
impl Reader {
    pub fn __construct(options: Option<&ReaderOptions>) -> Self {
        Self {
            options: options.cloned().unwrap_or_else(ReaderOptions::create),
        }
    }

    pub fn open(&self, path: String) -> PhpResult<ParquetFile> {
        let file = File::open(&path).map_err(|e| {
            PhpException::from_class::<ParquetException>(format!("Failed to open file: {e}"))
        })?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| {
            PhpException::from_class::<ParquetException>(format!("Invalid Parquet file: {e}"))
        })?;

        self.build_parquet_file(builder)
    }

    #[php(name = "openString")]
    pub fn open_string(&self, data: Binary<u8>) -> PhpResult<ParquetFile> {
        let bytes = Bytes::from(Vec::from(data));

        let builder = ParquetRecordBatchReaderBuilder::try_new(bytes).map_err(|e| {
            PhpException::from_class::<ParquetException>(format!("Invalid Parquet data: {e}"))
        })?;

        self.build_parquet_file(builder)
    }
}

#[php_class]
#[php(name = "Parquet\\ParquetFile")]
pub struct ParquetFile {
    reader: Option<ParquetRecordBatchReader>,
    schema: Schema,
    metadata: Metadata,
}

#[php_impl]
#[php(change_method_case = "none")]
impl ParquetFile {
    pub fn schema(&self) -> Schema {
        self.schema.clone()
    }

    pub fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }

    #[php(name = "readBatch")]
    pub fn read_batch(&mut self) -> PhpResult<Option<BatchRows>> {
        let Some(reader) = &mut self.reader else {
            return Ok(None);
        };

        match reader.next() {
            Some(Ok(batch)) => {
                let rows = record_batch_to_rows(&batch)?;
                Ok(Some(rows))
            }
            Some(Err(e)) => Err(PhpException::from_class::<ParquetException>(format!(
                "Failed to read batch: {e}"
            ))),
            None => {
                self.reader = None;
                Ok(None)
            }
        }
    }
}
