# Contributing

## Requirements

- PHP 8.2+
- Rust toolchain (https://rustup.rs)
- Composer

## Setup

```bash
git clone https://github.com/ptondereau/php-arrow-rs-ext.git
cd php-arrow-rs-ext
composer install
cargo build --release
```

## Running tests

```bash
php -dextension=./target/release/libparquet.so vendor/bin/phpunit
```

## Linting

```bash
cargo clippy -- -D warnings
```

## Pull requests

- Run the full test suite before submitting
- Follow existing code patterns
- Add tests for new features
