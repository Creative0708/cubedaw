//! Resource key library. Of the form `namespace:item`. TODO

use core::fmt;

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
/// Also, hard limit for resource key length is `u32::MAX`. It makes stuff faster and also like why would you need a resource key with a length of 4 billion
#[derive(Clone, PartialEq, Eq, Hash)]
// TODO: optimize, possibly ~~rip off~~ get inspiration from arcstr itself
pub struct ResourceKey(arcstr::ArcStr);

impl ResourceKey {
    pub const fn verify(str: &str) -> Result<(), ResourceKeyParseError<'_>> {
        macro_rules! bail {
            ($kind:expr) => {
                return Err(ResourceKeyParseError { str, kind: $kind })
            };
        }
        let bytes = str.as_bytes();
        if bytes.len() > u32::MAX as usize {
            bail!(ResourceKeyParseErrorKind::TooLong);
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
                                bail!(ResourceKeyParseErrorKind::MultipleColons {
                                    first: prev,
                                    second: i,
                                });
                            }
                            None => colon_position = Some(i),
                        }
                    }

                    if last_boundary_position == i {
                        bail!(ResourceKeyParseErrorKind::EmptyComponent {
                            pos: last_boundary_position,
                        });
                    }
                    last_boundary_position = i + 1;
                }
                b'a'..=b'z' | b'0'..=b'9' | b'_' => (),
                _ => bail!(ResourceKeyParseErrorKind::InvalidChar { pos: i }),
            }
            i += 1;
        }
        if colon_position.is_none() {
            bail!(ResourceKeyParseErrorKind::NoColon);
        }

        Ok(())
    }

    /// Creates a [`ResourceKey`] from a string slice.
    /// See [`crate::ResourceKey`] for error conditions.
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        Self::verify(str)?;
        // SAFETY: we just verified the string to be valid
        Ok(unsafe { Self::from_arcstr_unchecked(arcstr::ArcStr::from(str)) })
    }

    /// Creates a [`ResourceKey`] from an [`arcstr::ArcStr`].
    /// # Safety
    /// The parameter must be a valid resource key string. See [`crate::ResourceKey`] for the requirements.
    /// You can also use [`crate::ResourceKey::verify`] to check if a string is valid.
    pub const unsafe fn from_arcstr_unchecked(arcstr: arcstr::ArcStr) -> Self {
        Self(arcstr)
    }

    // TODO: precompute and store somewhere such that size_of::<ResourceKey>() == sizeof::<usize>()
    // but resource keys are usually really tiny so this shouldn't have too much overhead
    pub fn divider_pos(&self) -> u32 {
        // SAFETY: a resource key string always contains exactly one ':'.
        unsafe {
            self.0
                .as_bytes()
                .iter()
                .position(|x| *x == b':')
                .unwrap_unchecked() as u32
        }
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResourceKeyParseError<'a> {
    pub str: &'a str,
    pub kind: ResourceKeyParseErrorKind,
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

macro_rules! literal {
    ($text:expr $(,)?) => {
        let text = $text;
        const {
            if let Err(err) = $crate::ResourceKey::verify(text) {
                panic!("{}", err);
            }
        }
        unsafe { $crate::ResourceKey::from_arcstr_unchecked(arcstr::literal!(text)) }
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
