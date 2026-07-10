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
/// Images contain a name and an optional tag and/or digest. Each part of the image specification
/// must conform to a specific format. See [`Name`], [`Tag`], and [`Digest`] for details. The
/// general format is `{name}[:{tag}][@{digest}]`, matching the
/// [OCI / distribution reference grammar](https://github.com/distribution/reference), which allows
/// a reference to carry both a tag and a digest.
///
/// [compose-spec](https://github.com/compose-spec/compose-spec/blob/master/05-services.md#image)
#[derive(SerializeDisplay, DeserializeTryFromString, Debug, Clone)]
pub struct Image {
    /// Inner string.
    inner: String,

    /// Byte position of `inner` where the registry ends, if the image has a registry part.
    registry_end: Option<usize>,

    /// Byte position of `inner` where the tag starts, after its `:` separator, if the image has a
    /// tag. When a digest is also present, it always follows the tag.
    tag_start: Option<usize>,

    /// Byte position of `inner` where the digest starts, after its `@` separator, if the image has
    /// a digest.
    digest_start: Option<usize>,
}

/// Byte positions parsed from an image string: where the registry ends, and where the tag and the
/// digest each start (after their `:` / `@` separator), when present.
type ImagePartStarts = (Option<usize>, Option<usize>, Option<usize>);

impl Image {
    /// Parse an [`Image`] from a string.
    ///
    /// # Errors
    ///
    /// Images are made up of a [`Name`] and an optional [`Tag`] and/or [`Digest`]. Each part has
    /// specific requirements for that part of the string to conform to. See [`Name::new()`],
    /// [`Tag::new()`], and [`Digest::new()`] for details.
    ///
    /// A reference may contain both a tag and a digest (e.g. `name:tag@digest`), as permitted by
    /// the OCI / distribution reference grammar.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::Image;
    ///
    /// let image = Image::parse("quay.io/podman/hello:latest").unwrap();
    ///
    /// assert_eq!(image, "quay.io/podman/hello:latest");
    /// assert_eq!(image.registry(), Some("quay.io"));
    /// assert_eq!(image.name(), "quay.io/podman/hello");
    /// assert_eq!(image.tag(), Some("latest"));
    /// assert_eq!(image.digest(), None);
    ///
    /// // Images may have both a tag and a digest.
    /// let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// let image = Image::parse(format!("quay.io/podman/hello:latest@{digest}")).unwrap();
    /// assert_eq!(image.tag(), Some("latest"));
    /// assert_eq!(image.digest(), Some(digest));
    /// ```
    pub fn parse<T>(image: T) -> Result<Self, InvalidImageError>
    where
        T: AsRef<str> + Into<String>,
    {
        let (registry_end, tag_start, digest_start) = Self::parse_impl(image.as_ref())?;

        Ok(Self {
            inner: image.into(),
            registry_end,
            tag_start,
            digest_start,
        })
    }

    /// Concrete implementation for [`Self::parse()`].
    ///
    /// Returns the byte positions, within `image`, where the registry ends and where the tag and
    /// digest start (each after its separator).
    fn parse_impl(image: &str) -> Result<ImagePartStarts, InvalidImageError> {
        let (image, digest_start) = image
            .split_once('@')
            .map_or(Ok((image, None)), |(image, digest)| {
                Digest::new(digest).map(|_| (image, Some(image.len() + 1)))
            })?;

        let (image, tag_start) = image
            .rsplit_once(':')
            // If tag contains '/', then image has a registry with a port and no tag.
            .filter(|(_, tag)| !tag.contains('/'))
            .map_or(Ok((image, None)), |(image, tag)| {
                Tag::new(tag).map(|_| (image, Some(image.len() + 1)))
            })?;

        let name = Name::new(image)?;

        Ok((name.registry_end(), tag_start, digest_start))
    }

    /// Create an [`Image`] from validated parts.
    ///
    /// To create an image with both a tag and a digest, use [`set_digest()`](Self::set_digest())
    /// (or [`set_tag()`](Self::set_tag())) on the result.
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

        let mut tag_start = None;
        let mut digest_start = None;
        if let Some(tag_or_digest) = tag_or_digest {
            // Add one for the separator.
            let start = inner.len() + 1;
            tag_or_digest.push_to_string(&mut inner);
            match tag_or_digest {
                TagOrDigest::Tag(_) => tag_start = Some(start),
                TagOrDigest::Digest(_) => digest_start = Some(start),
            }
        }

