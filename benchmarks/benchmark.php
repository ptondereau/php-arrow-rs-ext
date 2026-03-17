<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Parquet\Column;
use Parquet\Compression;
use Parquet\Reader;
use Parquet\ReaderOptions;
use Parquet\Schema;
use Parquet\Writer;
use Parquet\WriterOptions;

$scales = [10_000, 100_000, 1_000_000];
if (isset($argv[1])) {
    $requested = (int) $argv[1];
    if (!in_array($requested, $scales, true)) {
        fwrite(STDERR, "Invalid scale: {$argv[1]}. Valid: 10000, 100000, 1000000\n");
        exit(1);
    }
    $scales = [$requested];
}

$hasFlowPhp = class_exists(\Flow\Parquet\ParquetFile\Schema\FlatColumn::class);

$categories = ['Electronics', 'Books', 'Clothing', 'Food', 'Sports'];
$tags = ['sale', 'new', 'popular', 'limited', 'exclusive'];

function generateRows(int $count, array $categories, array $tags): array
{
    $rows = [];
    for ($i = 0; $i < $count; $i++) {
        $rows[] = [
            'id' => $i,
            'name' => 'product_' . $i,
            'price' => round($i * 1.99, 2),
            'active' => ($i % 2) === 0,
            'category' => $categories[$i % count($categories)],
            'description' => 'Description for product ' . $i . ' with some padding text to simulate real data',
            'quantity' => $i % 1000,
            'rating' => round(($i % 50) / 10.0, 1),
            'email' => 'user' . $i . '@example.com',
            'tag' => $tags[$i % count($tags)],
        ];
    }
    return $rows;
}

function formatBytes(int $bytes): string
{
    if ($bytes >= 1024 * 1024) {
        return sprintf('%.1f MB', $bytes / (1024 * 1024));
    }
    if ($bytes >= 1024) {
        return sprintf('%.1f KB', $bytes / 1024);
    }
    return $bytes . ' B';
}

function formatMs(float $seconds): string
{
    return sprintf('%.1f', $seconds * 1000);
}

function resetMemory(): void
{
    gc_collect_cycles();
    gc_mem_caches();
}

function buildExtensionSchema(): Schema
{
    return Schema::create([
        Column::int64('id')->required(),
        Column::string('name'),
        Column::double('price'),
        Column::boolean('active'),
        Column::string('category'),
        Column::string('description'),
        Column::int32('quantity'),
        Column::float('rating'),
        Column::string('email'),
        Column::string('tag'),
    ]);
}

function benchExtensionWrite(string $path, array $rows): array
{
    resetMemory();
    $schema = buildExtensionSchema();
    $options = WriterOptions::create()->withCompression(Compression::Snappy);
    $writer = new Writer($options);

    $start = microtime(true);
    $writer->open($path, $schema);
    foreach (array_chunk($rows, 8192) as $chunk) {
        $writer->writeBatch($chunk);
    }
    $writer->close();
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'size' => filesize($path),
    ];
}

function benchExtensionReadAll(string $path): array
{
    resetMemory();
    $reader = new Reader();

    $start = microtime(true);
    $file = $reader->open($path);
    $totalRows = 0;
    while (($batch = $file->readBatch()) !== null) {
        $totalRows += count($batch);
    }
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'rows' => $totalRows,
    ];
}

function benchExtensionReadProjected(string $path): array
{
    resetMemory();
    $options = ReaderOptions::create()->withColumns(['id', 'name', 'price']);
    $reader = new Reader($options);

    $start = microtime(true);
    $file = $reader->open($path);
    $totalRows = 0;
    while (($batch = $file->readBatch()) !== null) {
        $totalRows += count($batch);
    }
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'rows' => $totalRows,
    ];
}

