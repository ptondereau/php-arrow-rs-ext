<?php

declare(strict_types=1);

use Parquet\Column;
use Parquet\Reader;
use Parquet\Schema;
use Parquet\Writer;

$rowCount = (int) ($argv[1] ?? getenv('BENCH_ROWS') ?: 100_000);
$outputFile = sys_get_temp_dir() . '/parquet-nested-bench-' . getmypid() . '.parquet';

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

$rows = [];

for ($i = 0; $i < $rowCount; $i++) {
    $rows[] = [
        'index' => $i,
        'order_id' => '254d61c5-22c8-4407-83a2-76f1cab53af2',
        'created_at' => '2025-01-01 12:00:00',
        'updated_at' => $i % 2 === 0 ? '2025-01-01 12:10:00' : null,
        'discount' => $i % 2 === 0 ? 24.4 : null,
        'email' => 'user-' . $i . '@example.com',
        'customer' => 'John Doe ' . $i,
        'address' => [
            'street' => '123 Main St, Apt ' . $i,
            'city' => 'City',
            'zip' => '12345-' . $i,
            'country' => 'PL',
        ],
        'notes' => [
            'Note 1 for order ' . $i,
            'Note 2 for order ' . $i,
            'Note 3 for order ' . $i,
        ],
        'items' => [
            ['sku' => 'SKU_0001', 'quantity' => 1, 'price' => 0.14],
            ['sku' => 'SKU_0002', 'quantity' => 2, 'price' => 25.13],
        ],
        'counters' => [
            'views' => $i * 10,
            'clicks' => $i * 3,
            'conversions' => $i % 5,
        ],
        'tags_by_category' => [
            'color' => ['red', 'blue'],
            'size' => ['M', 'L', 'XL'],
        ],
        'payments' => [
            ['method' => 'credit_card', 'amount' => 99.99, 'paid_at' => '2025-01-01 12:05:00'],
            ['method' => 'paypal', 'amount' => 50.00, 'paid_at' => '2025-01-01 12:06:00'],
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
    ];
}

$startTime = hrtime(true);

(new Writer())->write($outputFile, $schema, $rows);

$durationMs = (hrtime(true) - $startTime) / 1_000_000;
$fileSize = filesize($outputFile);
fwrite(STDERR, sprintf("write-only: %.1fms (file: %.1f MB)\n", $durationMs, $fileSize / 1024 / 1024));
fwrite(STDERR, sprintf("peak memory: %.1f MB\n", memory_get_peak_usage(true) / 1024 / 1024));

@unlink($outputFile);
