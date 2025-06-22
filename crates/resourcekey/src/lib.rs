//! Resource key library. Of the form `namespace:item`. TODO

#![allow(clippy::deref_addrof)]

use ahash::RandomState;
use core::fmt;
use std::{
    alloc::Layout,
    hash::{BuildHasher, Hash, Hasher},
    mem::{self, MaybeUninit},
    num::NonZeroU64,
    ops::Deref,
    ptr::{self, NonNull},
    slice, str,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

#[doc(hidden)]
pub use ctor as __ctor;

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
///
/// Also, hard limit for resource key length is `i32::MAX`. It makes stuff faster and also like why would you need a resource key with a length of 2 billion
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResourceKey(ResourceKeyPtr);

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
    pub const fn verify(str: &str) -> Result<ResourceKeyVerifyResult, ResourceKeyParseError<'_>> {
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
        let Some(divider_pos) = colon_position else {
            bail!(str, ResourceKeyParseErrorKind::NoColon);
        };

        Ok(ResourceKeyVerifyResult { len, divider_pos })
    }

    /// Creates a [`ResourceKey`] from a string slice.
    /// See [`crate::ResourceKey`] for error conditions.
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        Self::verify(str)?;
        // SAFETY: we just verified the string to be valid
        Ok(unsafe { Self::new_unchecked(str) })
    }

    /// Creates a [`ResourceKey`] from a [`&str`].
    ///
    /// # Safety
    /// The parameter must be a valid resource key string. See [`crate::ResourceKey`] for the requirements.
    /// You can also use [`crate::ResourceKey::verify`] to check if a string is valid.
    pub unsafe fn new_unchecked(str: &str) -> Self {
        // SAFETY: str is a valid resource key string and thus has exactly one colon.
        let divider_pos = unsafe { str.bytes().position(|x| x == b':').unwrap_unchecked() as u32 };
        Self(unsafe { ResourceKeyPtr::new_arc(Some(divider_pos), str) })
    }

    pub fn divider_pos(&self) -> u32 {
        self.0.divider_pos
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn is(&self, hash: ResourceKeyHash) -> bool {
        self.0.hash == hash
    }

    pub fn item(&self) -> Item {
        Item(self.0.clone())
    }
    pub fn item_str(&self) -> &str {
        // SAFETY: self.0 is a valid ResourceKeyInner
        unsafe { self.as_str().get_unchecked(0..self.0.divider_pos as usize) }
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
pub struct Namespace(ResourceKeyPtr);
impl Namespace {
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };

        Ok(Self(unsafe { ResourceKeyPtr::new_arc(None, str) }))
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
pub struct Item(ResourceKeyPtr);
impl Item {
    pub fn new(str: &str) -> Result<Self, ResourceKeyParseError> {
        let bytes = str.as_bytes();
        if bytes.len() > i32::MAX as usize {
            bail!(str, ResourceKeyParseErrorKind::TooLong);
        };

        Ok(Self(unsafe { ResourceKeyPtr::new_arc(None, str) }))
    }
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}
impl AsRef<str> for Item {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
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
        self.0.item_hash.hash(state)
    }
}
impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Clone, Copy)]
enum ResourceKeyPtrTag<'a> {
    Arc(&'a AtomicUsize),
    Static,
}

/// `self.0` is pointer tagged. If `& 1 == 0`, it's a reference-counted heap-allocated box. If `& 1 == 1`, it's a `'static` string reference.
#[doc(hidden)]
pub struct ResourceKeyPtr(NonNull<ResourceKeyHeader>);
unsafe impl Send for ResourceKeyPtr {}
unsafe impl Sync for ResourceKeyPtr {}
impl ResourceKeyPtr {
    pub fn into_raw(self) -> NonNull<ResourceKeyHeader> {
        self.0
    }
    pub fn from_raw(ptr: NonNull<ResourceKeyHeader>) -> Self {
        Self(ptr)
    }