function benchFlowPhpWrite(string $path, array $rows): ?array
{
    if (!class_exists(\Flow\Parquet\ParquetFile\Schema\FlatColumn::class)) {
        return null;
    }

    resetMemory();

    $schema = \Flow\Parquet\ParquetFile\Schema::with(
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::int64('id'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::string('name'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::double('price'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::boolean('active'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::string('category'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::string('description'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::int32('quantity'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::float('rating'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::string('email'),
        \Flow\Parquet\ParquetFile\Schema\FlatColumn::string('tag'),
    );

    $start = microtime(true);
    $writer = new \Flow\Parquet\Writer(
        compression: \Flow\Parquet\ParquetFile\Compressions::SNAPPY,
    );
    $writer->open($path, $schema);
    $writer->writeBatch($rows);
    $writer->close();
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'size' => filesize($path),
    ];
}

function benchFlowPhpReadAll(string $path): ?array
{
    if (!class_exists(\Flow\Parquet\Reader::class)) {
        return null;
    }

    resetMemory();

    $start = microtime(true);
    $reader = new \Flow\Parquet\Reader();
    $file = $reader->read($path);
    $totalRows = 0;
    foreach ($file->values() as $_row) {
        $totalRows++;
    }
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'rows' => $totalRows,
    ];
}

function benchFlowPhpReadProjected(string $path): ?array
{
    if (!class_exists(\Flow\Parquet\Reader::class)) {
        return null;
    }

    resetMemory();

    $start = microtime(true);
    $reader = new \Flow\Parquet\Reader();
    $file = $reader->read($path);
    $totalRows = 0;
    foreach ($file->values(['id', 'name', 'price']) as $_row) {
        $totalRows++;
    }
    $elapsed = microtime(true) - $start;

    return [
        'time' => $elapsed,
        'memory' => memory_get_peak_usage(true),
        'rows' => $totalRows,
    ];
}

$results = [];

foreach ($scales as $scale) {
    $label = number_format($scale);
    fwrite(STDERR, "Generating {$label} rows...\n");
    $rows = generateRows($scale, $categories, $tags);

    $extPath = '/tmp/parquet_bench_ext_' . $scale . '.parquet';
    $flowPath = '/tmp/parquet_bench_flow_' . $scale . '.parquet';

    fwrite(STDERR, "[{$label}] Extension: Write...\n");
    $extWrite = benchExtensionWrite($extPath, $rows);

    fwrite(STDERR, "[{$label}] Extension: Read All...\n");
    $extReadAll = benchExtensionReadAll($extPath);

    fwrite(STDERR, "[{$label}] Extension: Read Projected...\n");
    $extReadProjected = benchExtensionReadProjected($extPath);

    $flowWrite = null;
    $flowReadAll = null;
    $flowReadProjected = null;

    if ($hasFlowPhp) {
        fwrite(STDERR, "[{$label}] flow-php: Write...\n");
        $flowWrite = benchFlowPhpWrite($flowPath, $rows);

        if ($flowWrite !== null) {
            fwrite(STDERR, "[{$label}] flow-php: Read All...\n");
            $flowReadAll = benchFlowPhpReadAll($flowPath);

            fwrite(STDERR, "[{$label}] flow-php: Read Projected...\n");
            $flowReadProjected = benchFlowPhpReadProjected($flowPath);
        }
    }

    $results[] = [
        'scale' => $label,
        'write' => ['ext' => $extWrite, 'flow' => $flowWrite],
        'read_all' => ['ext' => $extReadAll, 'flow' => $flowReadAll],
        'read_projected' => ['ext' => $extReadProjected, 'flow' => $flowReadProjected],
    ];

    unset($rows);
    @unlink($extPath);
    @unlink($flowPath);
}

echo "\n## Parquet Benchmark Results\n\n";
echo "| Scale | Operation | Extension (ms) | Extension Memory | flow-php (ms) | flow-php Memory | Speedup |\n";
echo "|-------|-----------|----------------|------------------|---------------|-----------------|--------|\n";

foreach ($results as $r) {
    $ops = [
        'Write' => $r['write'],
        'Read All' => $r['read_all'],
        'Read Projected' => $r['read_projected'],
    ];

    foreach ($ops as $opName => $data) {
        $extMs = formatMs($data['ext']['time']);
        $extMem = formatBytes($data['ext']['memory']);

        if ($opName === 'Write') {
            $extMs .= ' (' . formatBytes($data['ext']['size']) . ')';
        }

        if ($data['flow'] !== null) {
            $flowMs = formatMs($data['flow']['time']);
            $flowMem = formatBytes($data['flow']['memory']);
            $speedup = sprintf('%.1fx', $data['flow']['time'] / $data['ext']['time']);

            if ($opName === 'Write') {
                $flowMs .= ' (' . formatBytes($data['flow']['size']) . ')';
            }
        } else {
            $flowMs = 'N/A';
            $flowMem = 'N/A';
            $speedup = '-';
        }

        echo "| {$r['scale']} | {$opName} | {$extMs} | {$extMem} | {$flowMs} | {$flowMem} | {$speedup} |\n";
    }
}

echo "\n";
