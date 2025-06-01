# dumpqlite

`dumpqlite` is a small Rust crate that extends [`rusqlite`](https://crates.io/crates/rusqlite)'s `Connection` with functionality similar to SQLite's `.dump` command.

## Usage

```toml
[dependencies]
rusqlite = { version = "0.36.0", features = ["bundled"] }
dumpqlite = { git = "https://github.com/NaokiM03/dumpqlite-rs" }
anyhow = "1.0.98" # As you want.
```

```rust,no_run
use dumpqlite::ConnectionExt as _;

fn main() -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open("./foo.db")?;

    let mut writer = std::io::stdout();
    conn.dump(&mut writer)?;

    Ok(())
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
