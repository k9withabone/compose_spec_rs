//! Procedural macros for use within the [`compose_spec`] crate.
//!
//! - [`AsShort`] - derive `compose_spec::AsShort`.
//! - [`FromShort`] - derive [`From<Short>`], designed for use in combination with [`AsShort`].
//! - [`SerializeDisplay`] - derive [`Serialize`] using the type's [`Display`] implementation.
//! - [`DeserializeFromStr`] - derive [`Deserialize`] using the type's [`FromStr`] implementation.
//! - [`DeserializeTryFromString`] - derive [`Deserialize`] using the type's [`TryFrom<String>`]
//!   implementation.
//! - [`platforms!`] - define `Platform`, `Os`, `Arch`, and `{Os}Arch` enums and implementations.
//!
//! # Warning
//!
//! These macros are not designed to be used outside of the [`compose_spec`] crate.
//!
//! [`compose_spec`]: https://docs.rs/compose_spec
//! [`Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
//! [`Display`]: std::fmt::Display
//! [`FromStr`]: std::str::FromStr
//! [`Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html

use proc_macro::TokenStream;
use syn::{parse_macro_input, Error};

mod as_short;
mod default;
mod platforms;
mod serde;

/// Derive macro for `AsShort`.
///
/// Has a `as_short` helper attribute which may be set on fields in the forms:
///
/// - `#[as_short(short)]`
/// - `#[as_short([default = {default_fn},][if_fn = {if_fn}])]`
///
/// `short` must be set on a single field. That field is used as the return value of
/// `AsShort::as_short()`.
///
/// `default_fn` and `if_fn` are either expressions or string literals representing expressions
/// which evaluate to functions.
///
/// `default_fn` is used in [`FromShort`](from_short()).
///
/// `if_fn` must evaluate to a function implementing `FnOnce(&<Type>) -> bool` where `Type` is the
/// type of the field the attribute is set on. It is used to determine whether only the `short`
/// field is set and therefore the struct can be represented in short form.
///
/// If `if_fn` is not given, it will default to:
///
/// - [`Option::is_none`] for [`Option`]s
/// - [`Not::not`](std::ops::Not::not()) for [`bool`]s
/// - `<Type>::is_empty` otherwise
///
/// # Warning
///
/// This macro is not designed to be used outside of the [`compose_spec`] crate.
///
/// # Examples
///
/// ```
/// # fn main() {}
/// # trait AsShort {
/// #     type Short;
/// #     fn as_short(&self) -> Option<&Self::Short>;
/// # }
/// # mod long {
/// use std::time::Duration;
///
/// use compose_spec_macros::AsShort;
///
/// #[derive(AsShort)]
/// struct Long {
///     #[as_short(short)]
///     short: u8,
///     #[as_short(if_fn = Duration::is_zero)]
///     duration: Duration,
///     option: Option<String>,
///     bool: bool,
///     vec: Vec<String>,
/// }
/// # }
/// ```
///
/// Which will generate code like:
///
/// ```
/// # fn main() {}
/// # trait AsShort {
/// #     type Short;
/// #     fn as_short(&self) -> Option<&Self::Short>;
/// # }
/// # mod long {
/// # use std::time::Duration;
/// # struct Long {
/// #     short: u8,
/// #     duration: Duration,
/// #     option: Option<String>,
/// #     bool: bool,
/// #     vec: Vec<String>,
/// # }
/// impl crate::AsShort for Long {
///     type Short = u8;
///
///     fn as_short(&self) -> Option<&Self::Short> {
///         if Duration::is_zero(&self.duration)
///             && Option::is_none(&self.option)
///             && !self.bool
///             && Vec::is_empty(&self.vec)
///         {
///             Some(&self.short)
///         } else {
///             None
///         }
///     }
/// }
/// # }
/// ```
///
/// Notice the `crate` path prefix to `AsShort`. This macro is not designed to be used outside of
/// the [`compose_spec`] crate.
///
/// [`compose_spec`]: https://docs.rs/compose_spec
#[proc_macro_derive(AsShort, attributes(as_short))]
pub fn as_short(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    as_short::Input::from_syn(&input)
        .map_or_else(Error::into_compile_error, |input| input.impl_as_short())
        .into()
}

