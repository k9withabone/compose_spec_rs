# compose_spec

[![Crates.io Version](https://img.shields.io/crates/v/compose_spec?style=flat-square&logo=rust)](https://crates.io/crates/compose_spec)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/compose_spec?style=flat-square&logo=rust)](#minimum-supported-rust-version-msrv)
[![docs.rs](https://img.shields.io/docsrs/compose_spec?style=flat-square&logo=rust)](https://docs.rs/compose_spec)
[![License](https://img.shields.io/crates/l/compose_spec?style=flat-square)](./LICENSE)
[![GitHub Actions CI Workflow Status](https://img.shields.io/github/actions/workflow/status/k9withabone/compose_spec_rs/ci.yaml?branch=main&style=flat-square&logo=github&label=ci)](https://github.com/k9withabone/compose_spec_rs/actions/workflows/ci.yaml?query=branch%3Amain)

`compose_spec` is a [Rust] library crate for (de)serializing from/to the [Compose specification].

`compose_spec` strives for:

- Idiomatic Rust 🦀
  - Uses semantically appropriate types from the standard library like `PathBuf` and `Duration`.
- Correctness
  - Values are fully validated and parsed.
  - Enums are used for fields which conflict with each other. For example, in `services`, `network_mode` and `networks` are combined into `network_config`.
- Ease of use
  - Fully documented, though the [documentation] could be fleshed out more with examples and explanations, help in this regard would be appreciated!
  - Helpful functions such as conversion between short and long syntax forms of values with multiple representations (e.g. `build` and `ports`).

See the [documentation] for more details.

## Examples

```rust
use compose_spec::{Compose, Service, service::Image};

let yaml = "\
services:
  caddy:
    image: docker.io/library/caddy:latest
    ports:
      - 8000:80
      - 8443:443
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy-data:/data
volumes:
  caddy-data:
";

// Deserialize `Compose`
let compose: Compose = serde_yaml::from_str(yaml)?;

// Serialize `Compose`
let value = serde_yaml::to_value(&compose)?;

// Get the `Image` of the "caddy" service
let caddy: Option<&Service> = compose.services.get("caddy");
let image: &Option<Image> = &caddy.unwrap().image;
let image: &Image = image.as_ref().unwrap();

assert_eq!(image, "docker.io/library/caddy:latest");
assert_eq!(image.name(), "docker.io/library/caddy");
assert_eq!(image.tag(), Some("latest"));
```

## Minimum Supported Rust Version (MSRV)

The minimum version of the Rust compiler `compose_spec` can currently compile with is 1.70, which is tested in CI.
Increasing the MSRV is **not** considered to be a breaking change.

## Contribution

Contributions, suggestions, and/or comments are appreciated! Feel free to create an [issue](https://github.com/k9withabone/compose_spec_rs/issues), [discussion](https://github.com/k9withabone/compose_spec_rs/discussions), or [pull request](https://github.com/k9withabone/compose_spec_rs/pulls).
Generally, it is preferable to start a discussion for a feature request or open an issue for reporting a bug before submitting changes with a pull request.

### Project Layout

`compose_spec` is composed of two packages set up in a Cargo workspace. The root package, `compose_spec`, is the main library.
The other package, `compose_spec_macros`, located in a directory of the same name, is a procedural macro library used in `compose_spec`. `compose_spec_macros` is not designed to be used outside the `compose_spec` library.

### Local CI

If you are submitting code changes in a pull request and would like to run the CI jobs locally, use the following commands:

- format: `cargo fmt --check --all`
- clippy: `cargo clippy --workspace --tests`
- test: `cargo test --workspace -- --include-ignored`
- doc: `cargo doc --workspace --document-private-items`
- docs-rs:
  - Install the nightly Rust toolchain, `rustup toolchain install nightly`.
  - Install [cargo-docs-rs](https://github.com/dtolnay/cargo-docs-rs).
  - `cargo docs-rs`
- spellcheck:
  - Install [typos](https://github.com/crate-ci/typos).
  - `typos`
- msrv:
  - Install [cargo-msrv](https://github.com/foresterre/cargo-msrv).
  - `cargo msrv verify`
- minimal-versions:
  - Install the nightly Rust toolchain, `rustup toolchain install nightly`.
  - Install [cargo-hack](https://github.com/taiki-e/cargo-hack).
  - Install [cargo-minimal-versions](https://github.com/taiki-e/cargo-minimal-versions).
  - `cargo minimal-versions check --workspace`
  - `cargo minimal-versions test --workspace`

## License

All source code for `compose_spec` is licensed under the [Mozilla Public License v2.0](https://www.mozilla.org/en-US/MPL/).
View the [LICENSE](./LICENSE) file for more information.

The [Compose specification] itself is licensed under the [Apache License v2.0](https://www.apache.org/licenses/LICENSE-2.0).
See that project's [LICENSE](https://github.com/compose-spec/compose-spec/blob/master/LICENSE) file for more information.

[Compose specification]: https://github.com/compose-spec/compose-spec
[documentation]: https://docs.rs/compose_spec
[Rust]: https://www.rust-lang.org/
