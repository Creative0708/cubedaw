use std::{
    cell::RefCell,
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

use ahash::{AHasher, HashMap, HashMapExt, HashSet, RandomState};

fn new_hasher() -> AHasher {
    // `printf cubedaw | sha256sum`
    RandomState::with_seeds(
        0x7631_a132_54ea_fc47,
        0xff44_f51d_93bf_9bb0,
        0x4d08_8811_297a_42f4,
        0x2367_3b90_f1e9_b53c,
    )
    .build_hasher()
}

std::thread_local! {
    static COUNTER: RefCell<IdInner> = RefCell::new(0);
}

// Due to this being a u64, birthday attacks are _technically_ possible but fairly unlikely.
// If this becomes an issue at any point in the future, switch this to a u128.
type IdInner = u64;

// The <T> is used to prevent accidental misuse of an Id<whatever> as an Id<something else>.
// This is definitely _not_ what generics are meant to be used for, but it's convenient soooo......
#[repr(transparent)]
pub struct Id<T>(IdInner, PhantomData<T>);

fn new_impl(source: impl Hash) -> IdInner {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    hasher.finish()
}

fn with_impl(source: IdInner, child: impl Hash) -> IdInner {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    child.hash(&mut hasher);
    hasher.finish()
}

impl<T> Id<T> {
    pub fn from_raw(raw: IdInner) -> Self {
        Self(raw, PhantomData)
    }
    pub fn raw(self) -> IdInner {
        self.0
    }

    pub fn new(source: impl Hash) -> Self {
        Self::from_raw(new_impl(source))
    }

    pub fn with<U>(self, child: impl Hash) -> Id<U> {
        Id::<U>::from_raw(with_impl(self.raw(), child))
    }

    pub fn arbitrary() -> Self {
        Self::new(COUNTER.with(|x| {
            let mut x = x.borrow_mut();
            *x += 1;
            *x
        }))
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

#[cfg(feature = "egui")]
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

// TODO if these are a performance bottleneck copy egui's id hasher implementation
// pub type IdMap<K, V = K> = HashMap<Id<K>, V>;

#[derive(Debug)]
pub struct IdMap<T> {
    map: HashMap<Id<T>, T>,
}

use std::collections::hash_map::{Iter, IterMut};

impl<T> IdMap<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, id: Id<T>) -> &T {
        self.map
            .get(&id)
            .unwrap_or_else(|| panic!("Invalid id for State::get: {id:?}"))
    }
    pub fn get_mut(&mut self, id: Id<T>) -> &mut T {
        self.map
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Invalid id for State::get_mut: {id:?}"))
    }
    pub fn set_mut(&mut self, id: Id<T>, val: T) -> Option<T> {
        self.map.insert(id, val)
    }
    pub fn create(&mut self, val: T) -> Id<T> {
        let id = Id::arbitrary();
        self.set_mut(id, val);
        id
    }

    pub fn iter(&self) -> Iter<'_, Id<T>, T> {
        self.map.iter()
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, Id<T>, T> {
        self.map.iter_mut()
    }
}

impl<T> Default for IdMap<T> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

pub type IdSet<T> = HashSet<Id<T>>;
