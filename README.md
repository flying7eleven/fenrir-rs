# fenrir-rs
[![Build status](https://github.com/flying7eleven/fenrir-rs/actions/workflows/build.yml/badge.svg)](https://github.com/flying7eleven/fenrir-rs/actions/workflows/build.yml)
[![Crates.io](https://img.shields.io/crates/v/fenrir-rs.svg)](https://crates.io/crates/fenrir-rs)
[![Crates.io](https://img.shields.io/crates/l/fenrir-rs.svg)](https://crates.io/crates/fenrir-rs)
[![Documentation](https://img.shields.io/badge/documentation-docs.rs-blue.svg)](https://docs.rs/fenrir-rs)

Fenrir (_Fenrir was the son of the trickster god Loki and the giantess Angrboða_) facilitates collecting and shipping your applications logs to a [Loki](https://grafana.com/oss/loki/) instance.
It does this by integrating with the log crate.

## Getting Started

> Examples are available for several use-cases, check out the [examples folder](https://github.com/flying7eleven/fenrir-rs/tree/main/examples).

1. Create a new Rust project: `cargo new example`
2. Add dependencies to this create to your **Cargo.toml** file:

    ```toml
    [dependencies.fenrir-rs]
    version = "0.1.0"
    default-features = false
    ```

3. Use it 😉

## Minimum Supported Rust Version (MSRV)
The MSRV for this tool ist `1.69.0` since it uses rust 2021 edition features.

## License
This project is licensed under the MIT License.
