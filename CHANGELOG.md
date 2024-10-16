# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2024-10-16

### New Features
- Add `Compose::validate_networks()`.
  - Ensures that all networks used in each service is defined in the top-level `networks` field of the `Compose` file.
- Add `Compose::validate_volumes()`.
  - Ensures that named volumes used across multiple services are defined in the top-level `volumes` field of the `Compose` file.
- Add `Compose::validate_configs()`.
  - Ensures that all configs used in each service is defined in the top-level `configs` field of the `Compose` file.
- Add `Compose::validate_secrets()`.
  - Ensures that all secrets used in each service is defined in the top-level `secrets` field of the `Compose` file.
- Add `Compose::validate_all()`.
- Add `Compose::options()` for setting deserialization options.
- Add `Options::apply_merge()`. ([#2](https://github.com/k9withabone/compose_spec_rs/issues/2))
  - Merges `<<` keys into the surrounding mapping.
- **BREAKING** *(service)* Add `services[].networks[].driver_opts` attribute. ([#29](https://github.com/k9withabone/compose_spec_rs/issues/29))
  - Added the `driver_opts` field to `compose_spec::service::network_config::Network`.
- *(service)* Add `Limit` string conversions.
  - Added `Display` and `FromStr` implementations to `compose_spec::service::Limit`.
  - Changed the string deserialization logic of `Limit` to deserialize "-1" as `Limit::Unlimited`.
- **BREAKING** *(service)* Support `network_mode: container:{name}`.
  - Added the `compose_spec::service::network_config::NetworkMode::Container` enum variant.

### Bug Fixes
- *(service)* Image registry with port is valid. ([#22](https://github.com/k9withabone/compose_spec_rs/issues/22))
  - Image names with a registry that have a port are now valid.
  - Changed `compose_spec::service::image::Name::new()` to allow for using `compose_spec::service::Image::set_registry()` with a registry with a port. The first part of a name is now treated as a registry if it has a dot (.) regardless of whether the name has multiple parts.
  - Added `compose_spec::service::image::InvalidNamePart::RegistryPort` error variant for when a registry's port is not a valid port number.
  - Refactored image tests to not use `unwrap()`.
- *(service)* Support host IP in brackets for `ports` short syntax. ([#24](https://github.com/k9withabone/compose_spec_rs/issues/24))
- **BREAKING** *(service)* `user` may have an optional group. ([#23](https://github.com/k9withabone/compose_spec_rs/issues/23))
  - Before this fix values for `services[].user` that included a GID or group name were rejected. The Compose Specification is unfortunately vague for `user` (see https://github.com/compose-spec/compose-spec/issues/39). However, both `docker run --user` and `podman run --user` accept the `{user}[:{group}]` syntax.
  - Renamed `compose_spec::service::UserOrGroup` to `IdOrName`.
  - Renamed `compose_spec::service::user_or_group` module to `user`.
  - Added `compose_spec::service::User`.
  - Changed the type of the `user` field in `compose_spec::Service` to `Option<User>`.
- **BREAKING** *(service)* Support unlimited ulimits. ([#31](https://github.com/k9withabone/compose_spec_rs/issues/31))
  - Changed `soft` and `hard` fields of `compose_spec::service::Ulimit` to `compose_spec::service::Limit<u64>`.
  - Changed `compose_spec::service::Ulimits` type alias (used for `ulimits` field of `compose_spec::Service` and `compose_spec::service::Build`) to `IndexMap<Resource, ShortOrLong<Limit<u64>, Ulimit>>`.
  - Changed `<Ulimit as AsShort>::Short` to `Limit<u64>`.
  - Added `impl From<Limit<u64>> for Ulimit`.
  - Added `impl From<u64> for Limit<u64>`.
  - Added `impl<T, L> From<Limit<T>> for ShortOrLong<Limit<T>, L>` and `impl<L> From<u64> for ShortOrLong<Limit<u64>, L>`.

### Documentation
- Add fragment documentation.
- *(macros)* Add symlink to `LICENSE` file. ([#21](https://github.com/k9withabone/compose_spec_rs/issues/21))
  - This ensures that the `LICENSE` file is included when the `compose_spec_macros` package is published to crates.io via `cargo publish`.
- *(changelog)* Update git-cliff config for v2.6.0.

### Miscellaneous
- *(lints)* Allow bare URL in `compose_spec::service::build::Context::Url` docs.
- *(lints)* Decrease priority of lint groups.
- *(deps)* Update lock file.
- *(ci)* Bump `typos` to v1.26.0.

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

[0.3.0]: https://github.com/k9withabone/compose_spec_rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/k9withabone/compose_spec_rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/k9withabone/compose_spec_rs/compare/51a31d82c34c13cf8881bf8a9cbda74a6b6aa9b6...v0.1.0
