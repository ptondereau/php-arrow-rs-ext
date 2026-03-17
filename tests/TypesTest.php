<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use PHPUnit\Framework\TestCase;

final class TypesTest extends TestCase
{
    private string $tempDir;

    protected function setUp(): void
    {
        $this->tempDir = sys_get_temp_dir() . '/parquet_types_' . uniqid();
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
     * @param list<array<string, mixed>> $rows
     * @return list<array<string, mixed>>
     */
    private function writeAndRead(Schema $schema, array $rows): array
    {
        $path = $this->tempDir . '/' . uniqid() . '.parquet';

        $writer = new Writer();
        $writer->write($path, $schema, $rows);

        $reader = new Reader();
        $file = $reader->open($path);
        $batch = $file->readBatch();

        self::assertNotNull($batch);

        return $batch;
    }

    public function test_boolean_round_trip(): void
    {
        $schema = Schema::create([Column::boolean('flag')]);
        $result = $this->writeAndRead($schema, [
            ['flag' => true],
            ['flag' => false],
        ]);

        self::assertSame(true, $result[0]['flag']);
        self::assertSame(false, $result[1]['flag']);
    }

    public function test_int32_round_trip(): void
    {
        $schema = Schema::create([Column::int32('value')]);
        $result = $this->writeAndRead($schema, [
            ['value' => 0],
            ['value' => 42],
            ['value' => -100],
        ]);

        self::assertSame(0, $result[0]['value']);
        self::assertSame(42, $result[1]['value']);
        self::assertSame(-100, $result[2]['value']);
    }

    public function test_int64_round_trip(): void
    {
        $schema = Schema::create([Column::int64('big')]);
        $result = $this->writeAndRead($schema, [
            ['big' => 0],
            ['big' => PHP_INT_MAX],
            ['big' => PHP_INT_MIN],
        ]);

        self::assertSame(0, $result[0]['big']);
        self::assertSame(PHP_INT_MAX, $result[1]['big']);
        self::assertSame(PHP_INT_MIN, $result[2]['big']);
    }

    public function test_float_round_trip(): void
    {
        $schema = Schema::create([Column::float('val')]);
        $result = $this->writeAndRead($schema, [
            ['val' => 1.5],
            ['val' => -3.14],
        ]);

        self::assertEqualsWithDelta(1.5, $result[0]['val'], 0.001);
        self::assertEqualsWithDelta(-3.14, $result[1]['val'], 0.01);
    }

    public function test_double_round_trip(): void
    {
        $schema = Schema::create([Column::double('precise')]);
        $result = $this->writeAndRead($schema, [
            ['precise' => 3.141592653589793],
            ['precise' => -2.718281828459045],
        ]);

        self::assertSame(3.141592653589793, $result[0]['precise']);
        self::assertSame(-2.718281828459045, $result[1]['precise']);
    }

    public function test_string_round_trip(): void
    {
        $schema = Schema::create([Column::string('text')]);
        $result = $this->writeAndRead($schema, [
            ['text' => 'hello'],
            ['text' => ''],
            ['text' => 'unicode: '],
        ]);

        self::assertSame('hello', $result[0]['text']);
        self::assertSame('', $result[1]['text']);
        self::assertSame('unicode: ', $result[2]['text']);
    }

    public function test_date_round_trip(): void
    {
        $schema = Schema::create([Column::date('day')]);
        $result = $this->writeAndRead($schema, [
            ['day' => '2024-01-15'],
            ['day' => '1970-01-01'],
        ]);

        self::assertSame('2024-01-15', $result[0]['day']);
        self::assertSame('1970-01-01', $result[1]['day']);
    }

    public function test_datetime_round_trip(): void
    {
        $schema = Schema::create([Column::dateTime('ts')]);
        $result = $this->writeAndRead($schema, [
            ['ts' => '2024-01-15T10:30:00Z'],
        ]);

        self::assertSame('2024-01-15T10:30:00.000000Z', $result[0]['ts']);
    }

    public function test_time_round_trip(): void
    {
        $schema = Schema::create([Column::time('t')]);
        $result = $this->writeAndRead($schema, [
            ['t' => '14:30:00'],
            ['t' => '09:05:30.123456'],
        ]);

        self::assertSame('14:30:00.000000', $result[0]['t']);
        self::assertSame('09:05:30.123456', $result[1]['t']);
    }

    public function test_decimal_round_trip(): void
    {
        $schema = Schema::create([Column::decimal('amount', 10, 2)]);
        $result = $this->writeAndRead($schema, [
            ['amount' => '123.45'],
            ['amount' => '-99.99'],
            ['amount' => '0.01'],
        ]);

        self::assertSame('123.45', $result[0]['amount']);
        self::assertSame('-99.99', $result[1]['amount']);
        self::assertSame('0.01', $result[2]['amount']);
    }

    public function test_uuid_round_trip(): void
    {
        $uuid = '550e8400-e29b-41d4-a716-446655440000';
        $schema = Schema::create([Column::uuid('id')]);
        $result = $this->writeAndRead($schema, [
            ['id' => $uuid],
        ]);

        self::assertSame($uuid, $result[0]['id']);
    }

    public function test_json_round_trip(): void
    {
        $json = '{"key":"value","num":42}';
        $schema = Schema::create([Column::json('data')]);
        $result = $this->writeAndRead($schema, [
            ['data' => $json],
        ]);

        self::assertSame($json, $result[0]['data']);
    }
}