/// Derive macro for [`From<Short>`], for use in combination with deriving [`AsShort`](as_short()).
///
/// Has a `as_short` helper attribute which may be set on fields in the forms:
///
/// - `#[as_short(short)]`
/// - `#[as_short([default = {default_fn},][if_fn = {if_fn}])]`
///
/// `short` must be set on a single field. That field is used as the [`From`] type.
///
/// `default_fn` and `if_fn` are either expressions or string literals representing expressions
/// which evaluate to functions.
///
/// `default_fn` is used to create a default value for the field. If not provided, the [`Default`]
/// implementation is used.
///
/// `if_fn` is used in deriving [`AsShort`](as_short()).
///
/// # Examples
///
/// ```
/// use compose_spec_macros::FromShort;
///
/// #[derive(FromShort)]
/// struct Long {
///     #[as_short(short)]
///     short: u8,
///     #[as_short(default = hello_default)]
///     hello: String,
///     option: Option<String>,
///     bool: bool,
///     vec: Vec<String>,
/// }
///
/// fn hello_default() -> String {
///     String::from("Hello!")
/// }
/// ```
///
/// Which will generate code like:
///
/// ```
/// # struct Long {
/// #     short: u8,
/// #     hello: String,
/// #     option: Option<String>,
/// #     bool: bool,
/// #     vec: Vec<String>,
/// # }
/// # fn hello_default() -> String { String::from("Hello!") }
/// impl From<u8> for Long {
///     fn from(short: u8) -> Self {
///         Self {
///             short,
///             hello: hello_default(),
///             option: Default::default(),
///             bool: Default::default(),
///             vec: Default::default(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(FromShort, attributes(as_short))]
pub fn from_short(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    as_short::Input::from_syn(&input)
        .map_or_else(Error::into_compile_error, |input| input.impl_from_short())
        .into()
}

/// Derive macro for [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) which
/// uses the type's [`Display`](std::fmt::Display) implementation to serialize it as a string.
///
/// # Examples
///
/// ```
/// use std::fmt::{self, Display, Formatter};
///
/// use compose_spec_macros::SerializeDisplay;
///
/// #[derive(SerializeDisplay)]
/// struct Example {
///     str: &'static str,
///     bool: bool,
/// }
///
/// impl Display for Example {
///     fn fmt(&self, f: &mut Formatter) -> fmt::Result {
///         write!(f, "{} is {}", &self.str, &self.bool)
///     }
/// }
///
/// let example = Example {
///     str: "hello",
///     bool: true,
/// };
/// assert_eq!(serde_yaml::to_string(&example).unwrap(), "hello is true\n");
/// ```
#[proc_macro_derive(SerializeDisplay)]
pub fn serialize_display(input: TokenStream) -> TokenStream {
    serde::Input::from_syn(parse_macro_input!(input))
        .map_or_else(
            Error::into_compile_error,
            serde::Input::impl_serialize_display,
        )
        .into()
}

/// Derive macro for [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html)
/// which uses the type's [`FromStr`](std::str::FromStr) implementation to deserialize it from a
/// string.
///
/// Optionally accepts a `#[serde(expecting = "...")]` attribute to give to the visitor.
///
/// # Warning
///
/// This macro is not designed to be used outside of the [`compose_spec`] crate.
///
/// # Examples
///
/// ```
/// # mod serde {
/// #     use ::serde::Deserialize;
/// #     pub struct FromStrVisitor;
/// #     impl FromStrVisitor {
/// #         pub fn new(_: &str) -> Self { Self }
/// #         pub fn deserialize<'de, T, D>(self, deserializer: D) -> Result<T, D::Error>
/// #         where
/// #             T: std::str::FromStr,
/// #             T::Err: std::fmt::Display,
/// #             D: ::serde::Deserializer<'de>,
/// #         {
/// #             let string = String::deserialize(deserializer)?;
/// #             string.parse().map_err(serde::de::Error::custom)
/// #         }
/// #     }
/// # }
/// # fn main() {
/// use std::str::FromStr;
///
/// use compose_spec_macros::DeserializeFromStr;
///
/// #[derive(DeserializeFromStr)]
/// #[serde(expecting = "an example string")]
/// struct Example {
///     inner: String,
/// }
///
/// impl FromStr for Example {
///     type Err = std::convert::Infallible;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         Ok(Self { inner: s.to_owned() })
///     }
/// }
///
/// let example: Example = serde_yaml::from_str("hello").unwrap();
/// assert_eq!(example.inner, "hello");
/// # }
/// ```
///
/// The macro generates code like:
///
/// ```
/// # mod serde {
/// #     use ::serde::Deserialize;
/// #     pub struct FromStrVisitor;
/// #     impl FromStrVisitor {
/// #         pub fn new(_: &str) -> Self { Self }
/// #         pub fn deserialize<'de, T, D>(self, deserializer: D) -> Result<T, D::Error>
/// #         where
/// #             D: ::serde::Deserializer<'de>,
/// #         { unimplemented!() }
/// #     }
/// # }
/// # fn main() {}
/// # struct Example { inner: String }
/// impl<'de> ::serde::Deserialize<'de> for Example {
///     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
///     where
///         D: ::serde::Deserializer<'de>,
///     {
///         crate::serde::FromStrVisitor::new("an example string").deserialize(deserializer)
///     }
/// }
/// ```
///
/// Note the use of `crate::serde::FromStrVisitor`. This macro is not designed to be used outside of
/// the [`compose_spec`] crate.
///
/// [`compose_spec`]: https://docs.rs/compose_spec
#[proc_macro_derive(DeserializeFromStr, attributes(serde))]
pub fn deserialize_from_str(input: TokenStream) -> TokenStream {
    serde::Input::from_syn(parse_macro_input!(input))
        .map_or_else(
            Error::into_compile_error,
            serde::Input::impl_deserialize_from_str,
        )
        .into()
}

