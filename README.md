# parquet-php

[![CI](https://github.com/ptondereau/php-arrow-rs-ext/actions/workflows/tests.yml/badge.svg)](https://github.com/ptondereau/php-arrow-rs-ext/actions/workflows/tests.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A PHP extension for reading and writing Apache Parquet files, powered by Rust and [arrow-rs](https://github.com/apache/arrow-rs).

Supports all primitive types, nested types (lists, structs, maps), 6 compression codecs, column projection, and in-memory I/O. Requires PHP 8.2+.

## Installation

### PIE

```bash
pie install ptondereau/parquet-php
```

PIE downloads a prebuilt binary for your platform. If none is available, it builds from source (requires the Rust toolchain).

### Prebuilt binaries

Prebuilt binaries are published on [GitHub Releases](https://github.com/ptondereau/php-arrow-rs-ext/releases) for PHP 8.2–8.5 (NTS and ZTS):

| OS    | Arch  | Libc        |
|-------|-------|-------------|
| Linux | x64   | glibc, musl |
| Linux | arm64 | glibc, musl |
| macOS | x64   | —           |
| macOS | arm64 | —           |

### Build from source

```bash
# Requires Rust toolchain (https://rustup.rs)
cargo build --release
cp target/release/libparquet.so "$(php-config --extension-dir)/parquet.so"
```

### Loading

```ini
; php.ini
extension=parquet.so
```

## Usage

### Defining a schema

```php
use Parquet\Column;
use Parquet\Schema;

$schema = Schema::create([
    Column::int64('id')->required(),
    Column::string('name'),
    Column::double('price'),
    Column::boolean('active'),
    Column::date('created_at'),
    Column::decimal('amount', 10, 2),
]);
```

### Writing a file

```php
use Parquet\Writer;
use Parquet\WriterOptions;
use Parquet\Compression;

$writer = new Writer(
    WriterOptions::create()->withCompression(Compression::Snappy)
);

$writer->write('data.parquet', $schema, [
    ['id' => 1, 'name' => 'Alice', 'price' => 9.99, 'active' => true, 'created_at' => '2024-01-15', 'amount' => '123.45'],
    ['id' => 2, 'name' => 'Bob',   'price' => 4.50, 'active' => false, 'created_at' => '2024-02-20', 'amount' => '67.89'],
]);
```

### Incremental writing

```php
$writer = new Writer();
$writer->open('large.parquet', $schema);

foreach (array_chunk($rows, 10_000) as $batch) {
    $writer->writeBatch($batch);
}

$writer->close();
```

### Reading a file

```php
use Parquet\Reader;
use Parquet\ReaderOptions;

$reader = new Reader(
    ReaderOptions::create()
        ->withBatchSize(5000)
        ->withColumns(['id', 'name'])
);

$file = $reader->open('data.parquet');

echo $file->metadata()->rowCount(); // total rows

while ($batch = $file->readBatch()) {
    foreach ($batch as $row) {
        echo $row['name'];
    }
}
```

### Nested types

```php
$schema = Schema::create([
    Column::string('name')->required(),
    Column::list('tags', Column::string('element')),
    Column::struct('address', [
        Column::string('street'),
        Column::string('city'),
        Column::string('zip'),
    ]),
    Column::map('metadata', Column::string('key'), Column::int32('value')),
]);

$writer = new Writer();
$writer->write('nested.parquet', $schema, [
    [
        'name' => 'Alice',
        'tags' => ['admin', 'active'],
        'address' => ['street' => '123 Main St', 'city' => 'Paris', 'zip' => '75001'],
        'metadata' => ['login_count' => 42, 'score' => 100],
    ],
]);
```

### In-memory I/O

```php
$writer = new Writer();
$buffer = $writer->writeToString($schema, $rows);

$reader = new Reader();
$file = $reader->openString($buffer);

while ($batch = $file->readBatch()) {
    // ...
}
```

### Compression

Six codecs are supported: `Uncompressed`, `Snappy`, `Gzip`, `Brotli`, `Lz4Raw`, `Zstd`.

```php
use Parquet\Compression;
use Parquet\Encoding;

$options = WriterOptions::create()
    ->withCompression(Compression::Zstd)
    ->withColumnCompression('payload', Compression::Snappy)
    ->withColumnEncoding('id', Encoding::DeltaBinaryPacked);
```

## Supported types

| Type | Column factory | PHP representation |
|------|---------------|--------------------|
| Boolean | `Column::boolean()` | `bool` |
| Int32 | `Column::int32()` | `int` |
| Int64 | `Column::int64()` | `int` |
| Float | `Column::float()` | `float` |
| Double | `Column::double()` | `float` |
| String | `Column::string()` | `string` |
| Date | `Column::date()` | `string` (Y-m-d) |
| DateTime | `Column::dateTime()` | `string` (ISO 8601) |
| Time | `Column::time()` | `string` (H:i:s.u) |
| Decimal | `Column::decimal()` | `string` |
| JSON | `Column::json()` | `string` |
| Enum | `Column::enum()` | `string` |
| UUID | `Column::uuid()` | `string` |
| Binary | `Column::binary()` | `string` |
| List | `Column::list()` | `array` |
| Struct | `Column::struct()` | `array` (assoc) |
| Map | `Column::map()` | `array` (assoc) |

All columns are nullable by default. Call `->required()` to enforce non-null.

For the full API, see [`parquet.stubs.php`](parquet.stubs.php).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