    const HEADER_SIZE: usize = size_of::<ResourceKeyHeader>();
    fn tag(&self) -> ResourceKeyPtrTag<'_> {
        match self.0.addr().get() & 1 {
            0 => ResourceKeyPtrTag::Arc(unsafe {
                let ptr: NonNull<()> = self.0.cast();
                let ptr_to_refcnt: NonNull<AtomicUsize> = ptr.byte_add(Self::HEADER_SIZE).cast();
                ptr_to_refcnt.as_ref()
            }),
            1 => ResourceKeyPtrTag::Static,
            _ => unreachable!(),
        }
    }
    fn untagged_ptr(&self) -> NonNull<ResourceKeyHeader> {
        match self.tag() {
            ResourceKeyPtrTag::Arc(_) => self.0,
            ResourceKeyPtrTag::Static => unsafe { self.0.byte_offset(-1) },
        }
    }
    /// Returns the size of the header, including the reference counter (if any).
    fn header_size(&self) -> usize {
        match self.tag() {
            ResourceKeyPtrTag::Arc(_) => Self::HEADER_SIZE + size_of::<AtomicUsize>(),
            ResourceKeyPtrTag::Static => Self::HEADER_SIZE,
        }
    }
    /// Returns the total heap-allocated length of `self`. Guaranteed to be a multiple of `core::mem::size_of::<usize>()`.
    fn total_size(&self) -> usize {
        self.header_size() + (self.len as usize).div_ceil(size_of::<usize>()) * size_of::<usize>()
    }
    fn as_str(&self) -> &str {
        self.as_ref()
    }

    unsafe fn new_arc(divider_pos: Option<u32>, str: &str) -> Self {
        let len: u32 = str.len().try_into().expect("str.len() has to fit in a u32");

        let header_size = Self::HEADER_SIZE + size_of::<AtomicUsize>();

        // size as a multiple of size_of::<usize>()
        let total_size = header_size / size_of::<usize>() + str.len().div_ceil(size_of::<usize>());

        let allocated: Box<[MaybeUninit<usize>]> = Box::new_uninit_slice(total_size);

        unsafe {
            Self::new_with_uninit_buf(
                divider_pos,
                str,
                NonNull::new_unchecked(Box::into_raw(allocated)).cast(),
            )
        }
    }

    #[doc(hidden)]
    pub unsafe fn new_with_uninit_buf(
        divider_pos: Option<u32>,
        str: &str,
        ptr: NonNull<MaybeUninit<ResourceKeyHeader>>,
    ) -> Self {
        let len: u32 = str.len().try_into().expect("str.len() has to fit in a u32");

        // strictly speaking violates invariants bc ptr should be initialized but whatever
        let this = Self(ptr.cast());
        let base_ptr = this.untagged_ptr();
        let header_size = this.header_size();

        let hash = ResourceKeyHash::new(str);
        unsafe {
            base_ptr.write(ResourceKeyHeader {
                len,
                divider_pos: divider_pos.unwrap_or(!0),
                hash,
                namespace_hash: match divider_pos {
                    Some(pos) => ResourceKeyHash::new(str.get_unchecked(..pos as usize)),
                    None => hash,
                },
                item_hash: match divider_pos {
                    Some(pos) => ResourceKeyHash::new(str.get_unchecked(pos as usize + 1..)),
                    None => hash,
                },
            });
            ptr::copy_nonoverlapping(
                str.as_ptr(),
                base_ptr.cast::<u8>().byte_add(header_size).as_ptr(),
                len as usize,
            );
        }

        this
    }

    pub fn into_resourcekey(self) -> ResourceKey {
        ResourceKey(self)
    }
    pub fn into_namespace(self) -> Namespace {
        Namespace(self)
    }
    pub fn into_item(self) -> Item {
        Item(self)
    }
}
impl AsRef<str> for ResourceKeyPtr {
    fn as_ref(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(
                self.untagged_ptr()
                    .cast::<u8>()
                    .byte_add(self.header_size())
                    .as_ptr(),
                self.len as usize,
            ))
        }
    }
}

