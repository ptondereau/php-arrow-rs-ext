<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Parquet\Column;
use Parquet\ParquetException;
use Parquet\Schema;
use PHPUnit\Framework\TestCase;

final class SchemaTest extends TestCase
{
    public function test_create_schema_with_columns(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
        ]);

        $columns = $schema->columns();
        self::assertCount(2, $columns);
        self::assertSame('id', $columns[0]->name());
        self::assertSame('name', $columns[1]->name());
    }

    public function test_column_accessors(): void
    {
        $col = Column::int32('age');

        self::assertSame('age', $col->name());
        self::assertSame('int32', $col->type());
        self::assertFalse($col->isRequired());
    }

    public function test_column_nullable_by_default(): void
    {
        $col = Column::string('email');
        self::assertFalse($col->isRequired());

        $required = $col->required();
        self::assertTrue($required->isRequired());
        self::assertFalse($col->isRequired());
    }

    public function test_decimal_column_type_display(): void
    {
        $col = Column::decimal('price', 10, 2);
        self::assertSame('decimal(10,2)', $col->type());
    }

    public function test_empty_schema_throws(): void
    {
        $this->expectException(ParquetException::class);
        Schema::create([]);
    }

    public function test_column_lookup(): void
    {
        $schema = Schema::create([
            Column::int32('id'),
            Column::string('name'),
            Column::double('score'),
        ]);

        $col = $schema->column('name');
        self::assertSame('name', $col->name());
        self::assertSame('string', $col->type());

        self::assertTrue($schema->has('id'));
        self::assertFalse($schema->has('missing'));
    }

    public function test_column_not_found_throws(): void
    {
        $schema = Schema::create([Column::int32('id')]);

        $this->expectException(ParquetException::class);
        $schema->column('nonexistent');
    }
}
