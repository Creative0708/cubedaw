//! Resource key library. Of the form `namespace:item`. TODO

use core::fmt;
use std::{
    hash::{Hash, Hasher},
    num::NonZeroU64,
    sync::Arc,
};

/// Resource key of the form `namespace:item`.
///
/// Both `namespace` and `item` are ASCII strings which only contain `[a-z0-9_.]`.
/// Periods separate components. Components must not be empty.
///
/// Valid keys:
/// - `module:item`
/// - `module.author:category.item`
/// - `a.b.c.d:e.f.g.h`
///
/// Invalid keys:
/// - `no_colon`
/// - `mult:iple:colons`
/// - `Invalid:characters`
/// - `:empty_namespace`
/// - `empty..component:a`
///
/// Also, hard limit for resource key length is `i32::MAX`. It makes stuff faster and also like why would you need a resource key with a length of 2 billion
#[derive(Clone, PartialEq, Eq, Hash)]
// TODO: optimize, possibly ~~rip off~~ get inspiration from arcstr which stores its string "inline"
// https://docs.rs/arcstr/latest/arcstr/
pub struct ResourceKey(Arc<ResourceKeyInner>);

macro_rules! bail {
    ($str:ident, $kind:expr) => {
        return Err(ResourceKeyParseError {
            str: $str,
            kind: $kind,
        })
    };
}
impl ResourceKey {
    pub const fn verify(str: &str) -> Result<(), ResourceKeyParseError<'_>> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };
        let len = bytes.len() as u32;
        let mut i = 0;
        let mut colon_position = None;
        let mut last_boundary_position = 0;
        while i <= len {
            // the else { b'.' } is just there to form a boundary at the end of the string
            let byte = if i < len { bytes[i as usize] } else { b'.' };

            match byte {
                b':' | b'.' => {
                    if byte == b':' {
                        match colon_position {
                            Some(prev) => {
                                bail!(
                                    str,
                                    ResourceKeyParseErrorKind::MultipleColons {
                                        first: prev,
                                        second: i,
                                    }
                                );
                            }
                            None => colon_position = Some(i),
                        }
                    }

                    if last_boundary_position == i {
                        bail!(
                            str,
                            ResourceKeyParseErrorKind::EmptyComponent {
                                pos: last_boundary_position,
                            }
                        );
                    }
                    last_boundary_position = i + 1;
                }
                b'a'..=b'z' | b'0'..=b'9' | b'_' => (),
                _ => bail!(str, ResourceKeyParseErrorKind::InvalidChar { pos: i }),
            }
            i += 1;
        }
        if colon_position.is_none() {
            bail!(str, ResourceKeyParseErrorKind::NoColon);
        }

        Ok(())
    }

    /// Creates a [`ResourceKey`] from a string slice.
    /// See [`crate::ResourceKey`] for error conditions.
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        Self::verify(str)?;
        // SAFETY: we just verified the string to be valid
        Ok(unsafe { Self::from_boxstr_unchecked(Box::from(str)) })
    }

    /// Creates a [`ResourceKey`] from an [`Box<str>`].
    /// # Safety
    /// The parameter must be a valid resource key string. See [`crate::ResourceKey`] for the requirements.
    /// You can also use [`crate::ResourceKey::verify`] to check if a string is valid.
    pub unsafe fn from_boxstr_unchecked(boxstr: Box<str>) -> Self {
        // SAFETY: boxstr is a valid resource key string and thus has exactly one colon.
        let divider_pos =
            unsafe { boxstr.bytes().position(|x| x == b':').unwrap_unchecked() as u32 };
        Self(Arc::new(ResourceKeyInner {
            divider_pos,
            hash: Some(ResourceKeyHash::new(&boxstr)),
            namespace_hash: Some(ResourceKeyHash::new(&unsafe {
                boxstr.get_unchecked(..divider_pos as usize)
            })),
            item_hash: Some(ResourceKeyHash::new(&unsafe {
                boxstr.get_unchecked(divider_pos as usize + 1..)
            })),
            str: boxstr,
        }))
    }

    pub fn divider_pos(&self) -> u32 {
        self.0.divider_pos
    }

    pub fn as_str(&self) -> &str {
        &self.0.str
    }

    pub fn is(&self, hash: ResourceKeyHash) -> bool {
        self.0.hash == Some(hash)
    }

    pub fn item(&self) -> Item {
        Item(self.0.clone())
    }
    pub fn item_str(&self) -> &str {
        // SAFETY: self.0 is a valid ResourceKeyInner
        unsafe { self.0.str.get_unchecked(0..self.0.divider_pos as usize) }
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.str.fmt(f)
    }
}
impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.str.fmt(f)
    }
}

#[cfg(feature = "serde")]
mod serde;

