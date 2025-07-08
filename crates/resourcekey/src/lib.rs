//! Resource key library. Of the form `namespace:item`. TODO

use std::fmt;

#[doc(hidden)]
pub use arcstr as __arcstr;
use arcstr::ArcStr;

/// Resource key of the form `namespace:item`.
///
/// Both `namespace` and `item` are ASCII strings which only contain `[a-z0-9_.]`.
/// Components are separated by periods. Components must not be empty.
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
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResourceKey(ArcStr);

const fn is_valid_component_byte(ch: u8) -> bool {
    matches!(ch, b'a'..=b'z' | b'0'..=b'9' | b'_')
}

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
        // can't use try_into as it's not const
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };
        let len: u32 = str.len() as u32;
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
                ch if is_valid_component_byte(ch) => (),
                _ => bail!(str, ResourceKeyParseErrorKind::InvalidChar { pos: i }),
            }
            i += 1;
        }
        if colon_position.is_none() {
            bail!(str, ResourceKeyParseErrorKind::NoColon);
        };

        Ok(())
    }

    /// Creates a [`ResourceKey`] from a string slice.
    /// See [`crate::ResourceKey`] for error conditions.
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        Self::verify(str)?;
        Ok(Self(ArcStr::from(str)))
    }

    #[doc(hidden)]
    pub const fn new_static(str: &'static str, arcstr: ArcStr) -> Self {
        match Self::verify(str) {
            Ok(()) => (),
            Err(_) => panic!("invalid resource key"),
        }
        Self(arcstr)
    }

    pub fn divider_pos(&self) -> u32 {
        self.0.find(':').unwrap() as u32
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}
impl AsRef<str> for ResourceKey {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

pub struct ResourceKeyVerifyResult {
    pub len: u32,
    pub divider_pos: u32,
}

#[cfg(feature = "serde")]
mod serde;

#[derive(Clone)]
pub struct Namespace(ArcStr);
impl Namespace {
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };

        // todo!(): verify that the namespace is valid
        Ok(Self(ArcStr::from(str)))
    }
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}
impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
impl fmt::Debug for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

// #[derive(Clone)]
// pub struct Item(ResourceKeyPtr);
// impl Item {
//     pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
//         let bytes = str.as_bytes();
//         if bytes.len() > i32::MAX as usize {
//             bail!(str, ResourceKeyParseErrorKind::TooLong);
//         };

//         Ok(Self(unsafe { ResourceKeyPtr::new_arc(None, str) }))
//     }
//     pub fn as_str(&self) -> &str {
//         self.as_ref()
//     }
// }
// impl AsRef<str> for Item {
//     fn as_ref(&self) -> &str {
//         self.0.as_ref()
//     }
// }
// impl PartialEq for Item {
//     fn eq(&self, other: &Self) -> bool {
//         self.0.item_hash == other.0.item_hash
//     }
// }
// impl Eq for Item {}
// impl Hash for Item {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.0.item_hash.hash(state)
//     }
// }
// impl std::fmt::Debug for Item {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.as_str().fmt(f)
//     }
// }

// #[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
// pub struct ResourceKeyHash(u64);
// impl ResourceKeyHash {
//     fn new(str: &str) -> Self {
//         Self(get_random_state().hash_one(str))
//     }
// }

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
    ($text:literal $(,)?) => {
        const { $crate::ResourceKey::new_static($text, $crate::__arcstr::literal!($text)) }
    };
}

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
