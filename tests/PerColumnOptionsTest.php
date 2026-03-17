<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\Compression;
use Parquet\Encoding;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use Parquet\WriterOptions;
use PHPUnit\Framework\TestCase;

final class PerColumnOptionsTest extends TestCase
{
    private string $tempDir;

    protected function setUp(): void
    {
        $this->tempDir = sys_get_temp_dir() . '/parquet_colopt_' . uniqid();
        mkdir($this->tempDir, 0o777, true);
    }

    protected function tearDown(): void
    {
        $files = glob($this->tempDir . '/*.parquet');
        if ($files !== false) {
            foreach ($files as $file) {
                unlink($file);
            }
        }
        if (is_dir($this->tempDir)) {
            rmdir($this->tempDir);
        }
    }

    public function test_per_column_compression(): void
    {
        $path = $this->tempDir . '/per_col_compression.parquet';
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
            Column::string('bio'),
        ]);

        $options = WriterOptions::create()
            ->withColumnCompression('name', Compression::Snappy)
            ->withColumnCompression('bio', Compression::Zstd);

        $writer = new Writer($options);
        $writer->write($path, $schema, [
            ['id' => 1, 'name' => 'Alice', 'bio' => str_repeat('long bio text ', 50)],
            ['id' => 2, 'name' => 'Bob', 'bio' => str_repeat('another bio ', 50)],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame('Alice', $batch[0]['name']);
        self::assertSame(str_repeat('long bio text ', 50), $batch[0]['bio']);
        self::assertSame(2, $batch[1]['id']);
        self::assertSame('Bob', $batch[1]['name']);
    }

    public function test_per_column_encoding(): void
    {
        $path = $this->tempDir . '/per_col_encoding.parquet';
        $schema = Schema::create([
            Column::int32('counter'),
            Column::string('label'),
        ]);

        $options = WriterOptions::create()
            ->withColumnEncoding('counter', Encoding::DeltaBinaryPacked);

        $writer = new Writer($options);
        $writer->write($path, $schema, [
            ['counter' => 100, 'label' => 'first'],
            ['counter' => 200, 'label' => 'second'],
            ['counter' => 300, 'label' => 'third'],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(3, $batch);
        self::assertSame(100, $batch[0]['counter']);
        self::assertSame(200, $batch[1]['counter']);
        self::assertSame(300, $batch[2]['counter']);
        self::assertSame('first', $batch[0]['label']);
        self::assertSame('second', $batch[1]['label']);
        self::assertSame('third', $batch[2]['label']);
    }

    public function test_encoding_enum_values(): void
    {
        self::assertSame(0, Encoding::Plain->value);
        self::assertSame(1, Encoding::PlainDictionary->value);
        self::assertSame(2, Encoding::RleDictionary->value);
        self::assertSame(3, Encoding::DeltaBinaryPacked->value);
        self::assertSame(4, Encoding::DeltaLengthByteArray->value);
        self::assertSame(5, Encoding::DeltaByteArray->value);
        self::assertSame(6, Encoding::ByteStreamSplit->value);
    }
}