#[derive(Clone)]
pub struct Namespace(Arc<ResourceKeyInner>);
impl Namespace {
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };

        Ok(Self(Arc::new(ResourceKeyInner {
            str: str.into(),
            divider_pos: !0,
            hash: None,
            namespace_hash: Some(ResourceKeyHash::new(str)),
            item_hash: None,
        })))
    }
    pub fn as_str(&self) -> &str {
        // SAFETY: self.0 is a valid ResourceKeyInner
        unsafe { &self.0.str.get_unchecked(0..self.0.divider_pos as usize) }
    }
}
impl PartialEq for Namespace {
    fn eq(&self, other: &Self) -> bool {
        self.0.namespace_hash == other.0.namespace_hash
    }
}
impl Eq for Namespace {}
impl Hash for Namespace {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.item_hash.hash(state)
    }
}
impl std::fmt::Debug for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Clone)]
pub struct Item(Arc<ResourceKeyInner>);
impl Item {
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };

        Ok(Self(Arc::new(ResourceKeyInner {
            str: str.into(),
            divider_pos: !0,
            hash: None,
            namespace_hash: Some(ResourceKeyHash::new(str)),
            item_hash: None,
        })))
    }
    pub fn as_str(&self) -> &str {
        // SAFETY: self.0 is a valid ResourceKeyInner
        unsafe { &self.0.str.get_unchecked(0..self.0.divider_pos as usize) }
    }
}
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.0.item_hash == other.0.item_hash
    }
}
impl Eq for Item {}
impl Hash for Item {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.namespace_hash.hash(state)
    }
}
impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// TODO optimize at some point in the future
struct ResourceKeyInner {
    pub str: Box<str>,
    pub divider_pos: u32,
    pub hash: Option<ResourceKeyHash>,
    pub namespace_hash: Option<ResourceKeyHash>,
    pub item_hash: Option<ResourceKeyHash>,
}

impl PartialEq for ResourceKeyInner {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl Eq for ResourceKeyInner {}

impl Hash for ResourceKeyInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub struct ResourceKeyHash(NonZeroU64);
impl ResourceKeyHash {
    const fn new(str: &str) -> Self {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(str);
        Self(NonZeroU64::new(hash).expect("unreachable"))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResourceKeyParseError<'a> {
    pub str: &'a str,
    pub kind: ResourceKeyParseErrorKind,
}
impl std::fmt::Display for ResourceKeyParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, PartialEq, Eq)]
// TODO document
pub enum ResourceKeyParseErrorKind {
    NoColon,
    MultipleColons { first: u32, second: u32 },
    InvalidChar { pos: u32 },
    EmptyComponent { pos: u32 },
    TooLong,
}

#[macro_export]
macro_rules! literal {
    ($text:expr $(,)?) => {{
        // TODO: optimize, possibly compute hashes at compile time
        const TEXT: &'static str = $text;
        const {
            if let Err(err) = $crate::ResourceKey::verify(TEXT) {
                panic!(concat!("invalid resource key: ", $text));
            }
        }
        unsafe { $crate::ResourceKey::from_boxstr_unchecked(String::from(TEXT).into_boxed_str()) }
    }};
}

// TODO
// macro_rules! hash {
//     ($str:expr) => {
//         static HASH_CACHE: AtomicU64 = AtomicU64::new(0);
//         let loaded = HASH_CACHE.load(Ordering::Relaxed);
//         if loaded != 0 {
//             return loaded;
//         }
//     };
// }

#[cfg(test)]
mod tests {

    use crate::{ResourceKey, ResourceKeyParseError, ResourceKeyParseErrorKind};

    #[test]
    pub fn test_parsing() {
        // replace with assert_matches! when it's stabilized for better error msgs
        // https://doc.rust-lang.org/std/assert_matches/macro.assert_matches.html
        assert!(ResourceKey::new("module:item").is_ok());
        assert!(ResourceKey::new("module.author:category.item").is_ok());
        assert!(ResourceKey::new("a.b.c.d:e.f.g.h").is_ok());

        for (str, kind) in [
            ("no_colon", ResourceKeyParseErrorKind::NoColon),
            (
                "mult:iple:colons",
                ResourceKeyParseErrorKind::MultipleColons {
                    first: 4,
                    second: 9,
                },
            ),
            (
                "Invalid:characters",
                ResourceKeyParseErrorKind::InvalidChar { pos: 0 },
            ),
            (
                ":empty_namespace",
                ResourceKeyParseErrorKind::EmptyComponent { pos: 0 },
            ),
            (
                "empty..component:a",
                ResourceKeyParseErrorKind::EmptyComponent { pos: 6 },
            ),
        ] {
            assert_eq!(
                ResourceKey::new(str),
                Err(ResourceKeyParseError { str, kind })
            );
        }
    }
    // TODO: tests for components and such
}
