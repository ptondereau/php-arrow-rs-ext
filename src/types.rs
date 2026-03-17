use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::array::{
    Array, ArrayBuilder, BinaryArray, BinaryBuilder, BooleanArray, BooleanBuilder, Date32Array,
    Date32Builder, Decimal128Array, Decimal128Builder, FixedSizeBinaryArray,
    FixedSizeBinaryBuilder, Float32Array, Float32Builder, Float64Array, Float64Builder, Int32Array,
    Int32Builder, Int64Array, Int64Builder, ListArray, ListBuilder, MapArray, StringArray,
    StringBuilder, StructArray, StructBuilder, Time64MicrosecondArray, Time64MicrosecondBuilder,
    TimestampMicrosecondArray, TimestampMicrosecondBuilder, make_builder,
};
use arrow::datatypes::{DataType, Fields, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendHashTable, Zval};

use crate::exception::ParquetException;

pub(crate) fn rows_to_record_batch(rows: &[&Zval], schema: &Arc<Schema>) -> PhpResult<RecordBatch> {
    let num_rows = rows.len();
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());

    for field in schema.fields() {
        let array = build_arrow_array(
            rows,
            field.name(),
            field.data_type(),
            field.is_nullable(),
            num_rows,
        )?;
        columns.push(array);
    }

    RecordBatch::try_new(Arc::clone(schema), columns).map_err(|e| {
        PhpException::from_class::<ParquetException>(format!("Failed to create RecordBatch: {e}"))
    })
}

pub(crate) fn record_batch_to_rows(batch: &RecordBatch) -> PhpResult<Vec<Vec<(String, Zval)>>> {
    let schema = batch.schema();
    let num_rows = batch.num_rows();
    let mut rows: Vec<Vec<(String, Zval)>> = Vec::with_capacity(num_rows);

    for row_idx in 0..num_rows {
        let mut row: Vec<(String, Zval)> = Vec::with_capacity(schema.fields().len());
        for (col_idx, field) in schema.fields().iter().enumerate() {
            let col = batch.column(col_idx);
            let mut zval = Zval::new();
            if col.is_null(row_idx) {
                zval.set_null();
            } else {
                set_zval_from_array(&mut zval, col.as_ref(), row_idx, field)?;
            }
            row.push((field.name().clone(), zval));
        }
        rows.push(row);
    }

    Ok(rows)
}

