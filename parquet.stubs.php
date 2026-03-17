<?php

namespace Parquet;

/**
 * @phpstan-type CompressionValue int
 */
enum Compression: int
{
    case Uncompressed = 0;
    case Snappy = 1;
    case Gzip = 2;
    case Brotli = 3;
    case Lz4Raw = 4;
    case Zstd = 5;
}

class ParquetException extends \Exception {}

class Column
{
    private function __construct() {}

    public static function boolean(string $name): self {}

    public static function int32(string $name): self {}

    public static function int64(string $name): self {}

    public static function float(string $name): self {}

    public static function double(string $name): self {}

    public static function string(string $name): self {}

    public static function date(string $name): self {}

    public static function dateTime(string $name): self {}

    public static function time(string $name): self {}

    /**
     * @param int $precision Precision (1-38)
     * @param int $scale Scale (0-38)
     * @throws ParquetException
     */
    public static function decimal(string $name, int $precision, int $scale): self {}

    public static function json(string $name): self {}

    public static function enum(string $name): self {}

    public static function uuid(string $name): self {}

    public static function binary(string $name): self {}

    public function required(): self {}

    public function name(): string {}

    public function type(): string {}

    public function isRequired(): bool {}
}

class Schema
{
    private function __construct() {}

    /**
     * @param list<Column> $columns
     * @throws ParquetException When columns array is empty or contains non-Column values
     */
    public static function create(array $columns): self {}

    /**
     * @return list<Column>
     */
    public function columns(): array {}

    /**
     * @throws ParquetException When column is not found
     */
    public function column(string $name): Column {}

    public function has(string $name): bool {}
}

class WriterOptions
{
    private function __construct() {}

    public static function create(): self {}

    public function withCompression(Compression $compression): self {}

    /**
     * @throws ParquetException When bytes is not positive
     */
    public function withRowGroupSize(int $bytes): self {}

    /**
     * @throws ParquetException When bytes is not positive
     */
    public function withPageSize(int $bytes): self {}
}

class Writer
{
    public function __construct(?WriterOptions $options = null) {}

    /**
     * @param list<array<string, mixed>> $rows
     * @throws ParquetException
     */
    public function write(string $path, Schema $schema, array $rows): void {}

    /**
     * @throws ParquetException
     */
    public function open(string $path, Schema $schema): void {}

    /**
     * @param list<array<string, mixed>> $rows
     * @throws ParquetException When writer is not open
     */
    public function writeBatch(array $rows): void {}

    /**
     * @throws ParquetException When writer is not open or already closed
     */
    public function close(): void {}
}

class ReaderOptions
{
    private function __construct() {}

    public static function create(): self {}

    /**
     * @throws ParquetException When size is not positive
     */
    public function withBatchSize(int $size): self {}

    /**
     * @param list<string> $columns
     */
    public function withColumns(array $columns): self {}
}

class Reader
{
    public function __construct(?ReaderOptions $options = null) {}

    /**
     * @throws ParquetException
     */
    public function open(string $path): ParquetFile {}
}

class ParquetFile
{
    private function __construct() {}

    public function schema(): Schema {}

    public function metadata(): Metadata {}

    /**
     * @return list<array<string, mixed>>|null
     * @throws ParquetException
     */
    public function readBatch(): ?array {}
}

class Metadata
{
    private function __construct() {}

    public function rowCount(): int {}

    public function rowGroupCount(): int {}

    public function createdBy(): ?string {}

    public function version(): int {}
}
