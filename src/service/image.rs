//! Provides [`Image`] for the `image` field of [`Service`](super::Service) and `tags` field of
//! [`Build`](super::Build).

mod digest;
mod name;
mod tag;

use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{AddAssign, SubAssign},
};

use compose_spec_macros::{DeserializeTryFromString, SerializeDisplay};
use thiserror::Error;

use crate::impl_from_str;

pub use self::{
    digest::{Digest, InvalidDigestError},
    name::{InvalidNamePartError, Name},
    tag::{InvalidTagError, Tag},
};

/// Container image specification.
///
/// Images contain a name and an optional tag or digest. Each part of the image specification must
/// conform to a specific format. See [`Name`], [`Tag`], and [`Digest`] for details. The general
/// format is `{name}[:{tag}|@{digest}]`.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#image)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone)]
pub struct Image {
    /// Inner string.
    inner: String,

    /// Byte position of `inner` where the registry ends, if the image has a registry part.
    registry_end: Option<usize>,

    /// Byte position of `inner` where the tag or digest starts, after its separator (: or @),
    /// if the image has a tag or digest.
    tag_or_digest_start: Option<TagOrDigestStart>,
}

impl Image {
    /// Parse an [`Image`] from a string.
    ///
    /// # Errors
    ///
    /// Images are made up of a [`Name`] and an optional [`Tag`] or [`Digest`]. Each part has
    /// specific requirements for that part of the string to conform to. See [`Name::new()`],
    /// [`Tag::new()`], and [`Digest::new()`] for details.
    ///
    /// This function will also error if the string contains both a tag and digest.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, InvalidImageError};
    ///
    /// let image = Image::parse("quay.io/podman/hello:latest").unwrap();
    ///
    /// assert_eq!(image, "quay.io/podman/hello:latest");
    /// assert_eq!(image.registry(), Some("quay.io"));
    /// assert_eq!(image.name(), "quay.io/podman/hello");
    /// assert_eq!(image.tag(), Some("latest"));
    /// assert_eq!(image.digest(), None);
    ///
    /// // Images cannot have a tag and a digest.
    /// let image = "quay.io/podman/hello:latest@sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// assert_eq!(Image::parse(image), Err(InvalidImageError::TagAndDigest));
    /// ```
    pub fn parse<T>(image: T) -> Result<Self, InvalidImageError>
    where
        T: AsRef<str> + Into<String>,
    {
        let (registry_end, tag_or_digest_start) = Self::parse_impl(image.as_ref())?;

        Ok(Self {
            inner: image.into(),
            registry_end,
            tag_or_digest_start,
        })
    }

    /// Concrete implementation for [`Self::parse()`].
    fn parse_impl(
        image: &str,
    ) -> Result<(Option<usize>, Option<TagOrDigestStart>), InvalidImageError> {
        let (image, digest_start) = if let Some((image, digest)) = image.split_once('@') {
            Digest::new(digest)?;
            (image, Some(image.len() + 1))
        } else {
            (image, None)
        };

        let (image, tag_start) = if let Some((image, tag)) = image.split_once(':') {
            Tag::new(tag)?;
            (image, Some(image.len() + 1))
        } else {
            (image, None)
        };

        let tag_or_digest = match (digest_start, tag_start) {
            (None, None) => None,
            (None, Some(tag_start)) => Some(TagOrDigestStart::Tag(tag_start)),
            (Some(digest_start), None) => Some(TagOrDigestStart::Digest(digest_start)),
            (Some(_), Some(_)) => return Err(InvalidImageError::TagAndDigest),
        };

        let name = Name::new(image)?;

        Ok((name.registry_end(), tag_or_digest))
    }

    /// Create an [`Image`] from validated parts.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Name, Tag};
    ///
    /// let name = Name::new("quay.io/podman/hello").unwrap();
    /// let tag = Tag::new("latest").unwrap();
    ///
    /// let image = Image::from_parts(name, Some(tag.into()));
    ///
    /// assert_eq!(image, "quay.io/podman/hello:latest");
    /// ```
    pub fn from_parts(name: Name, tag_or_digest: Option<TagOrDigest>) -> Self {
        let registry_end = name.registry_end();
        let name = name.into_inner();

        let tag_or_digest_len = tag_or_digest
            .as_ref()
            .map(TagOrDigest::len)
            .unwrap_or_default();
        let mut inner = String::with_capacity(name.len() + tag_or_digest_len);

        inner.push_str(name);

        let tag_or_digest_start = tag_or_digest.map(|tag_or_digest| {
            tag_or_digest.push_to_string(&mut inner);
            tag_or_digest.as_start(inner.len())
        });

        Self {
            inner,
            registry_end,
            tag_or_digest_start,
        }
    }