/// Derive macro for [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html)
/// which uses the type's [`TryFrom<String>`] implementation to deserialize it from a string.
///
/// Optionally accepts a `#[serde(expecting = "...")]` attribute to give to the visitor.
///
/// # Warning
///
/// This macro is not designed to be used outside of the [`compose_spec`] crate.
///
/// # Examples
///
/// ```
/// # mod serde {
/// #     use ::serde::Deserialize;
/// #     pub struct TryFromStringVisitor;
/// #     impl TryFromStringVisitor {
/// #         pub fn new(_: &str) -> Self { Self }
/// #         pub fn deserialize<'de, T, D>(self, deserializer: D) -> Result<T, D::Error>
/// #         where
/// #             String: TryInto<T>,
/// #             <String as TryInto<T>>::Error: std::fmt::Display,
/// #             D: ::serde::Deserializer<'de>,
/// #         {
/// #             let string = String::deserialize(deserializer)?;
/// #             string.try_into().map_err(serde::de::Error::custom)
/// #         }
/// #     }
/// # }
/// # fn main() {
/// use compose_spec_macros::DeserializeTryFromString;
///
/// #[derive(DeserializeTryFromString)]
/// #[serde(expecting = "an example string")]
/// struct Example {
///     inner: String,
/// }
///
/// impl TryFrom<String> for Example {
///     type Error = std::convert::Infallible;
///
///     fn try_from(value: String) -> Result<Self, Self::Error> {
///         Ok(Self { inner: value })
///     }
/// }
///
/// let example: Example = serde_yaml::from_str("hello").unwrap();
/// assert_eq!(example.inner, "hello");
/// # }
/// ```
///
/// The macro generates code like:
///
/// ```
/// # mod serde {
/// #     use ::serde::Deserialize;
/// #     pub struct TryFromStringVisitor;
/// #     impl TryFromStringVisitor {
/// #         pub fn new(_: &str) -> Self { Self }
/// #         pub fn deserialize<'de, T, D>(self, deserializer: D) -> Result<T, D::Error>
/// #         where
/// #             D: ::serde::Deserializer<'de>,
/// #         { unimplemented!() }
/// #     }
/// # }
/// # fn main() {}
/// # struct Example { inner: String }
/// impl<'de> ::serde::Deserialize<'de> for Example {
///     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
///     where
///         D: ::serde::Deserializer<'de>,
///     {
///         crate::serde::TryFromStringVisitor::new("an example string").deserialize(deserializer)
///     }
/// }
/// ```
///
/// Note the use of `crate::serde::TryFromStringVisitor`. This macro is not designed to be used outside of
/// the [`compose_spec`] crate.
///
/// [`compose_spec`]: https://docs.rs/compose_spec
#[proc_macro_derive(DeserializeTryFromString, attributes(serde))]
pub fn deserialize_try_from_string(input: TokenStream) -> TokenStream {
    serde::Input::from_syn(parse_macro_input!(input))
        .map_or_else(
            Error::into_compile_error,
            serde::Input::impl_deserialize_try_from_string,
        )
        .into()
}

/// Derive macro for [`Default`] which allows setting custom values via `#[default = ...]`
/// attributes.
///
/// # Examples
///
/// ```
/// use compose_spec_macros::Default;
///
/// #[derive(Default)]
/// struct Example {
///     #[default = true]
///     bool: bool,
///     string: String,
/// }
///
/// let default = Example::default();
/// assert_eq!(default.bool, true);
/// assert_eq!(default.string, "");
/// ```
///
/// The macro generates code like:
///
/// ```
/// # struct Example { bool: bool, string: String }
/// impl Default for Example {
///     fn default() -> Self {
///         Self {
///             bool: true,
///             string: Default::default(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(Default, attributes(default))]
pub fn default(input: TokenStream) -> TokenStream {
    default::Input::from_syn(&parse_macro_input!(input))
        .map_or_else(Error::into_compile_error, default::Input::expand)
        .into()
}