impl Deref for ResourceKeyPtr {
    type Target = ResourceKeyHeader;
    fn deref(&self) -> &Self::Target {
        // SAFETY: self.0 is a valid (tagged) pointer
        unsafe { self.untagged_ptr().as_ref() }
    }
}
impl Clone for ResourceKeyPtr {
    fn clone(&self) -> Self {
        match self.tag() {
            ResourceKeyPtrTag::Arc(refcnt) => {
                // standard arc stuff...
                const MAX_REFCOUNT: usize = (isize::MAX) as usize;
                let old_size = refcnt.fetch_add(1, Ordering::Relaxed);
                if old_size > MAX_REFCOUNT {
                    std::process::abort();
                }
                Self(self.0)
            }
            ResourceKeyPtrTag::Static => Self(self.0),
        }
    }
}
impl Drop for ResourceKeyPtr {
    fn drop(&mut self) {
        match self.tag() {
            ResourceKeyPtrTag::Arc(refcnt) => {
                if refcnt.fetch_sub(1, Ordering::Release) == 1 {
                    // woo drop logic!

                    drop(unsafe {
                        Box::from_raw(ptr::slice_from_raw_parts_mut(
                            self.untagged_ptr().as_ptr().cast::<usize>(),
                            self.total_size() / size_of::<usize>(),
                        ))
                    })
                }
            }
            ResourceKeyPtrTag::Static => (),
        }
    }
}
impl PartialEq for ResourceKeyPtr {
    fn eq(&self, other: &Self) -> bool {
        if self.0.addr() == other.0.addr() {
            return true;
        }
        self.hash == other.hash
    }
}
impl Eq for ResourceKeyPtr {}
impl Hash for ResourceKeyPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct ResourceKeyHeader {
    pub len: u32,
    pub divider_pos: u32,
    pub hash: ResourceKeyHash,
    pub namespace_hash: ResourceKeyHash,
    pub item_hash: ResourceKeyHash,
}
const _: () = {
    let align = mem::align_of::<ResourceKeyHeader>();
    assert!(align > 1 && align == size_of::<usize>());
};

impl PartialEq for ResourceKeyHeader {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl Eq for ResourceKeyHeader {}

impl Hash for ResourceKeyHeader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

static mut RANDOM_STATE_INITIALIZED: bool = false;
static mut RANDOM_STATE: MaybeUninit<RandomState> = MaybeUninit::uninit();

// std-less global randomstate generation. this has to be called before `RANDOM_STATE` is accessed directly, i.e. in functions marked #[ctor]. also this is not thread safe.
#[cold]
unsafe fn get_random_state_ctor() -> RandomState {
    unsafe {
        if !RANDOM_STATE_INITIALIZED {
            RANDOM_STATE_INITIALIZED = true;
            let mut buf = [0u64; 4];
            libc::getrandom(buf.as_mut_ptr().cast(), mem::size_of_val(&buf), 0);
            RANDOM_STATE = MaybeUninit::new(RandomState::with_seeds(buf[0], buf[1], buf[2], buf[3]))
        }
        (*(&raw const RANDOM_STATE)).assume_init_ref().clone()
    }
}

#[ctor::ctor]
fn init_random_state() {
    // SAFETY: ctor
    unsafe {
        get_random_state_ctor();
    }
}

// gets the random state in functions not marked #[ctor].
fn get_random_state() -> RandomState {
    RandomState::clone(unsafe { (*(&raw const RANDOM_STATE)).assume_init_ref() })
}

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub struct ResourceKeyHash(u64);
impl ResourceKeyHash {
    fn new(str: &str) -> Self {
        Self(get_random_state().hash_one(str))
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

#[doc(hidden)]
#[repr(C)]
pub struct StaticLiteralBuf<const LEN: usize> {
    pub header: ResourceKeyHeader,
    pub buf: [u8; LEN],
}

#[macro_export]
macro_rules! literal {
    ($text:literal $(,)?) => {{
        const TEXT: &'static str = $text;
        const RESULT: $crate::ResourceKeyVerifyResult = {
            match $crate::ResourceKey::verify(TEXT) {
                Ok(res) => res,
                Err(_) => panic!(concat!("invalid resource key: ", $text)),
            }
        };
        static mut RESOURCE_KEY_BUF: ::core::mem::MaybeUninit<
            $crate::StaticLiteralBuf<{ RESULT.len as usize }>,
        > = ::core::mem::MaybeUninit::uninit();

        #[$crate::__ctor::ctor(crate_path = $crate::__ctor)]
        fn init_resource_key_buf() {
            unsafe {
            $crate::ResourceKeyPtr::new_with_uninit_buf(
                Some(RESULT.divider_pos),
                TEXT,
                ::core::ptr::NonNull::new_unchecked(RESOURCE_KEY_BUF.as_mut_ptr().byte_offset(1)).cast(),
            );
            }
        }

        // SAFETY: after the #[ctor] function initializes the resource key, the key won't be modified. also, this won't be called in a #[ctor] function. hopefully.

        unsafe { $crate::ResourceKeyPtr::from_raw(::core::ptr::NonNull::new_unchecked(RESOURCE_KEY_BUF.as_mut_ptr()).cast()).into_resourcekey() }
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