    /// String slice of the entire image.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Registry portion of the image.
    ///
    /// Returns [`None`] if the image name does not have a registry component.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::Image;
    ///
    /// let image = Image::parse("quay.io/podman/hello").unwrap();
    /// assert_eq!(image.registry(), Some("quay.io"));
    ///
    /// let image = Image::parse("library/busybox").unwrap();
    /// assert_eq!(image.registry(), None);
    /// ```
    #[must_use]
    pub fn registry(&self) -> Option<&str> {
        self.registry_end.map(|end| &self.inner[..end])
    }

    /// Set the registry portion of the image name, use [`None`] to remove it.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Name};
    ///
    /// let mut image = Image::parse("quay.io/k9withabone/podlet").unwrap();
    ///
    /// image.set_registry(None);
    /// assert_eq!(image.registry(), None);
    /// assert_eq!(image, "k9withabone/podlet");
    ///
    /// image.set_registry(Some(Name::new("docker.io").unwrap()));
    /// assert_eq!(image.registry(), Some("docker.io"));
    /// assert_eq!(image, "docker.io/k9withabone/podlet");
    ///
    /// image.set_registry(Some(Name::new("quay.io").unwrap()));
    /// assert_eq!(image.registry(), Some("quay.io"));
    /// assert_eq!(image, "quay.io/k9withabone/podlet");
    /// ```
    pub fn set_registry(&mut self, registry: Option<Name>) {
        match (registry, self.registry_end) {
            // Replace registry
            (Some(registry), Some(end)) => {
                let registry = registry.into_inner();
                self.inner.replace_range(..end, registry);
                let new_len = registry.len();
                self.registry_end = Some(new_len);
                self.update_tag_or_digest_start(end, new_len);
            }
            // Add registry
            (Some(registry), None) => {
                let registry = registry.into_inner();
                self.inner = format!("{registry}/{}", &self.inner);
                self.registry_end = Some(registry.len());
                self.update_tag_or_digest_start(0, registry.len() + 1);
            }
            // Remove registry
            (None, Some(mut end)) => {
                // Add one to end for '/' separator.
                end += 1;
                self.inner.replace_range(..end, "");
                self.registry_end = None;
                self.update_tag_or_digest_start(end, 0);
            }
            // Status quo
            (None, None) => {}
        }
    }

