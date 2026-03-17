<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\ParquetException;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use PHPUnit\Framework\TestCase;

final class WriteReadTest extends TestCase
{
    private string $tempDir;

    protected function setUp(): void
    {
        $this->tempDir = sys_get_temp_dir() . '/parquet_test_' . uniqid();
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

    private function filePath(string $name = 'test.parquet'): string
    {
        return $this->tempDir . '/' . $name;
    }

    public function test_write_and_read_basic(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
        ]);

        $writer = new Writer();
        $writer->write($path, $schema, [
            ['id' => 1, 'name' => 'Alice'],
            ['id' => 2, 'name' => 'Bob'],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame('Alice', $batch[0]['name']);
        self::assertSame(2, $batch[1]['id']);
        self::assertSame('Bob', $batch[1]['name']);

        self::assertNull($file->readBatch());
    }

    public function test_incremental_write(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('value'),
        ]);

        $writer = new Writer();
        $writer->open($path, $schema);
        $writer->writeBatch([
            ['id' => 1, 'value' => 'first'],
        ]);
        $writer->writeBatch([
            ['id' => 2, 'value' => 'second'],
        ]);
        $writer->close();

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame(2, $batch[1]['id']);
    }

    public function test_nullable_values(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('note'),
        ]);

        $writer = new Writer();
        $writer->write($path, $schema, [
            ['id' => 1, 'note' => 'present'],
            ['id' => 2, 'note' => null],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertSame('present', $batch[0]['note']);
        self::assertNull($batch[1]['note']);
    }

    public function test_required_column_null_throws(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id')->required(),
            Column::string('name'),
        ]);

        $this->expectException(ParquetException::class);

        $writer = new Writer();
        $writer->write($path, $schema, [
            ['id' => null, 'name' => 'Alice'],
        ]);
    }

    public function test_column_projection(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
            Column::double('score'),
        ]);

        $writer = new Writer();
        $writer->write($path, $schema, [
            ['id' => 1, 'name' => 'Alice', 'score' => 9.5],
        ]);

        $options = \Parquet\ReaderOptions::create()->withColumns(['id', 'score']);
        $reader = new Reader($options);
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(1, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame(9.5, $batch[0]['score']);
        self::assertArrayNotHasKey('name', $batch[0]);
    }

    public function test_writer_state_errors(): void
    {
        $writer = new Writer();

        $this->expectException(ParquetException::class);
        $writer->writeBatch([['id' => 1]]);
    }

    public function test_metadata(): void
    {
        $path = $this->filePath();
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
        ]);

        $writer = new Writer();
        $writer->write($path, $schema, [
            ['id' => 1, 'name' => 'Alice'],
            ['id' => 2, 'name' => 'Bob'],
            ['id' => 3, 'name' => 'Charlie'],
        ]);

        $reader = new Reader();
        $file = $reader->open($path);
        $meta = $file->metadata();

        self::assertSame(3, $meta->rowCount());
        self::assertGreaterThanOrEqual(1, $meta->rowGroupCount());
        self::assertGreaterThanOrEqual(1, $meta->version());
    }

    public function test_file_not_found(): void
    {
        $this->expectException(ParquetException::class);

        $reader = new Reader();
        $reader->open('/nonexistent/path/file.parquet');
    }
}
