# Changelog
All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

### Added
- Add a new example for the use of the library without frameworks like `fern`
- Add the `NoopBackend` for dropping all messages and not sending them to any Loki instance
- Add a feature for selecting the `ureq` network backend
- Add a feature for selecting the `json` as a serialization method

### Changed
- Refactor the code to be able to deal with different network backends for sending information to a Loki instance
- Refactor the code to be able to switch the serialization type for the send messages

## 0.2.0 - 2023-05-21

### Added
- The docker example environment now also includes a nginx reverse proxy for authentication purposes
- Add support for sending logs to Loki using a reverse proxy for authentication via Basic Auth

### Changed
- Rename the `logging_level` to `level` to comply with the standard for Grafana
- Change the way the builder for the `Fenrir` is obtained to make it easier to use
- Reduce the minimum supported Rust version to 1.57.0

## 0.1.1 - 2023-05-12

### Added
- Add the content of the README.md file to the documentation on [docs.rs](https://docs.rs/fenrir-rs)
- Add a container setup for easier local testing (using `docker compose`)
- Add a `logging_level` tag to each logged entry send to Loki

### Changed
- Correctly added the license tag; it shows up now up correctly on [crates.io](https://crates.io/crates/fenrir-rs)

## 0.1.0 - 2023-05-10

### Added
- Initial support for basic logging in Loki using the JSON interface