/// Macro which defines `Platform`, `Os`, `Arch` and `{Os}Arch` enums and implementations.
///
/// Used for defining the type of the `platform` field in
/// [`compose_spec::Service`](https://docs.rs/compose_spec/*/compose_spec/service/struct.Service.html).
///
/// The format and the generated code are shown in the example below.
///
/// # Examples
///
/// ```
/// compose_spec_macros::platforms! {
///     // Attributes applied to all generated enums.
///     #![apply_to_all(derive(Debug, Clone))]
///
///     /// `Platform` doc comments or other attributes
///     pub enum Platform;
///
///     /// `Os` doc comments
///     pub enum Os {
///         /// `Platform::Linux` and `Os::Linux` doc comments
///         // The string literal below is what this OS is mapped from/to when converting from/to a string,
///         // `Platform::as_str()` returns strings in format "{os}/{arch}[/{variant}]".
///         // The string literal must also match one of the architectures defined in `Arch` below.
///         Linux => "linux" {
///             /// `LinuxArch` doc comments
///             // String literals set here must match one of the architectures defined in `Arch` below.
///             arch: ["amd64"],
///         },
///         /// `Platform::Darwin` and `Os::Darwin` doc comments
///         Darwin => "darwin" {
///             /// `DarwinArch` doc comments
///             arch: ["arm64"],
///         },
///     }
///
///     /// `Arch` doc comments
///     pub enum Arch {
///         /// `Arch::Amd64` and `LinuxArch::Amd64` doc comments
///         Amd64 => "amd64",
///         /// `Arch::Arm64` and `DarwinArch::Arm64` doc comments
///         Arm64 => "arm64" {
///             /// `Arm64Variant` doc comments
///             variants: [
///                 /// `Arm64Variant::V8` doc comments
///                 V8 => "v8",
///             ],
///         },
///     }
///
///     // The error type used for `FromStr` and `TryFrom<&str>` implementations.
///     // The type must be a newtype of a `String`.
///     type ParseError = ParseError;
///
///     // The error type used for `impl TryFrom<{Os}Arch> for Arch`.
///     // The type must be a struct with `arch: Arch` and `os: Os` fields.
///     type TryFromArchError = InvalidArchError;
/// }
///
/// pub struct ParseError(String);
///
/// pub struct InvalidArchError {
///     arch: Arch,
///     os: Os,
/// }
/// ```
///
/// The above macro invocation will approximately generate the following code:
///
/// ```
/// # use std::{str::FromStr, fmt::{self, Display, Formatter}};
/// # pub struct ParseError(String);
/// # pub struct InvalidArchError { arch: Arch, os: Os }
/// /// `Platform` doc comments or other attributes
/// #[derive(Debug, Clone)]
/// pub enum Platform {
///     /// `Platform::Linux` and `Os::Linux` doc comments
///     Linux(Option<LinuxArch>),
///     /// `Platform::Darwin` and `Os::Darwin` doc comments
///     Darwin(Option<DarwinArch>),
/// }
///
/// impl Platform {
///     pub const fn as_str(&self) -> &'static str {
///         match self {
///             Self::Linux(None) => "linux",
///             Self::Linux(Some(LinuxArch::Amd64)) => "linux/amd64",
///             Self::Darwin(None) => "darwin",
///             Self::Darwin(Some(DarwinArch::Arm64(None))) => "darwin/arm64",
///             Self::Darwin(Some(DarwinArch::Arm64(Some(Arm64Variant::V8)))) => "darwin/arm64/v8",
///         }
///     }
///
///     pub const fn os(&self) -> Os {
///         match self {
///             Self::Linux(_) => Os::Linux,
///             Self::Darwin(_) => Os::Darwin,
///         }
///     }
///
///     pub fn arch(&self) -> Option<Arch> {
///         match self {
///             Self::Linux(arch) => arch.clone().map(Into::into),
///             Self::Darwin(arch) => arch.clone().map(Into::into),
///         }
///     }
/// }
///
/// impl AsRef<str> for Platform {
///     fn as_ref(&self) -> &str {
///         self.as_str()
///     }
/// }
///
/// impl Display for Platform {
///     fn fmt(&self, f: &mut Formatter) -> fmt::Result {
///         f.write_str(self.as_str())
///     }
/// }
///
/// impl FromStr for Platform {
///     type Err = ParseError;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s {
///             "linux" => Ok(Self::Linux(None)),
///             "linux/amd64" => Ok(Self::Linux(Some(LinuxArch::Amd64))),
///             "darwin" => Ok(Self::Darwin(None)),
///             "darwin/arm64" => Ok(Self::Darwin(Some(DarwinArch::Arm64(None)))),
///             "darwin/arm64/v8" => Ok(Self::Darwin(Some(DarwinArch::Arm64(Some(Arm64Variant::V8))))),
///             s => Err(ParseError(s.to_owned())),
///         }
///     }
/// }
///
/// impl TryFrom<&str> for Platform {
///     type Error = ParseError;
///
///     fn try_from(value: &str) -> Result<Self, Self::Error> {
///         value.parse()
///     }
/// }
///
/// impl From<Os> for Platform {
///     fn from(value: Os) -> Self {
///         match value {
///             Os::Linux => Self::Linux(None),
///             Os::Darwin => Self::Darwin(None),
///         }
///     }
/// }
///
/// /// `LinuxArch` doc comments
/// #[derive(Debug, Clone)]
/// pub enum LinuxArch {
///     /// `Arch::Amd64` and `LinuxArch::Amd64` doc comments
///     Amd64,
/// }
///
/// /* `LinuxArch` will have implementations of `as_str()`, `AsRef<str>`, `Display`, `FromStr`, and
///    `TryFrom<&str>` similar to `Platform` */
///
/// impl LinuxArch {
///     pub const OS: Os = Os::Linux;
/// }
///
/// impl TryFrom<Arch> for LinuxArch {
///     type Error = InvalidArchError;
///
///     fn try_from(value: Arch) -> Result<Self, Self::Error> {
///         match value {
///             Arch::Amd64 => Ok(Self::Amd64),
///             arch => Err(InvalidArchError { arch, os: Self::OS }),
///         }
///     }
/// }
///
/// impl From<LinuxArch> for Arch {
///     fn from(value: LinuxArch) -> Self {
///         match value {
///             LinuxArch::Amd64 => Self::Amd64,
///         }
///     }
/// }
///
/// /// `DarwinArch` doc comments
/// #[derive(Debug, Clone)]
/// pub enum DarwinArch {
///     /// `Arch::Arm64` and `DarwinArch::Arm64` doc comments
///     Arm64(Option<Arm64Variant>),
/// }
///
/// /* `DarwinArch` will have similar implementations to `LinuxArch` */
///
/// impl From<DarwinArch> for Arch {
///     fn from(value: DarwinArch) -> Self {
///         match value {
///             DarwinArch::Arm64(variant) => Self::Arm64(variant),
///         }
///     }
/// }
///
/// /// `Os` doc comments
/// #[derive(Debug, Clone)]
/// pub enum Os {
///     /// `Platform::Linux` and `Os::Linux` doc comments
///     Linux,
///     /// `Platform::Darwin` and `Os::Darwin` doc comments
///     Darwin,
/// }
///
/// /* `Os` will have implementations of `as_str()`, `AsRef<str>`, `Display`, `FromStr`, and
///    `TryFrom<&str>` similar to `Platform` */
///
/// /// `Arch` doc comments
/// #[derive(Debug, Clone)]
/// pub enum Arch {
///     /// `Arch::Amd64` and `LinuxArch::Amd64` doc comments
///     Amd64,
///     /// `Arch::Arm64` and `LinuxArch::Arm64` doc comments
///     Arm64(Option<Arm64Variant>),
/// }
///
/// /* `Arch` will have implementations of `as_str()`, `AsRef<str>`, `Display`, `FromStr`, and
///    `TryFrom<&str>` similar to `Platform` */
///
/// /// `Arm64Variant` doc comments
/// #[derive(Debug, Clone)]
/// pub enum Arm64Variant {
///     /// `Arm64Variant::V8` doc comments
///     V8,
/// }
///
/// /* `Arm64Variant` will have implementations of `as_str()`, `AsRef<str>`, `Display`, `FromStr`, and
///    `TryFrom<&str>` similar to `Platform` */
/// ```
#[proc_macro]
pub fn platforms(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as platforms::Input)
        .expand()
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[cfg(test)]
mod tests {
    // This is to satisfy the `unused_crate_dependencies` lint. `serde` and `serde_yaml` are used in
    // the examples for the proc macros above but nowhere else.
    use serde as _;
    use serde_yaml as _;
}
