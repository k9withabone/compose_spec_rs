use compose_spec_macros::{platforms, DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

platforms! {
    #![apply_to_all(
        derive(SerializeDisplay, DeserializeFromStr, Debug, Clone, Copy, PartialEq, Eq, Hash),
    )]

    /// Target platforms.
    ///
    /// (De)serializes from/to a string with the format "{os}[/{arch}[/variant]]".
    ///
    /// See the [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#platform),
    /// [OCI Image Index Specification](https://github.com/opencontainers/image-spec/blob/main/image-index.md),
    /// and [`GOOS`/`GOARCH`](https://go.dev/doc/install/source#environment)
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::platform::{Arch, Arm64Variant, LinuxArch, Os, Platform};
    ///
    /// let platform = Platform::Linux(Some(LinuxArch::Arm64(Some(Arm64Variant::V8))));
    ///
    /// assert_eq!(platform.os(), Os::Linux);
    /// assert_eq!(platform.arch(), Some(Arch::Arm64(Some(Arm64Variant::V8))));
    /// assert_eq!(platform.as_str(), "linux/arm64/v8");
    /// assert_eq!(Platform::Linux(None), "linux".parse().unwrap());
    /// ```
    pub enum Platform;

    /// [`Platform`] operating systems.
    ///
    /// See [`GOOS`](https://go.dev/doc/install/source#environment).
    pub enum Os {
        /// IBM AIX
        Aix => "aix" {
            /// [IBM AIX](Platform::Aix) architectures.
            arch: ["ppc64"],
        },

        /// Android
        Android => "android" {
            /// [`Android`](Platform::Android) architectures.
            arch: ["386", "amd64", "arm", "arm64"],
        },

        /// Apple macOS
        Darwin => "darwin" {
            /// [Apple macOS](Platform::Darwin) architectures.
            arch: ["amd64", "arm64"],
        },

        /// DragonFly BSD
        DragonFly => "dragonfly" {
            /// [`DragonFly` BSD](Platform::DragonFly) architectures.
            arch: ["amd64"],
        },

        /// FreeBSD
        FreeBsd => "freebsd" {
            /// [FreeBSD](Platform::FreeBsd) architectures.
            arch: ["386", "amd64", "arm"],
        },

        /// Illumos
        Illumos => "illumos" {
            /// [`Illumos`](Platform::Illumos) architectures.
            arch: ["amd64"],
        },

        /// Apple iOS
        IOs => "ios" {
            /// [Apple iOS](Platform::IOs) architectures.
            arch: ["arm64"],
        },

        /// JavaScript
        Js => "js" {
            /// [JavaScript](Platform::Js) architectures.
            arch: ["wasm"],
        },

        /// Linux
        Linux => "linux" {
            /// [`Linux`](Platform::Linux) architectures.
            arch: [
                "386",
                "amd64",
                "arm",
                "arm64",
                "loong64",
                "mips",
                "mipsle",
                "mips64",
                "mips64le",
                "ppc64",
                "ppc64le",
                "riscv64",
                "s390x",
            ],
        },

        /// NetBSD
        NetBsd => "netbsd" {
            /// [NetBSD](Platform::NetBsd) architectures.
            arch: ["386", "amd64", "arm"],
        },

        /// OpenBSD
        OpenBsd => "openbsd" {
            /// [OpenBSD](Platform::OpenBsd) architectures.
            arch: ["386", "amd64", "arm", "arm64"],
        },

        /// Plan 9 from Bell Labs
        Plan9 => "plan9" {
            /// [Plan 9](Platform::Plan9) architectures.
            arch: ["386", "amd64", "arm"],
        },

        /// Oracle Solaris
        Solaris => "solaris" {
            /// [Oracle Solaris](Platform::Solaris) architectures.
            arch: ["amd64"],
        },

        /// WebAssembly System Interface (WASI) Preview 1
        WasiP1 => "wasip1" {
            /// [WASI](Platform::WasiP1) architectures.
            arch: ["wasm"],
        },

        /// Microsoft Windows
        Windows => "windows" {
            /// [Microsoft Windows](Platform::Windows) architectures.
            arch: ["386", "amd64", "arm", "arm64"],
        },
    }

    /// [`Platform`] architectures.
    ///
    /// [`GOARCH`](https://go.dev/doc/install/source#environment)
    pub enum Arch {
        /// x86 / 32-bit x86
        _386 => "386",

        /// x86_64 / 64-bit x86
        Amd64 => "amd64",

        /// 32-bit ARM
        Arm => "arm" {
            /// [`Arm`](Arch::Arm) Variants
            ///
            /// [OCI Image Index Specification](https://github.com/opencontainers/image-spec/blob/main/image-index.md#platform-variants)
            variants: [
                /// ARMv6
                V6 => "v6",
                /// ARMv7
                V7 => "v7",
                /// ARMv8
                V8 => "v8",
            ],
        },

        /// AArch64 / 64-bit ARM
        Arm64 => "arm64" {
            /// [`Arm64`](Arch::Arm64) Variants
            ///
            /// [OCI Image Index Specification](https://github.com/opencontainers/image-spec/blob/main/image-index.md#platform-variants)
            variants: [
                /// ARMv8
                V8 => "v8",
            ],
        },

        /// 64-bit LoongArch
        Loong64 => "loong64",

        /// MIPS 32-bit, big-endian
        Mips => "mips",

        /// MIPS 32-bit, little-endian
        MipsLe => "mipsle",

        /// MIPS 64-bit, big-endian
        Mips64 => "mips64",

        /// MIPS 64-bit, little-endian
        Mips64Le => "mips64le",

        /// PowerPC 64-bit, big-endian
        Ppc64 => "ppc64",

        /// PowerPC 64-bit, little-endian
        Ppc64Le => "ppc64le",

        /// RISC-V 64-bit
        RiscV64 => "riscv64",

        /// IBM System z 64-bit, big-endian
        S390x => "s390x",

        /// WebAssembly 32-bit
        Wasm => "wasm",
    }

    type ParseError = ParseError;
    type TryFromArchError = InvalidArchError;
}

/// Error returned when parsing a [`Platform`], [`Os`], [`Arch`], or arch variant from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("unknown/invalid platform or platform part `{0}`")]
pub struct ParseError(String);

/// Error returned when converting [`Arch`] to an OS arch e.g. [`LinuxArch`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("`{arch}` is not a valid arch for `{os}`")]
pub struct InvalidArchError {
    /// Architecture attempted to convert from.
    pub arch: Arch,
    /// Operating system attempted to convert to arch of.
    pub os: Os,
}
