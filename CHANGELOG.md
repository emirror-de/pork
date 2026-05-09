# Changelog
## v1.0.0
Release date: *2026-08-11*

### CI

#### 🚲 Miscellaneous Tasks

- Updated GitHub workflow [fae4ebb]

### Global changes

#### 🏗  Refactor

- Updated workspace structure [2294e90]
- Further improvement of developer experience [5d42558]

#### 🐞 Bug Fixes

- Replaced std::sync with tokio::sync, Used RwLock instead of Mutex for read heavy operations [35cc76e]

#### 📄 Documentation

- Added documentation and examples [4291d62]
- Updated README.md [32dfd22]
- Updated crate level documentation [7fd2637]

#### 🚲 Miscellaneous Tasks

- Added initial Rust package [4fddec6]
- Removed unused functions [3312bff]
- Added basic usage example [271b7a1]
- Crate pork-proto is no longer partially re-exported by pork [22ad485]
- Cleanup pass [ad32244]
- Updated .gitignore [da2e04c]
- Preparation for v1.0.0 [04fcf62]

#### 🛳  Features

- Initial implementation [d11f36c]
- Added management by binary name [e559cb6]
- Added graceful shutdown [eef3faa]
- Added pork-proto crate [049319d]
- Added restart possibility of a child [a23eb2a]
- Now fully async [7898fe7]
- Added child status tracking [1319ebc]

### Orchestrator

#### 🛳  Features

- Add dependency ordering with timeout and cycle detection [53c14a5]


