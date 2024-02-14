use std::{
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
    sync::atomic::AtomicU64,
};

use egui::{ahash::AHasher, epaint::ahash::RandomState};

fn new_hasher() -> AHasher {
    RandomState::with_seeds(
        0x7631_a132_54ea_fc47,
        0xff44_f51d_93bf_9bb0,
        0x4d08_8811_297a_42f4,
        0x2367_3b90_f1e9_b53c,
    )
    .build_hasher()
}

static COUNTER: AtomicU64 = AtomicU64::new(2);

#[repr(transparent)]
pub struct Id<T: Sized>(pub NonZeroU64, PhantomData<T>);
impl<T> Id<T> {
    pub fn new(source: impl std::hash::Hash) -> Self {
        let mut hasher = new_hasher();
        source.hash(&mut hasher);
        Self::from_raw(NonZeroU64::new(hasher.finish()).unwrap())
    }

    pub fn with(self, child: impl std::hash::Hash) -> Self {
        let mut hasher = new_hasher();
        hasher.write_u64(self.0.get());
        child.hash(&mut hasher);
        Self::from_raw(NonZeroU64::new(hasher.finish()).unwrap())
    }

    pub fn from_raw(raw: NonZeroU64) -> Self {
        Self(raw, PhantomData)
    }

    pub fn transmute<U>(self) -> Id<U> {
        Id::from_raw(self.0)
    }

    /// An arbitrary `Id`. Guaranteed to be unique, unless a hash collision happens
    pub fn arbitrary() -> Self {
        log::info!(
            "counter is {}",
            COUNTER.load(std::sync::atomic::Ordering::Relaxed)
        );
        Self::new(
            NonZeroU64::new(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)).unwrap(),
        )
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Id::<{}>({:#x})",
            std::any::type_name::<T>(),
            self.0
        ))
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> From<Id<T>> for egui::Id {
    fn from(value: Id<T>) -> Self {
        egui::Id::new(value.0)
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}

// Idea taken from the `nohash_hasher` crate.
#[derive(Default)]
pub struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u8(&mut self, _n: u8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u16(&mut self, _n: u16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u32(&mut self, _n: u32) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn write_usize(&mut self, _n: usize) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i8(&mut self, _n: i8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i16(&mut self, _n: i16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i32(&mut self, _n: i32) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i64(&mut self, _n: i64) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_isize(&mut self, _n: isize) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Default)]

pub struct BuildIdHasher;

impl std::hash::BuildHasher for BuildIdHasher {
    type Hasher = IdHasher;

    #[inline(always)]
    fn build_hasher(&self) -> IdHasher {
        IdHasher::default()
    }
}

/// `IdSet<T>` is a `HashSet<Id<T>>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet<T> = std::collections::HashSet<Id<T>, BuildIdHasher>;

/// `IdMap<T, U>` is a `HashMap<Id<T>, U>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<T, U> = std::collections::HashMap<Id<T>, U, BuildIdHasher>;

pub trait IdCorrespondenceMap<T> {
    fn id_get(&self, id: Id<T>) -> &T;
    fn id_get_mut(&mut self, id: Id<T>) -> &mut T;
    fn id_set(&mut self, id: Id<T>, value: T) -> &mut T;
    fn id_has(&self, id: Id<T>) -> bool;
}

impl<T> IdCorrespondenceMap<T> for IdMap<T, T> {
    fn id_get(&self, id: Id<T>) -> &T {
        self.get(&id)
            .unwrap_or_else(|| panic!("invalid Id: {:?}", id))
    }
    fn id_get_mut(&mut self, id: Id<T>) -> &mut T {
        self.get_mut(&id)
            .unwrap_or_else(|| panic!("invalid Id: {:?}", id))
    }
    fn id_set(&mut self, id: Id<T>, value: T) -> &mut T {
        self.entry(id).insert_entry(value).into_mut()
    }
    fn id_has(&self, id: Id<T>) -> bool {
        self.contains_key(&id)
    }
}
