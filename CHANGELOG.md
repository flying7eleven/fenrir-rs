# Changelog
All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

### Added
- The docker example environment now also includes a nginx reverse proxy for authentication purposes
- Add support for sending logs to Loki using a reverse proxy for authentication via Basic Auth

### Changed
- Rename the `logging_level` to `level` to comply with the standard for Grafana
- Replace `ureq` with `hyper` for sending the logs to Loki

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