#[allow(clippy::too_many_lines)]
fn build_arrow_array(
    rows: &[&Zval],
    col_name: &str,
    data_type: &DataType,
    nullable: bool,
    num_rows: usize,
) -> PhpResult<ArrayRef> {
    match data_type {
        DataType::Boolean => {
            let mut builder = BooleanBuilder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.bool()
                        .ok_or_else(|| type_error(col_name, "boolean", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int32 => {
            let mut builder = Int32Builder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.long()
                        .map(|v| i32::try_from(v).map_err(|_| type_error(col_name, "int32", i)))
                        .ok_or_else(|| type_error(col_name, "int32", i))?
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int64 => {
            let mut builder = Int64Builder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.long().ok_or_else(|| type_error(col_name, "int64", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Float32 => {
            let mut builder = Float32Builder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    #[allow(clippy::cast_possible_truncation)]
                    zval.double()
                        .map(|v| v as f32)
                        .ok_or_else(|| type_error(col_name, "float", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Float64 => {
            let mut builder = Float64Builder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.double()
                        .ok_or_else(|| type_error(col_name, "double", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Utf8 => {
            let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 32);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.str()
                        .map(ToOwned::to_owned)
                        .ok_or_else(|| type_error(col_name, "string", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Date32 => {
            let mut builder = Date32Builder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    let s = zval
                        .str()
                        .ok_or_else(|| type_error(col_name, "date (Y-m-d string)", i))?;
                    parse_date_to_days(s).ok_or_else(|| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Cannot convert value for column '{col_name}': invalid date format '{s}', expected Y-m-d"
                        ))
                    })
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Timestamp(TimeUnit::Microsecond, _) => {
            let mut builder = TimestampMicrosecondBuilder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    let s = zval
                        .str()
                        .ok_or_else(|| type_error(col_name, "dateTime (ISO 8601 string)", i))?;
                    parse_datetime_to_micros(s).ok_or_else(|| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Cannot convert value for column '{col_name}': invalid datetime format '{s}', expected ISO 8601"
                        ))
                    })
                })?;
            }
            let array = builder.finish().with_timezone("UTC");
            Ok(Arc::new(array))
        }
        DataType::Time64(TimeUnit::Microsecond) => {
            let mut builder = Time64MicrosecondBuilder::with_capacity(num_rows);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    let s = zval
                        .str()
                        .ok_or_else(|| type_error(col_name, "time (H:i:s string)", i))?;
                    parse_time_to_micros(s).ok_or_else(|| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Cannot convert value for column '{col_name}': invalid time format '{s}', expected H:i:s.u"
                        ))
                    })
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Decimal128(precision, scale) => {
            let mut builder = Decimal128Builder::with_capacity(num_rows)
                .with_precision_and_scale(*precision, *scale)
                .map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Invalid decimal config: {e}"
                    ))
                })?;
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    let s = zval
                        .str()
                        .ok_or_else(|| type_error(col_name, "decimal (numeric string)", i))?;
                    parse_decimal_to_i128(s, *scale).ok_or_else(|| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Cannot convert value for column '{col_name}': invalid decimal '{s}'"
                        ))
                    })
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::FixedSizeBinary(16) => {
            let mut builder = FixedSizeBinaryBuilder::with_capacity(num_rows, 16);
            for (i, row) in rows.iter().enumerate() {
                append_value_binary(&mut builder, row, col_name, nullable, i, |zval| {
                    let s = zval
                        .str()
                        .ok_or_else(|| type_error(col_name, "uuid (string)", i))?;
                    uuid::Uuid::parse_str(s)
                        .map(|u| u.into_bytes().to_vec())
                        .map_err(|_| {
                            PhpException::from_class::<ParquetException>(format!(
                                "Cannot convert value for column '{col_name}': invalid UUID '{s}'"
                            ))
                        })
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Binary => {
            let mut builder = BinaryBuilder::with_capacity(num_rows, num_rows * 64);
            for (i, row) in rows.iter().enumerate() {
                append_value(&mut builder, row, col_name, nullable, i, |zval| {
                    zval.str()
                        .map(|s| s.as_bytes().to_vec())
                        .ok_or_else(|| type_error(col_name, "binary (string)", i))
                })?;
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::List(element_field) => {
            let value_builder = make_builder(element_field.data_type(), num_rows);
            let mut list_builder =
                ListBuilder::with_capacity(value_builder, num_rows).with_field(element_field.clone());

            for (i, row) in rows.iter().enumerate() {
                match get_field_value(row, col_name, i, nullable)? {
                    Some(val) => {
                        let php_arr = val.array().ok_or_else(|| {
                            type_error(col_name, "array (for list)", i)
                        })?;
                        for (_idx, elem) in php_arr {
                            append_to_dyn_builder(
                                list_builder.values(),
                                elem,
                                element_field.data_type(),
                                col_name,
                                i,
                            )?;
                        }
                        list_builder.append(true);
                    }
                    None => {
                        list_builder.append(false);
                    }
                }
            }
            Ok(Arc::new(list_builder.finish()))
        }
        DataType::Struct(fields) => {
            build_struct_array(rows, col_name, fields, nullable, num_rows)
        }
        DataType::Map(entries_field, _) => {
            build_map_array(rows, col_name, entries_field, nullable, num_rows)
        }
        _ => Err(PhpException::from_class::<ParquetException>(format!(
            "Unsupported data type for column '{col_name}': {data_type}"
        ))),
    }
}

macro_rules! downcast_builder {
    ($builder:expr, $ty:ty) => {
        $builder
            .as_any_mut()
            .downcast_mut::<$ty>()
            .expect(concat!("builder downcast to ", stringify!($ty), " must succeed after DataType match"))
    };
}

fn append_null_to_builder(builder: &mut dyn ArrayBuilder, data_type: &DataType) {
    match data_type {
        DataType::Boolean => downcast_builder!(builder, BooleanBuilder).append_null(),
        DataType::Int32 => downcast_builder!(builder, Int32Builder).append_null(),
        DataType::Int64 => downcast_builder!(builder, Int64Builder).append_null(),
        DataType::Float32 => downcast_builder!(builder, Float32Builder).append_null(),
        DataType::Float64 => downcast_builder!(builder, Float64Builder).append_null(),
        DataType::Utf8 => downcast_builder!(builder, StringBuilder).append_null(),
        DataType::Date32 => downcast_builder!(builder, Date32Builder).append_null(),
        DataType::Timestamp(TimeUnit::Microsecond, _) => downcast_builder!(builder, TimestampMicrosecondBuilder).append_null(),
        DataType::Time64(TimeUnit::Microsecond) => downcast_builder!(builder, Time64MicrosecondBuilder).append_null(),
        DataType::Decimal128(_, _) => downcast_builder!(builder, Decimal128Builder).append_null(),
        DataType::Binary => downcast_builder!(builder, BinaryBuilder).append_null(),
        DataType::List(_) => {
            let lb = builder.as_any_mut().downcast_mut::<ListBuilder<Box<dyn ArrayBuilder>>>()
                .expect("builder downcast to ListBuilder must succeed after DataType match");
            lb.append(false);
        }
        DataType::Struct(fields) => {
            let sb = builder.as_any_mut().downcast_mut::<StructBuilder>()
                .expect("builder downcast to StructBuilder must succeed after DataType match");
            for (fi, field) in fields.iter().enumerate() {
                let child = sb.field_builders_mut().get_mut(fi)
                    .expect("field index valid in StructBuilder");
                append_null_to_builder(child.as_mut(), field.data_type());
            }
            sb.append(false);
        }
        DataType::Map(_, _) => {
            let mb = builder.as_any_mut().downcast_mut::<arrow::array::MapBuilder<Box<dyn ArrayBuilder>, Box<dyn ArrayBuilder>>>()
                .expect("builder downcast to MapBuilder must succeed after DataType match");
            let _ = mb.append(false);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_lines)]
fn append_to_dyn_builder(
    builder: &mut dyn ArrayBuilder,
    val: &Zval,
    data_type: &DataType,
    col_name: &str,
    row_idx: usize,
) -> PhpResult<()> {
    if val.is_null() {
        append_null_to_builder(builder, data_type);
        return Ok(());
    }
    match data_type {
        DataType::Boolean => {
            let v = val.bool().ok_or_else(|| type_error(col_name, "boolean", row_idx))?;
            downcast_builder!(builder, BooleanBuilder).append_value(v);
        }
        DataType::Int32 => {
            let v = val.long().ok_or_else(|| type_error(col_name, "int32", row_idx))?;
            let v = i32::try_from(v).map_err(|_| type_error(col_name, "int32", row_idx))?;
            downcast_builder!(builder, Int32Builder).append_value(v);
        }
        DataType::Int64 => {
            let v = val.long().ok_or_else(|| type_error(col_name, "int64", row_idx))?;
            downcast_builder!(builder, Int64Builder).append_value(v);
        }
        DataType::Float32 => {
            #[allow(clippy::cast_possible_truncation)]
            let v = val.double().map(|v| v as f32).ok_or_else(|| type_error(col_name, "float", row_idx))?;
            downcast_builder!(builder, Float32Builder).append_value(v);
        }
        DataType::Float64 => {
            let v = val.double().ok_or_else(|| type_error(col_name, "double", row_idx))?;
            downcast_builder!(builder, Float64Builder).append_value(v);
        }
        DataType::Utf8 => {
            let v = val.str().ok_or_else(|| type_error(col_name, "string", row_idx))?;
            downcast_builder!(builder, StringBuilder).append_value(v);
        }
        DataType::Date32 => {
            let s = val.str().ok_or_else(|| type_error(col_name, "date", row_idx))?;
            let days = parse_date_to_days(s).ok_or_else(|| type_error(col_name, "date (Y-m-d)", row_idx))?;
            downcast_builder!(builder, Date32Builder).append_value(days);
        }
        DataType::Timestamp(TimeUnit::Microsecond, _) => {
            let s = val.str().ok_or_else(|| type_error(col_name, "dateTime", row_idx))?;
            let micros = parse_datetime_to_micros(s).ok_or_else(|| type_error(col_name, "dateTime (ISO 8601)", row_idx))?;
            downcast_builder!(builder, TimestampMicrosecondBuilder).append_value(micros);
        }
        DataType::Time64(TimeUnit::Microsecond) => {
            let s = val.str().ok_or_else(|| type_error(col_name, "time", row_idx))?;
            let micros = parse_time_to_micros(s).ok_or_else(|| type_error(col_name, "time (H:i:s)", row_idx))?;
            downcast_builder!(builder, Time64MicrosecondBuilder).append_value(micros);
        }
        DataType::Decimal128(_, scale) => {
            let s = val.str().ok_or_else(|| type_error(col_name, "decimal", row_idx))?;
            let v = parse_decimal_to_i128(s, *scale).ok_or_else(|| type_error(col_name, "decimal", row_idx))?;
            downcast_builder!(builder, Decimal128Builder).append_value(v);
        }
        DataType::Binary => {
            let v = val.str().map(|s| s.as_bytes().to_vec()).ok_or_else(|| type_error(col_name, "binary", row_idx))?;
            downcast_builder!(builder, BinaryBuilder).append_value(&v);
        }
        DataType::List(element_field) => {
            let lb = builder.as_any_mut().downcast_mut::<ListBuilder<Box<dyn ArrayBuilder>>>()
                .expect("builder downcast to ListBuilder must succeed after DataType match");
            let php_arr = val.array().ok_or_else(|| type_error(col_name, "array (for list)", row_idx))?;
            for (_idx, elem) in php_arr {
                append_to_dyn_builder(lb.values().as_mut(), elem, element_field.data_type(), col_name, row_idx)?;
            }
            lb.append(true);
        }
        DataType::Struct(fields) => {
            let sb = builder.as_any_mut().downcast_mut::<StructBuilder>()
                .expect("builder downcast to StructBuilder must succeed after DataType match");
            let inner_arr = val.array().ok_or_else(|| type_error(col_name, "array (for struct)", row_idx))?;
            for (fi, field) in fields.iter().enumerate() {
                let null_zval = Zval::new();
                let val_ref = inner_arr.get(field.name().as_str()).unwrap_or(&null_zval);
                let child = sb.field_builders_mut().get_mut(fi)
                    .expect("field index valid in StructBuilder");
                append_to_dyn_builder(child.as_mut(), val_ref, field.data_type(), col_name, row_idx)?;
            }
            sb.append(true);
        }
        DataType::Map(entries_field, _) => {
            let DataType::Struct(struct_fields) = entries_field.data_type() else {
                return Err(PhpException::from_class::<ParquetException>(
                    format!("Map entries for '{col_name}' is not a struct")
                ));
            };
            let key_field = &struct_fields[0];
            let value_field = &struct_fields[1];
            let mb = builder.as_any_mut().downcast_mut::<arrow::array::MapBuilder<Box<dyn ArrayBuilder>, Box<dyn ArrayBuilder>>>()
                .expect("builder downcast to MapBuilder must succeed after DataType match");
            let php_arr = val.array().ok_or_else(|| type_error(col_name, "array (for map)", row_idx))?;
            for (k, v) in php_arr {
                let key_str = match k {
                    ext_php_rs::types::ArrayKey::Long(n) => n.to_string(),
                    ext_php_rs::types::ArrayKey::String(s) => s,
                    ext_php_rs::types::ArrayKey::Str(s) => (*s).to_owned(),
                };
                let mut key_zval = Zval::new();
                match key_field.data_type() {
                    DataType::Int32 | DataType::Int64 => {
                        if let Ok(n) = key_str.parse::<i64>() { key_zval.set_long(n); }
                    }
                    _ => { let _ = key_zval.set_string(&key_str, false); }
                }
                append_to_dyn_builder(mb.keys().as_mut(), &key_zval, key_field.data_type(), col_name, row_idx)?;
                append_to_dyn_builder(mb.values().as_mut(), v, value_field.data_type(), col_name, row_idx)?;
            }
            mb.append(true).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to append map for '{col_name}': {e}"))
            })?;
        }
        _ => {
            return Err(PhpException::from_class::<ParquetException>(format!(
                "Unsupported nested element type for column '{col_name}': {data_type}"
            )));
        }
    }
    Ok(())
}

fn build_struct_array(
    rows: &[&Zval],
    col_name: &str,
    fields: &Fields,
    nullable: bool,
    num_rows: usize,
) -> PhpResult<ArrayRef> {
    let mut struct_builder = StructBuilder::from_fields(fields.clone(), num_rows);
    let child_names: Vec<String> = fields
        .iter()
        .map(|f| format!("{col_name}.{}", f.name()))
        .collect();

    for (i, row) in rows.iter().enumerate() {
        if let Some(val) = get_field_value(row, col_name, i, nullable)? {
            let inner_arr = val.array().ok_or_else(|| {
                type_error(col_name, "array (for struct)", i)
            })?;
            for (field_idx, field) in fields.iter().enumerate() {
                let null_zval = Zval::new();
                let val_ref = inner_arr
                    .get(field.name().as_str())
                    .unwrap_or(&null_zval);
                let child_builder = struct_builder
                    .field_builders_mut()
                    .get_mut(field_idx)
                    .expect("field index must be valid within StructBuilder");
                append_to_dyn_builder(
                    child_builder.as_mut(),
                    val_ref,
                    field.data_type(),
                    &child_names[field_idx],
                    i,
                )?;
            }
            struct_builder.append(true);
        } else {
            for (field_idx, field) in fields.iter().enumerate() {
                let child_builder = struct_builder
                    .field_builders_mut()
                    .get_mut(field_idx)
                    .expect("field index must be valid within StructBuilder");
                append_null_to_builder(child_builder.as_mut(), field.data_type());
            }
            struct_builder.append(false);
        }
    }
    Ok(Arc::new(struct_builder.finish()))
}

fn build_map_array(
    rows: &[&Zval],
    col_name: &str,
    entries_field: &Arc<arrow::datatypes::Field>,
    nullable: bool,
    num_rows: usize,
) -> PhpResult<ArrayRef> {
    let DataType::Struct(struct_fields) = entries_field.data_type() else {
        return Err(PhpException::from_class::<ParquetException>(format!(
            "Map entries field for column '{col_name}' is not a struct"
        )));
    };
    let key_field = &struct_fields[0];
    let value_field = &struct_fields[1];

    let key_builder = make_builder(key_field.data_type(), num_rows);
    let value_builder = make_builder(value_field.data_type(), num_rows);
    let field_names = arrow::array::builder::MapFieldNames {
        entry: entries_field.name().clone(),
        key: key_field.name().clone(),
        value: value_field.name().clone(),
    };
    let mut map_builder = arrow::array::MapBuilder::with_capacity(
        Some(field_names),
        key_builder,
        value_builder,
        num_rows,
    )
    .with_keys_field(key_field.clone())
    .with_values_field(value_field.clone());

    let key_col_name = format!("{col_name}.key");
    let value_col_name = format!("{col_name}.value");

    for (i, row) in rows.iter().enumerate() {
        match get_field_value(row, col_name, i, nullable)? {
            Some(val) => {
                let php_arr = val.array().ok_or_else(|| {
                    type_error(col_name, "array (for map)", i)
                })?;
                for (k, v) in php_arr {
                    let key_str = match k {
                        ext_php_rs::types::ArrayKey::Long(n) => n.to_string(),
                        ext_php_rs::types::ArrayKey::String(s) => s,
                        ext_php_rs::types::ArrayKey::Str(s) => (*s).to_owned(),
                    };
                    let mut key_zval = Zval::new();
                    match key_field.data_type() {
                        DataType::Int32 | DataType::Int64 => {
                            if let Ok(n) = key_str.parse::<i64>() {
                                key_zval.set_long(n);
                            }
                        }
                        _ => {
                            let _ = key_zval.set_string(&key_str, false);
                        }
                    }
                    append_to_dyn_builder(
                        map_builder.keys(),
                        &key_zval,
                        key_field.data_type(),
                        &key_col_name,
                        i,
                    )?;
                    append_to_dyn_builder(
                        map_builder.values(),
                        v,
                        value_field.data_type(),
                        &value_col_name,
                        i,
                    )?;
                }
                map_builder.append(true).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to append map entry for column '{col_name}' in row {i}: {e}"
                    ))
                })?;
            }
            None => {
                map_builder.append(false).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to append null map for column '{col_name}' in row {i}: {e}"
                    ))
                })?;
            }
        }
    }
    Ok(Arc::new(map_builder.finish()))
}