    /// The full name portion of the image, including the registry, as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::Image;
    ///
    /// let image = Image::parse("quay.io/podman/hello:latest").unwrap();
    /// assert_eq!(image.name(), "quay.io/podman/hello");
    /// ```
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner[..self.name_end()]
    }

    /// The [`Name`] portion of the image.
    #[must_use]
    pub fn as_name(&self) -> Name {
        Name::new_unchecked(self.name(), self.registry_end)
    }

    /// Set the name portion of the image.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Name};
    ///
    /// let mut image = Image::parse("quay.io/podman/hello:latest").unwrap();
    /// assert_eq!(image.name(), "quay.io/podman/hello");
    ///
    /// image.set_name(Name::new("docker.io/library/busybox").unwrap());
    /// assert_eq!(image.name(), "docker.io/library/busybox");
    /// ```
    pub fn set_name(&mut self, name: Name) {
        let end = self.name_end();
        self.inner.replace_range(..end, name.as_ref());

        self.registry_end = name.registry_end();

        self.update_tag_or_digest_start(end, name.into_inner().len());
    }

    /// Return the byte positions where the image name ends.
    fn name_end(&self) -> usize {
        // Subtract one from tag or digest start for separator.
        self.tag_or_digest_start
            .map_or_else(|| self.inner.len(), |start| start.into_inner() - 1)
    }

    /// Returns a string slice of the image's tag if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::Image;
    ///
    /// let image = Image::parse("quay.io/podman/hello:latest").unwrap();
    /// assert_eq!(image.tag(), Some("latest"));
    /// ```
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        if let Some(TagOrDigestStart::Tag(start)) = self.tag_or_digest_start {
            Some(&self.inner[start..])
        } else {
            None
        }
    }

    /// The [`Tag`] portion of the image, if it has one.
    #[must_use]
    pub fn as_tag(&self) -> Option<Tag> {
        self.tag().map(Tag::new_unchecked)
    }

    /// Set or remove the image's tag.
    ///
    /// If the image has a digest it is removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Tag};
    ///
    /// let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// let mut image = Image::parse(format!("quay.io/podman/hello@{digest}")).unwrap();
    ///
    /// image.set_tag(Some(Tag::new("latest").unwrap()));
    /// assert_eq!(image.tag(), Some("latest"));
    /// ```
    pub fn set_tag(&mut self, tag: Option<Tag>) {
        self.set_tag_or_digest(tag.map(Into::into));
    }

    /// Returns a string slice of the image's digest if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::Image;
    ///
    /// let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// let image = Image::parse(format!("quay.io/podman/hello@{digest}")).unwrap();
    ///
    /// assert_eq!(image.digest(), Some(digest));
    /// ```
    #[must_use]
    pub fn digest(&self) -> Option<&str> {
        if let Some(TagOrDigestStart::Digest(start)) = self.tag_or_digest_start {
            Some(&self.inner[start..])
        } else {
            None
        }
    }

    /// The [`Digest`] portion of the image, if it has one.
    #[must_use]
    pub fn as_digest(&self) -> Option<Digest> {
        self.digest().map(Digest::new_unchecked)
    }

    /// Set or remove the image's digest.
    ///
    /// If the image has a tag it is removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Digest};
    ///
    /// let mut image = Image::parse(format!("quay.io/podman/hello:latest")).unwrap();
    ///
    /// let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// image.set_digest(Some(Digest::new(digest).unwrap()));
    /// assert_eq!(image.digest(), Some(digest));
    /// ```
    pub fn set_digest(&mut self, digest: Option<Digest>) {
        self.set_tag_or_digest(digest.map(Into::into));
    }

    /// The [`TagOrDigest`] portion of the image, if it has one.
    #[must_use]
    pub fn as_tag_or_digest(&self) -> Option<TagOrDigest> {
        match self.tag_or_digest_start {
            Some(TagOrDigestStart::Tag(start)) => {
                let tag = Tag::new_unchecked(&self.inner[start..]);
                Some(TagOrDigest::Tag(tag))
            }
            Some(TagOrDigestStart::Digest(start)) => {
                let digest = Digest::new_unchecked(&self.inner[start..]);
                Some(TagOrDigest::Digest(digest))
            }
            None => None,
        }
    }

    /// Set or remove the image's tag or digest.
    pub fn set_tag_or_digest(&mut self, tag_or_digest: Option<TagOrDigest>) {
        match (tag_or_digest, self.tag_or_digest_start) {
            // Replace tag
            (Some(TagOrDigest::Tag(tag)), Some(TagOrDigestStart::Tag(start))) => {
                self.inner.replace_range(start.., tag.into_inner());
            }
            // Replace digest
            (Some(TagOrDigest::Digest(digest)), Some(TagOrDigestStart::Digest(start))) => {
                self.inner.replace_range(start.., digest.into_inner());
            }
            // Set tag or digest / replace one with the other
            (Some(tag_or_digest), Some(_) | None) => {
                self.inner.truncate(self.name_end());
                // Add one for separator
                self.tag_or_digest_start = Some(tag_or_digest.as_start(self.inner.len()));
                tag_or_digest.push_to_string(&mut self.inner);
            }
            // Remove tag or digest
            (None, Some(start)) => {
                // Subtract one from tag or digest start for separator
                let new_end = start.into_inner() - 1;
                self.tag_or_digest_start = None;
                self.inner.truncate(new_end);
            }
            // Status quo
            (None, None) => {}
        }
    }

    /// Update the start position of the tag or digest, if it exists.
    ///
    /// The absolute sizes of `old_len` and `new_len` do not matter, only their relative size.
    fn update_tag_or_digest_start(&mut self, old_len: usize, new_len: usize) {
        if let Some(tag_or_digest) = &mut self.tag_or_digest_start {
            match old_len.cmp(&new_len) {
                Ordering::Less => *tag_or_digest += new_len - old_len,
                Ordering::Equal => {}
                Ordering::Greater => *tag_or_digest -= old_len - new_len,
            }
        }
    }

    /// The [`Name`] and [`TagOrDigest`] parts of the image.
    #[must_use]
    pub fn as_parts(&self) -> (Name, Option<TagOrDigest>) {
        (self.as_name(), self.as_tag_or_digest())
    }

    /// Consume the [`Image`] and return its inner [`String`].
    #[must_use]
    pub fn into_inner(self) -> String {
        self.inner
    }
}

