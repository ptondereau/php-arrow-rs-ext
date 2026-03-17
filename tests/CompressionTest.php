<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\Compression;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use Parquet\WriterOptions;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class CompressionTest extends TestCase
{
    private string $tempDir;

    protected function setUp(): void
    {
        $this->tempDir = sys_get_temp_dir() . '/parquet_compression_' . uniqid();
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

    /**
     * @return array<string, array{Compression}>
     */
    public static function compressionProvider(): array
    {
        return [
            'uncompressed' => [Compression::Uncompressed],
            'snappy' => [Compression::Snappy],
            'gzip' => [Compression::Gzip],
            'brotli' => [Compression::Brotli],
            'lz4raw' => [Compression::Lz4Raw],
            'zstd' => [Compression::Zstd],
        ];
    }

    #[DataProvider('compressionProvider')]
    public function test_write_read_with_compression(Compression $codec): void
    {
        $path = $this->tempDir . '/' . $codec->name . '.parquet';
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('data'),
        ]);

        $options = WriterOptions::create()->withCompression($codec);
        $writer = new Writer($options);
        $writer->write($path, $schema, [
            ['id' => 1, 'data' => 'hello world'],
            ['id' => 2, 'data' => str_repeat('compressed data ', 100)],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame('hello world', $batch[0]['data']);
        self::assertSame(2, $batch[1]['id']);
        self::assertSame(str_repeat('compressed data ', 100), $batch[1]['data']);
    }
}