fn get_field_value<'a>(
    row: &'a Zval,
    col_name: &str,
    row_index: usize,
    nullable: bool,
) -> PhpResult<Option<&'a Zval>> {
    let arr = row.array().ok_or_else(|| {
        PhpException::from_class::<ParquetException>(format!("Row {row_index} is not an array"))
    })?;

    match arr.get(col_name) {
        Some(val) if !val.is_null() => Ok(Some(val)),
        _ if nullable => Ok(None),
        _ => Err(PhpException::from_class::<ParquetException>(format!(
            "Missing required column '{col_name}' in row {row_index}"
        ))),
    }
}

fn append_value<T, B, F>(
    builder: &mut B,
    row: &Zval,
    col_name: &str,
    nullable: bool,
    row_index: usize,
    convert: F,
) -> PhpResult<()>
where
    B: ArrayBuilder + AppendValue<T>,
    F: FnOnce(&Zval) -> PhpResult<T>,
{
    if let Some(val) = get_field_value(row, col_name, row_index, nullable)? {
        let v = convert(val)?;
        builder.append_val(v);
    } else {
        builder.append_null_val();
    }
    Ok(())
}

fn append_value_binary<F>(
    builder: &mut FixedSizeBinaryBuilder,
    row: &Zval,
    col_name: &str,
    nullable: bool,
    row_index: usize,
    convert: F,
) -> PhpResult<()>
where
    F: FnOnce(&Zval) -> PhpResult<Vec<u8>>,
{
    if let Some(val) = get_field_value(row, col_name, row_index, nullable)? {
        let bytes = convert(val)?;
        builder.append_value(&bytes).map_err(|e| {
            PhpException::from_class::<ParquetException>(format!("Failed to write UUID: {e}"))
        })?;
    } else {
        builder.append_null();
    }
    Ok(())
}

