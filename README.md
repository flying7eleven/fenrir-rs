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
    version = "0.5.0"
    features = ["ureq", "json"]
    ```

3. Use it 😉

## Development and testing
For easier testing and during the development phase, you can use the example docker-compose.yml to start your own instance of Loki locally.
Just use `docker compose` to start the Loki container as well as a local Grafana instance for viewing the messages:

```shell
docker compose up -d
```

After the containers have started, you can visit [http://localhost:3000/explore](http://localhost:3000/explore) to query messages in your local Loki instance.

## Minimum Supported Rust Version (MSRV)
The MSRV for this tool ist `1.82.0`.

## License
This project is licensed under the MIT License.