/// Error returned when parsing an [`Image`] from a string.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InvalidImageError {
    /// Given digest was invalid.
    #[error("invalid image digest")]
    Digest(#[from] InvalidDigestError),

    /// Given tag was invalid.
    #[error("invalid image tag")]
    Tag(#[from] InvalidTagError),

    /// Both a tag and digest were given.
    #[error("image cannot have a tag and a digest")]
    TagAndDigest,

    /// Part of the given image name was invalid.
    #[error("invalid image name part")]
    NamePart(#[from] InvalidNamePartError),
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl Eq for Image {}

impl PartialEq<str> for Image {
    fn eq(&self, other: &str) -> bool {
        self.inner.eq(other)
    }
}

impl PartialEq<&str> for Image {
    fn eq(&self, other: &&str) -> bool {
        self.inner.eq(other)
    }
}

impl PartialOrd for Image {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Image {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl Hash for Image {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl_from_str!(Image => InvalidImageError);

impl<'a> From<&'a Image> for (Name<'a>, Option<TagOrDigest<'a>>) {
    fn from(value: &'a Image) -> Self {
        value.as_parts()
    }
}

impl<'a> From<(Name<'a>, Option<TagOrDigest<'a>>)> for Image {
    fn from((name, tag_or_digest): (Name<'a>, Option<TagOrDigest<'a>>)) -> Self {
        Self::from_parts(name, tag_or_digest)
    }
}

impl<'a> From<(Name<'a>, TagOrDigest<'a>)> for Image {
    fn from((name, tag_or_digest): (Name<'a>, TagOrDigest<'a>)) -> Self {
        (name, Some(tag_or_digest)).into()
    }
}

impl<'a> From<Name<'a>> for Image {
    fn from(value: Name<'a>) -> Self {
        (value, None).into()
    }
}

impl AsRef<str> for Image {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Image {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<Image> for String {
    fn from(value: Image) -> Self {
        value.into_inner()
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.inner)
    }
}

/// Byte position where the tag or digest starts, after the separator (: or @), in an [`Image`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagOrDigestStart {
    /// The [`Image`] contains a [`Tag`].
    Tag(usize),
    /// The [`Image`] contains a [`Digest`].
    Digest(usize),
}

impl TagOrDigestStart {
    /// Return the inner start value for either variant.
    const fn into_inner(self) -> usize {
        match self {
            Self::Tag(tag_start) => tag_start,
            Self::Digest(digest_start) => digest_start,
        }
    }
}

impl AsMut<usize> for TagOrDigestStart {
    fn as_mut(&mut self) -> &mut usize {
        match self {
            Self::Tag(tag_start) => tag_start,
            Self::Digest(digest_start) => digest_start,
        }
    }
}

impl AddAssign<usize> for TagOrDigestStart {
    fn add_assign(&mut self, rhs: usize) {
        *self.as_mut() += rhs;
    }
}

impl SubAssign<usize> for TagOrDigestStart {
    fn sub_assign(&mut self, rhs: usize) {
        *self.as_mut() -= rhs;
    }
}

/// Validated [`Image`] [`Tag`] or [`Digest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagOrDigest<'a> {
    /// Validated [`Image`] [`Tag`].
    Tag(Tag<'a>),
    /// Validated [`Image`] [`Digest`].
    Digest(Digest<'a>),
}

impl<'a> TagOrDigest<'a> {
    /// Returns [`Some`] if [`Tag`].
    #[must_use]
    pub const fn into_tag(self) -> Option<Tag<'a>> {
        if let Self::Tag(tag) = self {
            Some(tag)
        } else {
            None
        }
    }

    /// Returns [`Some`] if [`Digest`].
    #[must_use]
    pub const fn into_digest(self) -> Option<Digest<'a>> {
        if let Self::Digest(digest) = self {
            Some(digest)
        } else {
            None
        }
    }

    /// Returns the separator character (':' or '@') to use this tag or digest in an [`Image`].
    #[must_use]
    pub const fn separator(&self) -> char {
        match self {
            Self::Tag(_) => ':',
            Self::Digest(_) => '@',
        }
    }

    /// Length in bytes of the inner string slice, plus one for the beginning separator (':' or '@').
    fn len(&self) -> usize {
        match self {
            Self::Tag(tag) => tag.as_ref().len() + 1,
            Self::Digest(digest) => digest.as_ref().len() + 1,
        }
    }

    /// Create a [`TagOrDigestStart`] based on the [`TagOrDigest`] variant.
    const fn as_start(&self, name_end: usize) -> TagOrDigestStart {
        // Add one for separator.
        let start = name_end + 1;
        match self {
            Self::Tag(_) => TagOrDigestStart::Tag(start),
            Self::Digest(_) => TagOrDigestStart::Digest(start),
        }
    }

    /// Push the correct separator (':' or '@'), then the inner string slice to the given `string`.
    fn push_to_string(&self, string: &mut String) {
        string.push(self.separator());
        string.push_str(self.as_ref());
    }
}

impl<'a> From<Tag<'a>> for TagOrDigest<'a> {
    fn from(value: Tag<'a>) -> Self {
        Self::Tag(value)
    }
}

impl<'a> From<Digest<'a>> for TagOrDigest<'a> {
    fn from(value: Digest<'a>) -> Self {
        Self::Digest(value)
    }
}

impl<'a> AsRef<str> for TagOrDigest<'a> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Tag(tag) => tag.as_ref(),
            Self::Digest(digest) => digest.as_ref(),
        }
    }
}

