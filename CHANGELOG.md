# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-04-24

### New Features
- *(volume)* Add `Volume::is_empty()`.
- *(network)* Add `Network::is_empty()`.
  - Add `is_empty()` methods to `network::{Network, Ipam, IpamConfig}`.
- *(service)* Add `service::Logging::is_empty()`.
- *(service)* Add `service::healthcheck::Command::is_empty()`.
- *(service)* Add `service::Build::is_empty()`.
- *(service)* Add `service::BlkioConfig::is_empty()`.
- *(service)* Add `is_empty()` methods to `service::volumes::mount` types.
  - Add `is_empty()` methods to `service::volumes::mount::{VolumeOptions, BindOptions, TmpfsOptions}`.
- *(service)* Implement `Default` for `service::deploy::resources::Device`.
- *(service)* Add `service::deploy::Resources::is_empty()`.
  - Add `is_empty()` methods to `service::deploy::resources::{Resources, Limits, Reservations, Device, GenericResource, DiscreteResourceSpec}`.
- *(service)* Add `service::Deploy::is_empty()`.
  - Add `is_empty()` methods to `service::deploy::{Deploy, Placement, Preference, RestartPolicy, UpdateRollbackConfig}`.
- *(service)* Implement `Default` for `service::Healthcheck`.
- *(service)* Add `into_short()` methods to `service::volumes::mount::{Volume, Bind}`.
- *(service)* Implement `Display` for `service::blkio_config::Weight`.
- *(service)* Implement `From<service::UserOrGroup>` for `String`.
- Implement `PartialEq<str>` and `PartialEq<&str>` for key types.
  - `compose_spec::{Identifier, MapKey, ExtensionKey, service::{build::CacheOption, user_or_group::Name, Hostname, Resource}}`
- *(service)* Add `service::volumes::mount::Tmpfs::from_target()`.
  - Also implemented `From<service::volumes::AbsolutePath>` for `service::volumes::mount::Tmpfs` using the new function.
- **BREAKING** *(service)* Add `entitlements` field to `service::Build` ([#15](https://github.com/k9withabone/compose_spec_rs/issues/15)).

### Bug Fixes
- **BREAKING** *(service)* Allow for unlimited `pids_limit`.
  - Generalize `service::MemswapLimit` into `service::Limit<T>`.
  - `Service.memswap_limit` is now an `Option<Limit<ByteValue>>`.
  - `Service.pids_limit` is now an `Option<Limit<u32>>`.
  - `service::deploy::resources::Limits.pids` is now an `Option<Limit<u32>>`.
- **BREAKING** `service::Device` no longer implements `Default` (this was a mistake).
- **BREAKING** *(service)* Container paths must be absolute.
  - `Service.tmpfs` is now an `Option<ItemOrList<AbsolutePath>>`.
  - `Service.working_dir` is now an `Option<AbsolutePath>`.
  - `service::blkio_config::BpsLimit.path` is now an `AbsolutePath`.
  - `service::blkio_config::IopsLimit.path` is now an `AbsolutePath`.
  - `service::blkio_config::WeightDevice.path` is now an `AbsolutePath`.
  - `service::develop::WatchRule.target` is now an `Option<AbsolutePath>`.
  - `service::Device.container_path` is now an `AbsolutePath`.
  - Add `service::device::ParseDeviceError::ContainerPathAbsolute` variant.
- *(service)* `ShortVolume::into_long()` set `create_host_path: true`.
  - When converting a `service::volumes::ShortVolume` into a `service::volumes::mount::Bind`, the `create_host_path` field in `service::volumes::mount::BindOptions` should be set to `true`.

### Documentation
- Fix `ListOrMap::into_list()` docs.
  - Last line was a normal comment instead of a doc comment.
- *(changelog)* Add [git-cliff](https://github.com/orhun/git-cliff) configuration

### Tests
- *(service)* Fix `service::ports::ShortRanges` generation.
  - The `offset` range could become `0..0` which caused `proptest` to panic.

### Miscellaneous
- *(ci)* Add semver-checks job.
  - Use [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) to make sure the package version is increased correctly when making changes.
- *(deps)* Update dependencies.
- *(ci)* Bump `typos` to v1.20.9.

## [0.1.0] - 2024-04-05

The initial release of `compose_spec`!

### Features

- (De)serialize from/to the structure of the Compose specification.
- Values are fully validated and parsed.
- Completely documented.
- Conversion between short and long syntax forms of values.
- Conversion between `std::time::Duration` and the duration string format from the compose-spec.

[0.2.0]: https://github.com/k9withabone/compose_spec_rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/k9withabone/compose_spec_rs/compare/51a31d82c34c13cf8881bf8a9cbda74a6b6aa9b6...v0.1.0
