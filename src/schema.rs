use ext_php_rs::convert::FromZval;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendClassObject, Zval};

use crate::column::Column;
use crate::exception::ParquetException;

#[php_class]
#[php(name = "Parquet\\Schema")]
#[derive(Debug, Clone)]
pub struct Schema {
    columns: Vec<Column>,
}

impl Schema {
    pub(crate) fn to_arrow_schema(&self) -> arrow::datatypes::Schema {
        let fields: Vec<arrow::datatypes::Field> =
            self.columns.iter().map(Column::to_arrow_field).collect();
        arrow::datatypes::Schema::new(fields)
    }

    pub(crate) fn from_arrow_schema(arrow_schema: &arrow::datatypes::Schema) -> Self {
        let columns = arrow_schema
            .fields()
            .iter()
            .map(|f| Column::from_arrow_field(f))
            .collect();
        Self { columns }
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

#[php_impl]
#[php(change_method_case = "none")]
impl Schema {
    pub fn create(columns: Vec<&Zval>) -> PhpResult<Self> {
        if columns.is_empty() {
            return Err(PhpException::from_class::<ParquetException>(
                "Schema must have at least one column".to_owned(),
            ));
        }
        let columns = extract_columns(&columns)?;
        Ok(Self { columns })
    }

    pub fn columns(&self) -> Vec<Column> {
        self.columns.clone()
    }

    pub fn column(&self, name: String) -> PhpResult<Column> {
        self.columns
            .iter()
            .find(|c| c.name() == name)
            .cloned()
            .ok_or_else(|| {
                PhpException::from_class::<ParquetException>(format!(
                    "Column '{name}' not found in schema"
                ))
            })
    }

    pub fn has(&self, name: String) -> bool {
        self.columns.iter().any(|c| c.name() == name)
    }
}
