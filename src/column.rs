use std::borrow::Cow;
use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Fields, TimeUnit};
use ext_php_rs::convert::FromZval;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendClassObject, Zval};

use crate::exception::ParquetException;

#[derive(Debug, Clone)]
pub enum ColumnType {
    Boolean,
    Int32,
    Int64,
    Float,
    Double,
    String,
    Date,
    DateTime,
    Time,
    Decimal { precision: u8, scale: i8 },
    Json,
    Enum,
    Uuid,
    Binary,
    List { element: Box<Column> },
    Struct { children: Vec<Column> },
    Map { key: Box<Column>, value: Box<Column> },
}

impl ColumnType {
    pub fn to_arrow_data_type(&self) -> DataType {
        match self {
            Self::Boolean => DataType::Boolean,
            Self::Int32 => DataType::Int32,
            Self::Int64 => DataType::Int64,
            Self::Float => DataType::Float32,
            Self::Double => DataType::Float64,
            Self::String | Self::Json | Self::Enum => DataType::Utf8,
            Self::Date => DataType::Date32,
            Self::DateTime => DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC"))),
            Self::Time => DataType::Time64(TimeUnit::Microsecond),
            Self::Decimal { precision, scale } => DataType::Decimal128(*precision, *scale),
            Self::Uuid => DataType::FixedSizeBinary(16),
            Self::Binary => DataType::Binary,
            Self::List { element } => {
                DataType::List(Arc::new(element.to_arrow_field()))
            }
            Self::Struct { children } => {
                let fields: Vec<Field> = children.iter().map(Column::to_arrow_field).collect();
                DataType::Struct(Fields::from(fields))
            }
            Self::Map { key, value } => {
                let key_arrow_field = Field::new(
                    "key",
                    key.col_type.to_arrow_data_type(),
                    false,
                );
                let value_arrow_field = value.to_arrow_field_with_name("value");
                let entries_field = Field::new(
                    "entries",
                    DataType::Struct(Fields::from(vec![key_arrow_field, value_arrow_field])),
                    false,
                );
                DataType::Map(Arc::new(entries_field), false)
            }
        }
    }

    pub fn display_name(&self) -> Cow<'static, str> {
        match self {
            Self::Boolean => Cow::Borrowed("boolean"),
            Self::Int32 => Cow::Borrowed("int32"),
            Self::Int64 => Cow::Borrowed("int64"),
            Self::Float => Cow::Borrowed("float"),
            Self::Double => Cow::Borrowed("double"),
            Self::String => Cow::Borrowed("string"),
            Self::Date => Cow::Borrowed("date"),
            Self::DateTime => Cow::Borrowed("dateTime"),
            Self::Time => Cow::Borrowed("time"),
            Self::Decimal { precision, scale } => Cow::Owned(format!("decimal({precision},{scale})")),
            Self::Json => Cow::Borrowed("json"),
            Self::Enum => Cow::Borrowed("enum"),
            Self::Uuid => Cow::Borrowed("uuid"),
            Self::Binary => Cow::Borrowed("binary"),
            Self::List { element } => {
                Cow::Owned(format!("list<{}>", element.col_type.display_name()))
            }
            Self::Struct { children } => {
                let inner: Vec<std::string::String> = children
                    .iter()
                    .map(|c| format!("{}:{}", c.name, c.col_type.display_name()))
                    .collect();
                Cow::Owned(format!("struct<{}>", inner.join(",")))
            }
            Self::Map { key, value } => {
                Cow::Owned(format!(
                    "map<{},{}>",
                    key.col_type.display_name(),
                    value.col_type.display_name()
                ))
            }
        }
    }
}

fn extract_columns(zvals: &[&Zval]) -> PhpResult<Vec<Column>> {
    let mut columns = Vec::with_capacity(zvals.len());
    for (i, zval) in zvals.iter().enumerate() {
        let class_obj = <&ZendClassObject<Column>>::from_zval(zval).ok_or_else(|| {
            PhpException::from_class::<ParquetException>(format!(
                "Argument {i} is not a Column instance"
            ))
        })?;
        columns.push((*class_obj).clone());
    }
    Ok(columns)
}

#[php_class]
#[php(name = "Parquet\\Column")]
#[derive(Debug, Clone)]
pub struct Column {
    name: std::string::String,
    col_type: ColumnType,
    required: bool,
}

impl Column {
    fn new(name: std::string::String, col_type: ColumnType) -> Self {
        Self {
            name,
            col_type,
            required: false,
        }
    }

    pub(crate) fn to_arrow_field(&self) -> Field {
        Field::new(
            &self.name,
            self.col_type.to_arrow_data_type(),
            !self.required,
        )
    }

    pub(crate) fn to_arrow_field_with_name(&self, name: &str) -> Field {
        Field::new(
            name,
            self.col_type.to_arrow_data_type(),
            !self.required,
        )
    }

