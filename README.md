# compose_spec

[![Crates.io Version](https://img.shields.io/crates/v/compose_spec?style=flat-square&logo=rust)](https://crates.io/crates/compose_spec)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/compose_spec?style=flat-square&logo=rust)](#minimum-supported-rust-version-msrv)
[![docs.rs](https://img.shields.io/docsrs/compose_spec?style=flat-square&logo=rust)](https://docs.rs/compose_spec)
[![License](https://img.shields.io/crates/l/compose_spec?style=flat-square)](./LICENSE)
[![GitHub Actions CI Workflow Status](https://img.shields.io/github/actions/workflow/status/containers/compose_spec_rs/ci.yaml?branch=main&style=flat-square&logo=github&label=ci)](https://github.com/containers/compose_spec_rs/actions/workflows/ci.yaml?query=branch%3Amain)

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

The minimum version of the Rust compiler `compose_spec` can currently compile with is 1.85, which is tested in CI.
The goal is to match the Rust version used by [Debian stable](https://packages.debian.org/stable/rustc).
However, this is not a hard requirement and the MSRV may be increased as necessary.
Increasing the MSRV is **not** considered to be a breaking change.

## Contribution

Contributions, suggestions, and/or comments are appreciated! See the
[contribution guide](./CONTRIBUTING.md) for more information on reporting issues, submitting pull
requests, the project layout, running CI tasks locally, and communication channels.

## License

All source code for `compose_spec` is licensed under the [Mozilla Public License v2.0](https://www.mozilla.org/en-US/MPL/).
View the [LICENSE](./LICENSE) file for more information.

The [Compose specification] itself is licensed under the [Apache License v2.0](https://www.apache.org/licenses/LICENSE-2.0).
See that project's [LICENSE](https://github.com/compose-spec/compose-spec/blob/master/LICENSE) file for more information.

[Compose specification]: https://github.com/compose-spec/compose-spec
[documentation]: https://docs.rs/compose_spec
[Rust]: https://www.rust-lang.org/
