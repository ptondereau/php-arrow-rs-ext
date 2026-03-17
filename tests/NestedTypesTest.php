<?php

declare(strict_types=1);

namespace Parquet\Tests;

use Closure;
use Generator;
use Parquet\Column;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;
use PHPUnit\Framework\Attributes\DataProvider;
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

    public static function typeDisplayProvider(): Generator
    {
        yield 'list' => [
            fn () => Column::list('tags', Column::string('element')),
            'list<string>',
        ];
        yield 'struct' => [
            fn () => Column::struct('address', [Column::string('street'), Column::string('city')]),
            'struct<street:string,city:string>',
        ];
        yield 'map' => [
            fn () => Column::map('scores', Column::string('key'), Column::int32('value')),
            'map<string,int32>',
        ];
    }

    #[DataProvider('typeDisplayProvider')]
    public function test_type_display(Closure $columnFactory, string $expectedType): void
    {
        self::assertSame($expectedType, $columnFactory()->type());
    }

    public static function roundTripProvider(): Generator
    {
        yield 'list of strings' => [
            fn () => [
                Schema::create([Column::list('tags', Column::string('element'))]),
                [['tags' => ['a', 'b', 'c']]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertSame(['a', 'b', 'c'], $result[0]['tags']);
            },
        ];

        yield 'list of integers' => [
            fn () => [
                Schema::create([Column::list('numbers', Column::int32('element'))]),
                [
                    ['numbers' => [10, 20, 30]],
                    ['numbers' => [42]],
                ],
            ],
            function (array $result): void {
                self::assertCount(2, $result);
                self::assertSame([10, 20, 30], $result[0]['numbers']);
                self::assertSame([42], $result[1]['numbers']);
            },
        ];

        yield 'struct' => [
            fn () => [
                Schema::create([
                    Column::struct('address', [Column::string('street'), Column::string('city')]),
                ]),
                [['address' => ['street' => '123 Main St', 'city' => 'Springfield']]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertSame('123 Main St', $result[0]['address']['street']);
                self::assertSame('Springfield', $result[0]['address']['city']);
            },
        ];

        yield 'map string to int' => [
            fn () => [
                Schema::create([
                    Column::map('scores', Column::string('key'), Column::int32('value')),
                ]),
                [['scores' => ['math' => 95, 'english' => 87]]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertSame(95, $result[0]['scores']['math']);
                self::assertSame(87, $result[0]['scores']['english']);
            },
        ];

        yield 'list of structs' => [
            fn () => [
                Schema::create([
                    Column::list('items', Column::struct('element', [
                        Column::string('sku'),
                        Column::int32('quantity'),
                        Column::float('price'),
                    ])),
                ]),
                [
                    ['items' => [
                        ['sku' => 'SKU_001', 'quantity' => 1, 'price' => 9.99],
                        ['sku' => 'SKU_002', 'quantity' => 3, 'price' => 25.50],
                    ]],
                    ['items' => [
                        ['sku' => 'SKU_003', 'quantity' => 5, 'price' => 0.14],
                    ]],
                ],
            ],
            function (array $result): void {
                self::assertCount(2, $result);
                self::assertCount(2, $result[0]['items']);
                self::assertSame('SKU_001', $result[0]['items'][0]['sku']);
                self::assertSame(1, $result[0]['items'][0]['quantity']);
                self::assertSame('SKU_002', $result[0]['items'][1]['sku']);
                self::assertSame(3, $result[0]['items'][1]['quantity']);
                self::assertCount(1, $result[1]['items']);
                self::assertSame('SKU_003', $result[1]['items'][0]['sku']);
            },
        ];

        yield 'map with list values' => [
            fn () => [
                Schema::create([
                    Column::map('tags_by_category', Column::string('key'),
                        Column::list('value', Column::string('element'))
                    ),
                ]),
                [['tags_by_category' => [
                    'color' => ['red', 'blue'],
                    'size' => ['M', 'L', 'XL'],
                ]]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertSame(['red', 'blue'], $result[0]['tags_by_category']['color']);
                self::assertSame(['M', 'L', 'XL'], $result[0]['tags_by_category']['size']);
            },
        ];

        yield 'map with list of structs values' => [
            fn () => [
                Schema::create([
                    Column::map('attributes', Column::string('key'),
                        Column::list('value', Column::struct('element', [
                            Column::string('name'),
                            Column::string('value'),
                        ]))
                    ),
                ]),
                [['attributes' => [
                    'material' => [
                        ['name' => 'fabric', 'value' => 'cotton'],
                        ['name' => 'weight', 'value' => '200g'],
                    ],
                    'dimensions' => [
                        ['name' => 'width', 'value' => '30cm'],
                    ],
                ]]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertCount(2, $result[0]['attributes']['material']);
                self::assertSame('fabric', $result[0]['attributes']['material'][0]['name']);
                self::assertSame('cotton', $result[0]['attributes']['material'][0]['value']);
                self::assertSame('200g', $result[0]['attributes']['material'][1]['value']);
                self::assertCount(1, $result[0]['attributes']['dimensions']);
                self::assertSame('width', $result[0]['attributes']['dimensions'][0]['name']);
            },
        ];

        yield 'list of structs with datetime' => [
            fn () => [
                Schema::create([
                    Column::list('payments', Column::struct('element', [
                        Column::string('method'),
                        Column::float('amount'),
                        Column::dateTime('paid_at'),
                    ])),
                ]),
                [['payments' => [
                    ['method' => 'card', 'amount' => 99.99, 'paid_at' => '2025-01-01 12:05:00'],
                    ['method' => 'paypal', 'amount' => 50.0, 'paid_at' => '2025-01-01 12:06:00'],
                ]]],
            ],
            function (array $result): void {
                self::assertCount(1, $result);
                self::assertCount(2, $result[0]['payments']);
                self::assertSame('card', $result[0]['payments'][0]['method']);
                self::assertStringContainsString('2025-01-01', $result[0]['payments'][0]['paid_at']);
                self::assertSame('paypal', $result[0]['payments'][1]['method']);
            },
        ];

        yield 'mixed nested and flat' => [
            fn () => [
                Schema::create([
                    Column::int32('id'),
                    Column::string('name'),
                    Column::list('tags', Column::string('element')),
                    Column::struct('address', [Column::string('street'), Column::string('city')]),
                ]),
                [
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
                ],
            ],
            function (array $result): void {
                self::assertCount(2, $result);
                self::assertSame(1, $result[0]['id']);
                self::assertSame('Alice', $result[0]['name']);
                self::assertSame(['admin', 'user'], $result[0]['tags']);
                self::assertSame('1 Elm St', $result[0]['address']['street']);
                self::assertSame(2, $result[1]['id']);
                self::assertSame(['guest'], $result[1]['tags']);
                self::assertSame('Denver', $result[1]['address']['city']);
            },
        ];
    }

    #[DataProvider('roundTripProvider')]
    public function test_round_trip(Closure $setup, Closure $assertions): void
    {
        [$schema, $rows] = $setup();
        $result = $this->writeAndRead($schema, $rows);
        $assertions($result);
    }

    public static function nullableProvider(): Generator
    {
        yield 'list' => [
            fn () => [
                Schema::create([Column::list('items', Column::string('element'))]),
                [
                    ['items' => ['x']],
                    ['items' => null],
                    ['items' => ['y', 'z']],
                ],
            ],
            function (array $result): void {
                self::assertCount(3, $result);
                self::assertSame(['x'], $result[0]['items']);
                self::assertNull($result[1]['items']);
                self::assertSame(['y', 'z'], $result[2]['items']);
            },
        ];

        yield 'struct' => [
            fn () => [
                Schema::create([
                    Column::struct('info', [Column::string('label'), Column::int32('count')]),
                ]),
                [
                    ['info' => ['label' => 'test', 'count' => 5]],
                    ['info' => null],
                ],
            ],
            function (array $result): void {
                self::assertCount(2, $result);
                self::assertSame('test', $result[0]['info']['label']);
                self::assertSame(5, $result[0]['info']['count']);
                self::assertNull($result[1]['info']);
            },
        ];

        yield 'map' => [
            fn () => [
                Schema::create([
                    Column::map('meta', Column::string('key'), Column::int32('value')),
                ]),
                [
                    ['meta' => ['a' => 1]],
                    ['meta' => null],
                ],
            ],
            function (array $result): void {
                self::assertCount(2, $result);
                self::assertSame(1, $result[0]['meta']['a']);
                self::assertNull($result[1]['meta']);
            },
        ];

        yield 'list of structs' => [
            fn () => [
                Schema::create([
                    Column::list('payments', Column::struct('element', [
                        Column::string('method'),
                        Column::float('amount'),
                    ])),
                ]),
                [
                    ['payments' => [['method' => 'card', 'amount' => 99.99]]],
                    ['payments' => null],
                    ['payments' => [['method' => 'paypal', 'amount' => 50.0]]],
                ],
            ],
            function (array $result): void {
                self::assertCount(3, $result);
                self::assertSame('card', $result[0]['payments'][0]['method']);
                self::assertNull($result[1]['payments']);
                self::assertSame('paypal', $result[2]['payments'][0]['method']);
            },
        ];
    }

    #[DataProvider('nullableProvider')]
    public function test_nullable(Closure $setup, Closure $assertions): void
    {
        [$schema, $rows] = $setup();
        $result = $this->writeAndRead($schema, $rows);
        $assertions($result);
    }

    public function test_full_nested_schema(): void
    {
        $schema = Schema::create([
            Column::int32('index'),
            Column::string('order_id'),
            Column::dateTime('created_at'),
            Column::dateTime('updated_at'),
            Column::float('discount'),
            Column::string('email'),
            Column::string('customer'),
            Column::struct('address', [
                Column::string('street'),
                Column::string('city'),
                Column::string('zip'),
                Column::string('country'),
            ]),
            Column::list('notes', Column::string('element')),
            Column::list('items', Column::struct('element', [
                Column::string('sku'),
                Column::int32('quantity'),
                Column::float('price'),
            ])),
            Column::map('counters', Column::string('key'), Column::int32('value')),
            Column::map('tags_by_category', Column::string('key'),
                Column::list('value', Column::string('element'))
            ),
            Column::list('payments', Column::struct('element', [
                Column::string('method'),
                Column::float('amount'),
                Column::dateTime('paid_at'),
            ])),
            Column::map('attributes', Column::string('key'),
                Column::list('value', Column::struct('element', [
                    Column::string('name'),
                    Column::string('value'),
                ]))
            ),
        ]);

        $result = $this->writeAndRead($schema, [
            [
                'index' => 0,
                'order_id' => '254d61c5-22c8-4407-83a2-76f1cab53af2',
                'created_at' => '2025-01-01 12:00:00',
                'updated_at' => '2025-01-01 12:10:00',
                'discount' => 24.4,
                'email' => 'user-0@example.com',
                'customer' => 'John Doe 0',
                'address' => [
                    'street' => '123 Main St, Apt 0',
                    'city' => 'City',
                    'zip' => '12345-0',
                    'country' => 'PL',
                ],
                'notes' => ['Note 1', 'Note 2', 'Note 3'],
                'items' => [
                    ['sku' => 'SKU_0001', 'quantity' => 1, 'price' => 0.14],
                    ['sku' => 'SKU_0002', 'quantity' => 2, 'price' => 25.13],
                ],
                'counters' => ['views' => 0, 'clicks' => 0, 'conversions' => 0],
                'tags_by_category' => [
                    'color' => ['red', 'blue'],
                    'size' => ['M', 'L', 'XL'],
                ],
                'payments' => [
                    ['method' => 'credit_card', 'amount' => 99.99, 'paid_at' => '2025-01-01 12:05:00'],
                    ['method' => 'paypal', 'amount' => 50.0, 'paid_at' => '2025-01-01 12:06:00'],
                ],
                'attributes' => [
                    'material' => [
                        ['name' => 'fabric', 'value' => 'cotton'],
                        ['name' => 'weight', 'value' => '200g'],
                    ],
                    'dimensions' => [
                        ['name' => 'width', 'value' => '30cm'],
                        ['name' => 'height', 'value' => '40cm'],
                        ['name' => 'depth', 'value' => '10cm'],
                    ],
                ],
            ],
            [
                'index' => 1,
                'order_id' => '254d61c5-22c8-4407-83a2-76f1cab53af2',
                'created_at' => '2025-01-01 12:00:00',
                'updated_at' => null,
                'discount' => null,
                'email' => 'user-1@example.com',
                'customer' => 'John Doe 1',
                'address' => [
                    'street' => '123 Main St, Apt 1',
                    'city' => 'City',
                    'zip' => '12345-1',
                    'country' => 'PL',
                ],
                'notes' => ['Note 1', 'Note 2', 'Note 3'],
                'items' => [
                    ['sku' => 'SKU_0001', 'quantity' => 1, 'price' => 0.14],
                ],
                'counters' => ['views' => 10, 'clicks' => 3, 'conversions' => 1],
                'tags_by_category' => [
                    'color' => ['red', 'blue'],
                    'size' => ['M', 'L', 'XL'],
                ],
                'payments' => [
                    ['method' => 'credit_card', 'amount' => 99.99, 'paid_at' => '2025-01-01 12:05:00'],
                ],
                'attributes' => [
                    'material' => [
                        ['name' => 'fabric', 'value' => 'cotton'],
                    ],
                ],
            ],
        ]);

        self::assertCount(2, $result);

        self::assertSame(0, $result[0]['index']);
        self::assertSame('254d61c5-22c8-4407-83a2-76f1cab53af2', $result[0]['order_id']);
        self::assertSame('PL', $result[0]['address']['country']);
        self::assertSame(['Note 1', 'Note 2', 'Note 3'], $result[0]['notes']);
        self::assertCount(2, $result[0]['items']);
        self::assertSame('SKU_0001', $result[0]['items'][0]['sku']);
        self::assertSame(0, $result[0]['counters']['views']);
        self::assertSame(['red', 'blue'], $result[0]['tags_by_category']['color']);
        self::assertSame(['M', 'L', 'XL'], $result[0]['tags_by_category']['size']);
        self::assertCount(2, $result[0]['payments']);
        self::assertSame('credit_card', $result[0]['payments'][0]['method']);
        self::assertCount(2, $result[0]['attributes']['material']);
        self::assertCount(3, $result[0]['attributes']['dimensions']);

        self::assertSame(1, $result[1]['index']);
        self::assertNull($result[1]['updated_at']);
        self::assertNull($result[1]['discount']);
        self::assertCount(1, $result[1]['items']);
        self::assertSame(10, $result[1]['counters']['views']);
        self::assertCount(1, $result[1]['payments']);
        self::assertCount(1, $result[1]['attributes']['material']);
    }
}