    pub(crate) fn from_arrow_field(field: &Field) -> Self {
        let col_type = match field.data_type() {
            DataType::Boolean => ColumnType::Boolean,
            DataType::Int32 => ColumnType::Int32,
            DataType::Int64 => ColumnType::Int64,
            DataType::Float32 => ColumnType::Float,
            DataType::Float64 => ColumnType::Double,
            DataType::Utf8 => ColumnType::String,
            DataType::Date32 => ColumnType::Date,
            DataType::Timestamp(_, _) => ColumnType::DateTime,
            DataType::Time64(_) => ColumnType::Time,
            DataType::Decimal128(p, s) => ColumnType::Decimal {
                precision: *p,
                scale: *s,
            },
            DataType::FixedSizeBinary(16) => ColumnType::Uuid,
            DataType::List(element_field) => ColumnType::List {
                element: Box::new(Self::from_arrow_field(element_field)),
            },
            DataType::Struct(fields) => ColumnType::Struct {
                children: fields.iter().map(|f| Self::from_arrow_field(f)).collect(),
            },
            DataType::Map(entries_field, _) => {
                if let DataType::Struct(fields) = entries_field.data_type() {
                    ColumnType::Map {
                        key: Box::new(Self::from_arrow_field(&fields[0])),
                        value: Box::new(Self::from_arrow_field(&fields[1])),
                    }
                } else {
                    ColumnType::Binary
                }
            }
            _ => ColumnType::Binary,
        };
        Self {
            name: field.name().clone(),
            col_type,
            required: !field.is_nullable(),
        }
    }
}

#[php_impl]
#[php(change_method_case = "none")]
impl Column {
    #[php(name = "boolean")]
    pub fn php_boolean(name: String) -> Self {
        Self::new(name, ColumnType::Boolean)
    }

    pub fn int32(name: String) -> Self {
        Self::new(name, ColumnType::Int32)
    }

    pub fn int64(name: String) -> Self {
        Self::new(name, ColumnType::Int64)
    }

    #[php(name = "float")]
    pub fn php_float(name: String) -> Self {
        Self::new(name, ColumnType::Float)
    }

    pub fn double(name: String) -> Self {
        Self::new(name, ColumnType::Double)
    }

    #[php(name = "string")]
    pub fn php_string(name: String) -> Self {
        Self::new(name, ColumnType::String)
    }

    pub fn date(name: String) -> Self {
        Self::new(name, ColumnType::Date)
    }

    #[php(name = "dateTime")]
    pub fn date_time(name: String) -> Self {
        Self::new(name, ColumnType::DateTime)
    }

    pub fn time(name: String) -> Self {
        Self::new(name, ColumnType::Time)
    }

    pub fn decimal(name: String, precision: i64, scale: i64) -> PhpResult<Self> {
        let precision = u8::try_from(precision).map_err(|_| {
            PhpException::from_class::<ParquetException>(format!(
                "Invalid precision: {precision}, must be 1-38"
            ))
        })?;
        let scale = i8::try_from(scale).map_err(|_| {
            PhpException::from_class::<ParquetException>(format!(
                "Invalid scale: {scale}, must be 0-38"
            ))
        })?;
        Ok(Self::new(name, ColumnType::Decimal { precision, scale }))
    }

    pub fn json(name: String) -> Self {
        Self::new(name, ColumnType::Json)
    }

    #[php(name = "enum")]
    pub fn php_enum(name: String) -> Self {
        Self::new(name, ColumnType::Enum)
    }

    pub fn uuid(name: String) -> Self {
        Self::new(name, ColumnType::Uuid)
    }

    pub fn binary(name: String) -> Self {
        Self::new(name, ColumnType::Binary)
    }

    pub fn list(name: String, element: &Column) -> Self {
        Self::new(
            name,
            ColumnType::List {
                element: Box::new(element.clone()),
            },
        )
    }

    #[php(name = "struct")]
    pub fn php_struct(name: String, children: Vec<&Zval>) -> PhpResult<Self> {
        let columns = extract_columns(&children)?;
        Ok(Self::new(name, ColumnType::Struct { children: columns }))
    }

    pub fn map(name: String, key: &Column, value: &Column) -> Self {
        Self::new(
            name,
            ColumnType::Map {
                key: Box::new(key.clone()),
                value: Box::new(value.clone()),
            },
        )
    }

    pub fn required(&self) -> Self {
        Self {
            name: self.name.clone(),
            col_type: self.col_type.clone(),
            required: true,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[php(name = "type")]
    pub fn php_type(&self) -> String {
        self.col_type.display_name().into_owned()
    }

    #[php(name = "isRequired")]
    pub fn is_required(&self) -> bool {
        self.required
    }
}
