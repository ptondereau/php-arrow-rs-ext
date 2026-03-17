use ext_php_rs::prelude::*;
use ext_php_rs::zend::ce;

#[php_class]
#[php(name = "Parquet\\ParquetException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct ParquetException;
