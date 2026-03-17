use ext_php_rs::prelude::*;

#[php_class]
#[php(name = "Parquet\\Metadata")]
#[derive(Debug, Clone)]
pub struct Metadata {
    row_count: i64,
    row_group_count: i64,
    created_by: Option<String>,
    version: i64,
}

impl Metadata {
    pub(crate) fn new(
        row_count: i64,
        row_group_count: i64,
        created_by: Option<String>,
        version: i64,
    ) -> Self {
        Self {
            row_count,
            row_group_count,
            created_by,
            version,
        }
    }
}

#[php_impl]
#[php(change_method_case = "none")]
impl Metadata {
    #[php(name = "rowCount")]
    pub fn row_count(&self) -> i64 {
        self.row_count
    }

    #[php(name = "rowGroupCount")]
    pub fn row_group_count(&self) -> i64 {
        self.row_group_count
    }

    #[php(name = "createdBy")]
    pub fn created_by(&self) -> Option<String> {
        self.created_by.clone()
    }

    pub fn version(&self) -> i64 {
        self.version
    }
}