/// Returns `true` if `char` is a lowercase ASCII alphanumeric character.
const fn char_is_alnum(char: char) -> bool {
    matches!(char, 'a'..='z' | '0'..='9')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parts_eq(
        image: &Image,
        registry: Option<&str>,
        name: &str,
        tag_or_digest: Option<&str>,
    ) {
        assert_eq!(image.registry(), registry);
        assert_eq!(image.name(), name);
        assert_eq!(
            image.as_tag_or_digest().as_ref().map(AsRef::as_ref),
            tag_or_digest,
        );
    }

    #[test]
    fn registry() {
        let mut image = Image::parse("quay.io/podman/hello:latest").unwrap();
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );

        // Replace registry
        image.set_registry(Some(Name::new("docker.io").unwrap()));
        assert_parts_eq(
            &image,
            Some("docker.io"),
            "docker.io/podman/hello",
            Some("latest"),
        );

        // Remove registry
        image.set_registry(None);
        assert_parts_eq(&image, None, "podman/hello", Some("latest"));

        // Add registry
        image.set_registry(Some(Name::new("quay.io").unwrap()));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );
    }

    #[test]
    fn name() {
        let mut image = Image::parse("quay.io/podman/hello:latest").unwrap();
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );
        assert_eq!(image.as_name(), "quay.io/podman/hello");

        image.set_name(Name::new("docker.io/library/busybox").unwrap());
        assert_parts_eq(
            &image,
            Some("docker.io"),
            "docker.io/library/busybox",
            Some("latest"),
        );
        assert_eq!(image.as_name(), "docker.io/library/busybox");
    }

    #[test]
    fn tag_and_digest() {
        let mut image = Image::parse("quay.io/podman/hello:latest").unwrap();
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );
        assert_eq!(image.as_tag().unwrap(), "latest");

        // Replace tag
        image.set_tag(Some(Tag::new("test").unwrap()));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("test"),
        );

        // Replace tag with digest
        let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
        image.set_digest(Some(Digest::new(digest).unwrap()));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some(digest),
        );
        assert_eq!(image.as_digest().unwrap(), digest);

        // Replace digest
        image.set_digest(Some(Digest::new("algo:data").unwrap()));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("algo:data"),
        );

        // Remove tag or digest
        image.set_tag_or_digest(None);
        assert_parts_eq(&image, Some("quay.io"), "quay.io/podman/hello", None);

        // Add tag back
        image.set_tag(Some(Tag::new("latest").unwrap()));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );
    }
}