        Self {
            inner,
            registry_end,
            tag_start,
            digest_start,
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
        self.registry_end.map(|end| {
            // `registry_end` is always within `inner`.
            // `inner` only contains ASCII.
            // Checked with `registry()` test.
            #[allow(clippy::indexing_slicing, clippy::string_slice)]
            &self.inner[..end]
        })
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
                self.shift_tag_and_digest_start(end, new_len);
            }
            // Add registry
            (Some(registry), None) => {
                let registry = registry.into_inner();
                self.inner = format!("{registry}/{}", &self.inner);
                self.registry_end = Some(registry.len());
                self.shift_tag_and_digest_start(0, registry.len() + 1);
            }
            // Remove registry
            (None, Some(mut end)) => {
                // Add one to end for '/' separator.
                end += 1;
                self.inner.replace_range(..end, "");
                self.registry_end = None;
                self.shift_tag_and_digest_start(end, 0);
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
        // `name_end()` is always within `inner`.
        // `inner` only contains ASCII.
        // Checked with `name()` test.
        #[allow(clippy::indexing_slicing, clippy::string_slice)]
        &self.inner[..self.name_end()]
    }

    /// The [`Name`] portion of the image.
    #[must_use]
    pub fn as_name(&self) -> Name<'_> {
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

        self.shift_tag_and_digest_start(end, name.into_inner().len());
    }

    /// Return the byte position within `inner` where the image name ends, i.e. the first separator
    /// (`:` or `@`), or the end of the string if there is neither a tag nor a digest.
    fn name_end(&self) -> usize {
        // Subtract one from the tag or digest start for its separator.
        match (self.tag_start, self.digest_start) {
            (Some(start), _) | (None, Some(start)) => start - 1,
            (None, None) => self.inner.len(),
        }
    }

    /// Return the byte position within `inner` where the tag ends, i.e. the `@` before the digest,
    /// or the end of the string if there is no digest.
    fn tag_end(&self) -> usize {
        // Subtract one from the digest start for its separator.
        self.digest_start
            .map_or(self.inner.len(), |start| start - 1)
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
        self.tag_start.map(|start| {
            // `start` and `tag_end()` are always within `inner`.
            // `inner` only contains ASCII.
            // Checked with `tag_and_digest()` test.
            #[allow(clippy::indexing_slicing, clippy::string_slice)]
            &self.inner[start..self.tag_end()]
        })
    }

    /// The [`Tag`] portion of the image, if it has one.
    #[must_use]
    pub fn as_tag(&self) -> Option<Tag<'_>> {
        self.tag().map(Tag::new_unchecked)
    }

    /// Set or remove the image's tag.
    ///
    /// A digest, if present, is left unchanged; the tag is always placed before it.
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
    /// assert_eq!(image.digest(), Some(digest));
    /// ```
    pub fn set_tag(&mut self, tag: Option<Tag>) {
        match (tag, self.tag_start) {
            // Replace existing tag.
            (Some(tag), Some(start)) => {
                let tag = tag.into_inner();
                let end = self.tag_end();
                let old_len = end - start;
                self.inner.replace_range(start..end, tag);
                // A digest following the tag shifts by the change in tag length.
                if let Some(digest_start) = self.digest_start.as_mut() {
                    match tag.len().cmp(&old_len) {
                        Ordering::Greater => *digest_start += tag.len() - old_len,
                        Ordering::Less => *digest_start -= old_len - tag.len(),
                        Ordering::Equal => {}
                    }
                }
            }
            // Add a tag before the name's end (i.e. before any digest).
            (Some(tag), None) => {
                let tag = tag.into_inner();
                let name_end = self.name_end();
                self.inner.insert(name_end, ':');
                self.inner.insert_str(name_end + 1, tag);
                self.tag_start = Some(name_end + 1);
                // A digest shifts right by the inserted `:{tag}`.
                if let Some(digest_start) = self.digest_start.as_mut() {
                    *digest_start += tag.len() + 1;
                }
            }
            // Remove the tag, including its separator.
            (None, Some(start)) => {
                let end = self.tag_end();
                let removed = end - (start - 1);
                self.inner.replace_range((start - 1)..end, "");
                self.tag_start = None;
                if let Some(digest_start) = self.digest_start.as_mut() {
                    *digest_start -= removed;
                }
            }
            // Status quo.
            (None, None) => {}
        }
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
        self.digest_start.map(|start| {
            // The digest is always the last part of `inner`, so `start` is always within `inner`.
            // `inner` only contains ASCII.
            // Checked with `tag_and_digest()` test.
            #[allow(clippy::indexing_slicing, clippy::string_slice)]
            &self.inner[start..]
        })
    }

    /// The [`Digest`] portion of the image, if it has one.
    #[must_use]
    pub fn as_digest(&self) -> Option<Digest<'_>> {
        self.digest().map(Digest::new_unchecked)
    }

    /// Set or remove the image's digest.
    ///
    /// A tag, if present, is left unchanged; the digest is always placed after it.
    ///
    /// # Examples
    ///
    /// ```
    /// use compose_spec::service::image::{Image, Digest};
    ///
    /// let mut image = Image::parse("quay.io/podman/hello:latest").unwrap();
    ///
    /// let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
    /// image.set_digest(Some(Digest::new(digest).unwrap()));
    /// assert_eq!(image.digest(), Some(digest));
    /// assert_eq!(image.tag(), Some("latest"));
    /// ```
    pub fn set_digest(&mut self, digest: Option<Digest>) {
        // The digest is always the last part of `inner`, so no other positions shift.
        match (digest, self.digest_start) {
            // Replace existing digest.
            (Some(digest), Some(start)) => {
                self.inner.replace_range(start.., digest.into_inner());
            }
            // Append a digest.
            (Some(digest), None) => {
                self.digest_start = Some(self.inner.len() + 1);
                self.inner.push('@');
                self.inner.push_str(digest.into_inner());
            }
            // Remove the digest, including its separator.
            (None, Some(start)) => {
                self.inner.truncate(start - 1);
                self.digest_start = None;
            }
            // Status quo.
            (None, None) => {}
        }
    }

    /// The [`TagOrDigest`] portion of the image, if it has one.
    ///
    /// If the image has both a tag and a digest, the [`Digest`] is returned, as it is the more
    /// specific identifier. Use [`tag()`](Self::tag()) / [`digest()`](Self::digest()) (or their
    /// [`as_tag()`](Self::as_tag()) / [`as_digest()`](Self::as_digest()) counterparts) to access
    /// each part individually.
    #[must_use]
    pub fn as_tag_or_digest(&self) -> Option<TagOrDigest<'_>> {
        self.digest().map_or_else(
            || {
                self.tag()
                    .map(|tag| TagOrDigest::Tag(Tag::new_unchecked(tag)))
            },
            |digest| Some(TagOrDigest::Digest(Digest::new_unchecked(digest))),
        )
    }

    /// Set or remove the image's tag or digest.
    ///
    /// [`TagOrDigest::Tag`] delegates to [`set_tag()`](Self::set_tag()) and
    /// [`TagOrDigest::Digest`] to [`set_digest()`](Self::set_digest()), each of which leaves the
    /// other part unchanged. [`None`] removes both the tag and the digest.
    pub fn set_tag_or_digest(&mut self, tag_or_digest: Option<TagOrDigest>) {
        match tag_or_digest {
            Some(TagOrDigest::Tag(tag)) => self.set_tag(Some(tag)),
            Some(TagOrDigest::Digest(digest)) => self.set_digest(Some(digest)),
            None => {
                self.set_digest(None);
                self.set_tag(None);
            }
        }
    }

    /// Shift the tag and digest start positions to account for a change in the length of an earlier
    /// part of `inner` (e.g. the registry or name).
    ///
    /// The absolute sizes of `old_len` and `new_len` do not matter, only their relative size.
    fn shift_tag_and_digest_start(&mut self, old_len: usize, new_len: usize) {
        let shift = |start: &mut usize| match old_len.cmp(&new_len) {
            Ordering::Less => *start += new_len - old_len,
            Ordering::Equal => {}
            Ordering::Greater => *start -= old_len - new_len,
        };
        if let Some(start) = self.tag_start.as_mut() {
            shift(start);
        }
        if let Some(start) = self.digest_start.as_mut() {
            shift(start);
        }
    }

    /// The [`Name`] and [`TagOrDigest`] parts of the image.
    #[must_use]
    pub fn as_parts(&self) -> (Name<'_>, Option<TagOrDigest<'_>>) {
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

impl AsRef<str> for TagOrDigest<'_> {
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
    fn registry() -> Result<(), InvalidImageError> {
        let mut image = Image::parse("quay.io/podman/hello:latest")?;
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );

        // Replace registry
        image.set_registry(Some(Name::new("docker.io")?));
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
        image.set_registry(Some(Name::new("quay.io")?));
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );

        // Registry with port
        let image = Image::parse("quay.io:443/podman/hello")?;
        assert_parts_eq(
            &image,
            Some("quay.io:443"),
            "quay.io:443/podman/hello",
            None,
        );

        // Registry with port and tag
        let image = Image::parse("quay.io:443/podman/hello:latest")?;
        assert_parts_eq(
            &image,
            Some("quay.io:443"),
            "quay.io:443/podman/hello",
            Some("latest"),
        );

        Ok(())
    }

    #[test]
    fn name() -> Result<(), InvalidImageError> {
        let mut image = Image::parse("quay.io/podman/hello:latest")?;
        assert_parts_eq(
            &image,
            Some("quay.io"),
            "quay.io/podman/hello",
            Some("latest"),
        );
        assert_eq!(image.as_name(), "quay.io/podman/hello");

        image.set_name(Name::new("docker.io/library/busybox")?);
        assert_parts_eq(
            &image,
            Some("docker.io"),
            "docker.io/library/busybox",
            Some("latest"),
        );
        assert_eq!(image.as_name(), "docker.io/library/busybox");

        Ok(())
    }

    #[test]
    fn tag_and_digest() -> Result<(), InvalidImageError> {
        let mut image = Image::parse("quay.io/podman/hello:latest")?;
        assert_eq!(image.tag(), Some("latest"));
        assert_eq!(image.as_tag().map(Tag::into_inner), Some("latest"));

        // Replace the tag.
        image.set_tag(Some(Tag::new("test")?));
        assert_eq!(image, "quay.io/podman/hello:test");

        // Adding a digest leaves the tag in place.
        let digest = "sha256:075975296016084fc66b59c35c9d4504765d95aadcd5469f28d2b75750348fc5";
        image.set_digest(Some(Digest::new(digest)?));
        assert_eq!(
            image,
            format!("quay.io/podman/hello:test@{digest}").as_str()
        );
        assert_eq!(image.tag(), Some("test"));
        assert_eq!(image.digest(), Some(digest));
        assert_eq!(image.as_digest().map(Digest::into_inner), Some(digest));
        // `as_tag_or_digest` prefers the digest when both are present.
        assert_eq!(
            image.as_tag_or_digest().as_ref().map(AsRef::as_ref),
            Some(digest),
        );

        // Replacing the digest leaves the tag untouched.
        image.set_digest(Some(Digest::new("algo:data")?));
        assert_eq!(image, "quay.io/podman/hello:test@algo:data");
        assert_eq!(image.tag(), Some("test"));

        // Replacing the tag leaves the digest untouched.
        image.set_tag(Some(Tag::new("latest")?));
        assert_eq!(image, "quay.io/podman/hello:latest@algo:data");
        assert_eq!(image.digest(), Some("algo:data"));

        // Removing only the tag keeps the digest.
        image.set_tag(None);
        assert_eq!(image, "quay.io/podman/hello@algo:data");
        assert_eq!(image.tag(), None);
        assert_eq!(image.digest(), Some("algo:data"));

        // Adding the tag back places it before the digest.
        image.set_tag(Some(Tag::new("latest")?));
        assert_eq!(image, "quay.io/podman/hello:latest@algo:data");

        // Removing only the digest keeps the tag.
        image.set_digest(None);
        assert_eq!(image, "quay.io/podman/hello:latest");
        assert_eq!(image.digest(), None);
        assert_eq!(image.tag(), Some("latest"));

        // `set_tag_or_digest(None)` removes both.
        image.set_digest(Some(Digest::new(digest)?));
        image.set_tag_or_digest(None);
        assert_parts_eq(&image, Some("quay.io"), "quay.io/podman/hello", None);
        assert_eq!(image, "quay.io/podman/hello");

        Ok(())
    }

    #[test]
    fn parse_tag_and_digest() -> Result<(), InvalidImageError> {
        let digest = "sha256:4963247afc4cd33c7d3b2d2816b9f7f8eeebab148d29056c2ca4d7cbc966f2d9";

        // A reference with both a tag and a digest parses and round-trips unchanged.
        let image = Image::parse(format!("docker.io/valkey/valkey:9@{digest}"))?;
        assert_eq!(image.registry(), Some("docker.io"));
        assert_eq!(image.name(), "docker.io/valkey/valkey");
        assert_eq!(image.tag(), Some("9"));
        assert_eq!(image.digest(), Some(digest));
        assert_eq!(image.as_tag().map(Tag::into_inner), Some("9"));
        assert_eq!(image.as_digest().map(Digest::into_inner), Some(digest));
        assert_eq!(
            image,
            format!("docker.io/valkey/valkey:9@{digest}").as_str()
        );

        // Registry with a port, plus a tag and a digest.
        let mut image = Image::parse(format!("quay.io:443/podman/hello:latest@{digest}"))?;
        assert_eq!(image.registry(), Some("quay.io:443"));
        assert_eq!(image.name(), "quay.io:443/podman/hello");
        assert_eq!(image.tag(), Some("latest"));
        assert_eq!(image.digest(), Some(digest));

        // Changing the registry keeps both the tag and digest intact.
        image.set_registry(Some(Name::new("docker.io")?));
        assert_eq!(
            image,
            format!("docker.io/podman/hello:latest@{digest}").as_str()
        );
        assert_eq!(image.tag(), Some("latest"));
        assert_eq!(image.digest(), Some(digest));

        // `from_parts` plus `set_digest` builds an image with both.
        let mut built = Image::from_parts(
            Name::new("docker.io/valkey/valkey")?,
            Some(Tag::new("9")?.into()),
        );
        built.set_digest(Some(Digest::new(digest)?));
        assert_eq!(
            built,
            format!("docker.io/valkey/valkey:9@{digest}").as_str()
        );

        Ok(())
    }
}
