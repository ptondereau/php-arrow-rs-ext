<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use PHPUnit\Framework\TestCase;

final class NestedTypesTest extends TestCase
{
    private string $tempDir;

    protected function setUp(): void
    {
        $this->tempDir = sys_get_temp_dir() . '/parquet_nested_' . uniqid();
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

    public function test_list_round_trip(): void
    {
        $schema = Schema::create([
            Column::list('tags', Column::string('element')),
        ]);

        $result = $this->writeAndRead($schema, [
            ['tags' => ['a', 'b', 'c']],
        ]);

        self::assertCount(1, $result);
        self::assertSame(['a', 'b', 'c'], $result[0]['tags']);
    }

    public function test_list_of_integers(): void
    {
        $schema = Schema::create([
            Column::list('numbers', Column::int32('element')),
        ]);

        $result = $this->writeAndRead($schema, [
            ['numbers' => [10, 20, 30]],
            ['numbers' => [42]],
        ]);

        self::assertCount(2, $result);
        self::assertSame([10, 20, 30], $result[0]['numbers']);
        self::assertSame([42], $result[1]['numbers']);
    }

    public function test_list_nullable(): void
    {
        $schema = Schema::create([
            Column::list('items', Column::string('element')),
        ]);

        $result = $this->writeAndRead($schema, [
            ['items' => ['x']],
            ['items' => null],
            ['items' => ['y', 'z']],
        ]);

        self::assertCount(3, $result);
        self::assertSame(['x'], $result[0]['items']);
        self::assertNull($result[1]['items']);
        self::assertSame(['y', 'z'], $result[2]['items']);
    }

    public function test_struct_round_trip(): void
    {
        $schema = Schema::create([
            Column::struct('address', [Column::string('street'), Column::string('city')]),
        ]);

        $result = $this->writeAndRead($schema, [
            ['address' => ['street' => '123 Main St', 'city' => 'Springfield']],
        ]);

        self::assertCount(1, $result);
        self::assertSame('123 Main St', $result[0]['address']['street']);
        self::assertSame('Springfield', $result[0]['address']['city']);
    }

    public function test_struct_nullable(): void
    {
        $schema = Schema::create([
            Column::struct('info', [Column::string('label'), Column::int32('count')]),
        ]);

        $result = $this->writeAndRead($schema, [
            ['info' => ['label' => 'test', 'count' => 5]],
            ['info' => null],
        ]);

        self::assertCount(2, $result);
        self::assertSame('test', $result[0]['info']['label']);
        self::assertSame(5, $result[0]['info']['count']);
        self::assertNull($result[1]['info']);
    }

    public function test_map_round_trip(): void
    {
        $schema = Schema::create([
            Column::map('scores', Column::string('key'), Column::int32('value')),
        ]);

        $result = $this->writeAndRead($schema, [
            ['scores' => ['math' => 95, 'english' => 87]],
        ]);

        self::assertCount(1, $result);
        self::assertSame(95, $result[0]['scores']['math']);
        self::assertSame(87, $result[0]['scores']['english']);
    }

    public function test_map_nullable(): void
    {
        $schema = Schema::create([
            Column::map('meta', Column::string('key'), Column::int32('value')),
        ]);

        $result = $this->writeAndRead($schema, [
            ['meta' => ['a' => 1]],
            ['meta' => null],
        ]);

        self::assertCount(2, $result);
        self::assertSame(1, $result[0]['meta']['a']);
        self::assertNull($result[1]['meta']);
    }

    public function test_nested_schema_type_display(): void
    {
        $listCol = Column::list('tags', Column::string('element'));
        self::assertSame('list<string>', $listCol->type());

        $structCol = Column::struct('address', [Column::string('street'), Column::string('city')]);
        self::assertSame('struct<street:string,city:string>', $structCol->type());

        $mapCol = Column::map('scores', Column::string('key'), Column::int32('value'));
        self::assertSame('map<string,int32>', $mapCol->type());
    }

    public function test_mixed_nested_and_flat(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
            Column::list('tags', Column::string('element')),
            Column::struct('address', [Column::string('street'), Column::string('city')]),
        ]);

        $result = $this->writeAndRead($schema, [
            [
                'id' => 1,
                'name' => 'Alice',
                'tags' => ['admin', 'user'],
                'address' => ['street' => '1 Elm St', 'city' => 'Boston'],
            ],
            [
                'id' => 2,
                'name' => 'Bob',
                'tags' => ['guest'],
                'address' => ['street' => '2 Oak Ave', 'city' => 'Denver'],
            ],
        ]);

        self::assertCount(2, $result);
        self::assertSame(1, $result[0]['id']);
        self::assertSame('Alice', $result[0]['name']);
        self::assertSame(['admin', 'user'], $result[0]['tags']);
        self::assertSame('1 Elm St', $result[0]['address']['street']);
        self::assertSame(2, $result[1]['id']);
        self::assertSame(['guest'], $result[1]['tags']);
        self::assertSame('Denver', $result[1]['address']['city']);
    }
}
