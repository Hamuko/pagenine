# Changelog

## [Unreleased]

## [1.3.0] - 2025-11-29

### Added

- Option to ignore threads that have hit bump limit.
- Logs indicate if a thread has hit the bump limit.
- Arguments can be passed as environment variables.

### Fixed

- Failing to fetch the catalog no longer results in the state being reset. This should prevent duplicate notifications being sent in rapid succession when the API is ailing.

### Changed

- Thread titles are now unescaped.
- Tweaked refresh delays.
- Docker images now use Debian trixie.
- Upgraded dependencies.

## [1.2.1] - 2024-12-21

### Changed

- Upgraded dependencies.

## [1.2.0] - 2024-09-15

### Changed

- Improved notifications on macOS.
- Tweaked refresh intervals.

## [1.1.0] - 2024-09-01

### Changed

- User-Agent is sent with API requests to get through Cloudflare.
- Upgraded dependencies.

## [1.0.0] - 2023-11-14

- Initial release.

[Unreleased]: https://github.com/Hamuko/pagenine/compare/1.3.0...HEAD
[1.3.0]: https://github.com/Hamuko/pagenine/compare/1.2.1...1.3.0
[1.2.1]: https://github.com/Hamuko/pagenine/compare/1.2.0...1.2.1
[1.2.0]: https://github.com/Hamuko/pagenine/compare/1.1.0...1.2.0
[1.1.0]: https://github.com/Hamuko/pagenine/compare/1.0.0...1.1.0
[1.0.0]: https://github.com/Hamuko/pagenine/releases/tag/1.0.0