trait AppendValue<T> {
    fn append_val(&mut self, value: T);
    fn append_null_val(&mut self);
}

impl AppendValue<bool> for BooleanBuilder {
    fn append_val(&mut self, value: bool) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i32> for Int32Builder {
    fn append_val(&mut self, value: i32) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i64> for Int64Builder {
    fn append_val(&mut self, value: i64) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<f32> for Float32Builder {
    fn append_val(&mut self, value: f32) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<f64> for Float64Builder {
    fn append_val(&mut self, value: f64) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<String> for StringBuilder {
    fn append_val(&mut self, value: String) {
        self.append_value(&value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i32> for Date32Builder {
    fn append_val(&mut self, value: i32) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i64> for TimestampMicrosecondBuilder {
    fn append_val(&mut self, value: i64) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i64> for Time64MicrosecondBuilder {
    fn append_val(&mut self, value: i64) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<i128> for Decimal128Builder {
    fn append_val(&mut self, value: i128) {
        self.append_value(value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

impl AppendValue<Vec<u8>> for BinaryBuilder {
    fn append_val(&mut self, value: Vec<u8>) {
        self.append_value(&value);
    }
    fn append_null_val(&mut self) {
        self.append_null();
    }
}

fn type_error(col_name: &str, expected: &str, row_index: usize) -> PhpException {
    PhpException::from_class::<ParquetException>(format!(
        "Column '{col_name}' expected {expected} in row {row_index}"
    ))
}

fn parse_date_to_days(s: &str) -> Option<i32> {
    let mut parts = s.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    let epoch = UNIX_EPOCH_DATE?;
    let date = chrono::NaiveDate::from_ymd_opt(y, m, d)?;
    #[allow(clippy::cast_possible_truncation)]
    Some((date - epoch).num_days() as i32)
}

fn parse_datetime_to_micros(s: &str) -> Option<i64> {
    use chrono::{DateTime, NaiveDateTime};
    if let Ok(dt) = s.parse::<DateTime<chrono::Utc>>() {
        return Some(dt.timestamp_micros());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().timestamp_micros());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp_micros());
    }
    None
}

fn parse_time_to_micros(s: &str) -> Option<i64> {
    let mut parts = s.splitn(3, ':');
    let h: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let sec_str = parts.next().unwrap_or("0");
    let (s_part, us_part) = if let Some((sec, frac)) = sec_str.split_once('.') {
        let sec: i64 = sec.parse().ok()?;
        let frac_str = format!("{frac:0<6}");
        let us: i64 = frac_str[..6].parse().ok()?;
        (sec, us)
    } else {
        (sec_str.parse::<i64>().ok()?, 0)
    };
    Some(h * 3_600_000_000 + m * 60_000_000 + s_part * 1_000_000 + us_part)
}

fn parse_decimal_to_i128(s: &str, scale: i8) -> Option<i128> {
    let s = s.trim();
    let scale_usize = usize::try_from(scale).ok()?;
    let scale_u32 = u32::try_from(scale).ok()?;
    if let Some((int_part, frac_part)) = s.split_once('.') {
        let int_val: i128 = int_part.parse().ok()?;
        let frac_str = format!("{frac_part:0<scale_usize$}");
        let frac_val: i128 = frac_str[..scale_usize].parse().ok()?;
        let multiplier: i128 = 10_i128.pow(scale_u32);
        if int_val < 0 {
            Some(int_val * multiplier - frac_val)
        } else {
            Some(int_val * multiplier + frac_val)
        }
    } else {
        let int_val: i128 = s.parse().ok()?;
        let multiplier: i128 = 10_i128.pow(scale_u32);
        Some(int_val * multiplier)
    }
}

macro_rules! downcast_array {
    ($array:expr, $ty:ty) => {
        $array
            .as_any()
            .downcast_ref::<$ty>()
            .expect(concat!("array downcast to ", stringify!($ty), " must succeed after DataType match"))
    };
}

const UNIX_EPOCH_DATE: Option<chrono::NaiveDate> = chrono::NaiveDate::from_ymd_opt(1970, 1, 1);

#[allow(clippy::too_many_lines)]
fn set_zval_from_array(
    zval: &mut Zval,
    array: &dyn Array,
    row_idx: usize,
    field: &arrow::datatypes::Field,
) -> PhpResult<()> {
    match field.data_type() {
        DataType::Boolean => {
            let arr = downcast_array!(array, BooleanArray);
            zval.set_bool(arr.value(row_idx));
        }
        DataType::Int32 => {
            let arr = downcast_array!(array, Int32Array);
            zval.set_long(i64::from(arr.value(row_idx)));
        }
        DataType::Int64 => {
            let arr = downcast_array!(array, Int64Array);
            zval.set_long(arr.value(row_idx));
        }
        DataType::Float32 => {
            let arr = downcast_array!(array, Float32Array);
            zval.set_double(f64::from(arr.value(row_idx)));
        }
        DataType::Float64 => {
            let arr = downcast_array!(array, Float64Array);
            zval.set_double(arr.value(row_idx));
        }
        DataType::Utf8 => {
            let arr = downcast_array!(array, StringArray);
            zval.set_string(arr.value(row_idx), false).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to set string: {e}"))
            })?;
        }
        DataType::Date32 => {
            let arr = downcast_array!(array, Date32Array);
            let days = arr.value(row_idx);
            let epoch = UNIX_EPOCH_DATE.expect("1970-01-01 is a valid date");
            let date = epoch + chrono::Days::new(u64::try_from(days).unwrap_or(0));
            zval.set_string(&date.format("%Y-%m-%d").to_string(), false)
                .map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!("Failed to set date: {e}"))
                })?;
        }
        DataType::Timestamp(TimeUnit::Microsecond, _) => {
            let arr = downcast_array!(array, TimestampMicrosecondArray);
            let micros = arr.value(row_idx);
            let dt = chrono::DateTime::from_timestamp_micros(micros).ok_or_else(|| {
                PhpException::from_class::<ParquetException>(format!(
                    "Invalid timestamp value: {micros}"
                ))
            })?;
            zval.set_string(&dt.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string(), false)
                .map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to set datetime: {e}"
                    ))
                })?;
        }
        DataType::Time64(TimeUnit::Microsecond) => {
            let arr = downcast_array!(array, Time64MicrosecondArray);
            let micros = arr.value(row_idx);
            let total_secs = micros / 1_000_000;
            let frac_micros = micros % 1_000_000;
            let h = total_secs / 3600;
            let m = (total_secs % 3600) / 60;
            let s = total_secs % 60;
            let time_str = format!("{h:02}:{m:02}:{s:02}.{frac_micros:06}");
            zval.set_string(&time_str, false).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to set time: {e}"))
            })?;
        }
        DataType::Decimal128(_, scale) => {
            let arr = downcast_array!(array, Decimal128Array);
            let val = arr.value(row_idx);
            let decimal_str = format_decimal(val, *scale);
            zval.set_string(&decimal_str, false).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to set decimal: {e}"))
            })?;
        }
        DataType::FixedSizeBinary(16) => {
            let arr = downcast_array!(array, FixedSizeBinaryArray);
            let bytes = arr.value(row_idx);
            let u = uuid::Uuid::from_bytes(
                bytes
                    .try_into()
                    .expect("FixedSizeBinary(16) always yields 16 bytes"),
            );
            zval.set_string(&u.to_string(), false).map_err(|e| {
                PhpException::from_class::<ParquetException>(format!("Failed to set uuid: {e}"))
            })?;
        }
        DataType::Binary => {
            let arr = downcast_array!(array, BinaryArray);
            let bytes = arr.value(row_idx);
            zval.set_string(&String::from_utf8_lossy(bytes), false)
                .map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to set binary: {e}"
                    ))
                })?;
        }
        DataType::List(element_field) => {
            let list_arr = downcast_array!(array, ListArray);
            let values = list_arr.value(row_idx);
            let mut php_array = ZendHashTable::new();
            for i in 0..values.len() {
                let mut elem_zval = Zval::new();
                if values.is_null(i) {
                    elem_zval.set_null();
                } else {
                    set_zval_from_array(&mut elem_zval, values.as_ref(), i, element_field)?;
                }
                php_array.push(elem_zval).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to push list element: {e}"
                    ))
                })?;
            }
            zval.set_hashtable(php_array);
        }
        DataType::Struct(fields) => {
            let struct_arr = downcast_array!(array, StructArray);
            let mut php_array = ZendHashTable::new();
            for (col_idx, child_field) in fields.iter().enumerate() {
                let child = struct_arr.column(col_idx);
                let mut child_zval = Zval::new();
                if child.is_null(row_idx) {
                    child_zval.set_null();
                } else {
                    set_zval_from_array(&mut child_zval, child.as_ref(), row_idx, child_field)?;
                }
                php_array.insert(child_field.name().as_str(), child_zval).map_err(|e| {
                    PhpException::from_class::<ParquetException>(format!(
                        "Failed to insert struct field '{}': {e}",
                        child_field.name()
                    ))
                })?;
            }
            zval.set_hashtable(php_array);
        }
        DataType::Map(entries_field, _) => {
            let map_arr = downcast_array!(array, MapArray);
            let entry_struct = map_arr.value(row_idx);
            let key_arr = entry_struct.column(0);
            let val_arr = entry_struct.column(1);

            let DataType::Struct(struct_fields) = entries_field.data_type() else {
                zval.set_null();
                return Ok(());
            };
            let key_field = &struct_fields[0];
            let value_field = &struct_fields[1];

            let mut php_array = ZendHashTable::new();
            for i in 0..entry_struct.len() {
                let mut key_zval = Zval::new();
                set_zval_from_array(&mut key_zval, key_arr.as_ref(), i, key_field)?;

                let mut val_zval = Zval::new();
                if val_arr.is_null(i) {
                    val_zval.set_null();
                } else {
                    set_zval_from_array(&mut val_zval, val_arr.as_ref(), i, value_field)?;
                }

                if let Some(key_str) = key_zval.str() {
                    let key_owned = key_str.to_owned();
                    php_array.insert(key_owned.as_str(), val_zval).map_err(|e| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Failed to insert map entry: {e}"
                        ))
                    })?;
                } else if let Some(key_long) = key_zval.long() {
                    php_array.insert_at_index(key_long, val_zval).map_err(|e| {
                        PhpException::from_class::<ParquetException>(format!(
                            "Failed to insert map entry: {e}"
                        ))
                    })?;
                }
            }
            zval.set_hashtable(php_array);
        }
        _ => {
            zval.set_null();
        }
    }
    Ok(())
}

fn format_decimal(val: i128, scale: i8) -> String {
    if scale <= 0 {
        return val.to_string();
    }
    #[allow(clippy::cast_sign_loss)]
    let scale_u32 = scale as u32;
    let divisor = 10_i128.pow(scale_u32);
    let int_part = val / divisor;
    let frac_part = (val % divisor).abs();
    #[allow(clippy::cast_sign_loss)]
    let width = scale as usize;
    format!("{int_part}.{frac_part:0>width$}")
}
