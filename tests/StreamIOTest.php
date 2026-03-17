<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\Compression;
use Parquet\ParquetException;
use Parquet\Reader;
use Parquet\ReaderOptions;
use Parquet\Schema;
use Parquet\Writer;
use Parquet\WriterOptions;
use PHPUnit\Framework\TestCase;

final class StreamIOTest extends TestCase
{
    public function test_write_to_string_and_read(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
        ]);

        $writer = new Writer();
        $data = $writer->writeToString($schema, [
            ['id' => 1, 'name' => 'Alice'],
            ['id' => 2, 'name' => 'Bob'],
        ]);

        self::assertNotEmpty($data);

        $reader = new Reader();
        $file = $reader->openString($data);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame('Alice', $batch[0]['name']);
        self::assertSame(2, $batch[1]['id']);
        self::assertSame('Bob', $batch[1]['name']);
    }

    public function test_incremental_buffer_write(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('value'),
        ]);

        $writer = new Writer();
        $writer->openBuffer($schema);
        $writer->writeBatch([
            ['id' => 1, 'value' => 'first'],
            ['id' => 2, 'value' => 'second'],
        ]);
        $writer->writeBatch([
            ['id' => 3, 'value' => 'third'],
        ]);
        $writer->close();

        $buffer = $writer->getBuffer();
        self::assertNotEmpty($buffer);

        $reader = new Reader();
        $file = $reader->openString($buffer);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(3, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame('first', $batch[0]['value']);
        self::assertSame(2, $batch[1]['id']);
        self::assertSame('second', $batch[1]['value']);
        self::assertSame(3, $batch[2]['id']);
        self::assertSame('third', $batch[2]['value']);
    }

    public function test_open_string_metadata(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
        ]);

        $writer = new Writer();
        $data = $writer->writeToString($schema, [
            ['id' => 1, 'name' => 'Alice'],
            ['id' => 2, 'name' => 'Bob'],
            ['id' => 3, 'name' => 'Charlie'],
        ]);

        $reader = new Reader();
        $file = $reader->openString($data);
        $meta = $file->metadata();

        self::assertSame(3, $meta->rowCount());
        self::assertGreaterThanOrEqual(1, $meta->rowGroupCount());
        self::assertGreaterThanOrEqual(1, $meta->version());
    }

    public function test_open_string_with_projection(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
            Column::double('score'),
        ]);

        $writer = new Writer();
        $data = $writer->writeToString($schema, [
            ['id' => 1, 'name' => 'Alice', 'score' => 9.5],
            ['id' => 2, 'name' => 'Bob', 'score' => 8.0],
        ]);

        $options = ReaderOptions::create()->withColumns(['id', 'score']);
        $reader = new Reader($options);
        $file = $reader->openString($data);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame(9.5, $batch[0]['score']);
        self::assertArrayNotHasKey('name', $batch[0]);
    }

    public function test_write_to_string_with_compression(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('data'),
        ]);

        $options = WriterOptions::create()->withCompression(Compression::Snappy);
        $writer = new Writer($options);
        $data = $writer->writeToString($schema, [
            ['id' => 1, 'data' => str_repeat('compressed payload ', 100)],
            ['id' => 2, 'data' => str_repeat('another payload ', 100)],
        ]);

        self::assertNotEmpty($data);

        $reader = new Reader();
        $file = $reader->openString($data);
        $batch = $file->readBatch();

        self::assertNotNull($batch);
        self::assertCount(2, $batch);
        self::assertSame(1, $batch[0]['id']);
        self::assertSame(str_repeat('compressed payload ', 100), $batch[0]['data']);
    }

    public function test_empty_buffer_throws(): void
    {
        $writer = new Writer();

        $this->expectException(ParquetException::class);
        $writer->getBuffer();
    }
}
