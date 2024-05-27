use std::{
    cell::RefCell,
    collections::hash_map,
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

use ahash::{AHasher, HashMap, HashSet, RandomState};

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

// Due to this being a u64, birthday attacks are _technically_ possible but fairly unlikely.
// If this becomes an issue at any point in the future, switch this to a u128.
type IdInner = u64;

// The <T> is used to prevent accidental misuse of an Id<whatever> as an Id<something else>.
// This is definitely _not_ what generics are meant to be used for, but it's convenient soooo......
// Also this may result in unneeded generic impls for stuff like IdMap. :shrug:
#[repr(transparent)]
pub struct Id<T = ()>(IdInner, PhantomData<T>);

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

fn arbitrary_impl() -> IdInner {
    std::thread_local! {
        static COUNTER: RefCell<IdInner> = const { RefCell::new(0) };
    }

    COUNTER.with(|x| {
        let mut x = x.borrow_mut();
        *x += 1;
        *x
    })
}

impl<T> Id<T> {
    pub const fn zero() -> Self {
        Self::from_raw(0)
    }

    pub const fn invalid() -> Self {
        // Random number. Most likely won't be equal to anything, ever.
        Self::from_raw(0x4fccc597ae63b037)
    }

    pub const fn from_raw(raw: IdInner) -> Self {
        Self(raw, PhantomData)
    }
    pub const fn raw(self) -> IdInner {
        self.0
    }

    pub fn new(source: impl Hash) -> Self {
        Self::from_raw(new_impl(source))
    }

    pub fn with<U>(self, child: impl Hash) -> Id<U> {
        Id::<U>::from_raw(with_impl(self.raw(), child))
    }

    pub fn arbitrary() -> Self {
        Self::new(arbitrary_impl())
    }

    pub const fn transmute<U>(self) -> Id<U> {
        Id::from_raw(self.raw())
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

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

// TODO if these are a performance bottleneck copy egui's id hasher implementation
pub struct IdMap<T, V = T> {
    map: HashMap<Id<T>, V>,
    // TODO revise the entire "tracking" thing
    // events: Option<Vec<TrackingMapEvent<T>>>,
}

impl<T, V> IdMap<T, V> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
            // events: Some(Default::default()),
        }
    }

    pub fn has(&self, id: Id<T>) -> bool {
        self.map.contains_key(&id)
    }

    pub fn get(&self, id: Id<T>) -> Option<&V> {
        self.map.get(&id)
    }
    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut V> {
        self.map.get_mut(&id)
    }
    pub fn get_mut_or_default(&mut self, id: Id<T>) -> &mut V
    where
        V: Default,
    {
        self.map
            .entry(id)
            .and_modify(|_| panic!("hash collision in IdMap"))
            .or_default()
    }
    pub fn insert(&mut self, id: Id<T>, val: V) {
        // if let Some(ref mut events) = self.events {
        //     events.push(TrackingMapEvent::Create(id));
        // }
        if self.map.insert(id, val).is_some() {
            panic!("tried to insert already existing id into IdMap");
        }
    }
    pub fn insert_and_get_mut(&mut self, id: Id<T>, val: V) -> &mut V {
        // TODO currently there's no way to not have two hashmap accesses, change this when entry_insert is stabilized
        self.insert(id, val);
        self.get_mut(id).unwrap_or_else(|| unreachable!())
    }
    // pub fn create(&mut self, val: V) -> Id<T> {
    //     let id = Id::arbitrary();
    //     self.set(id, val);
    //     id
    // }
    // pub fn create_and_get_mut(&mut self, val: V) -> (Id<T>, &mut V) {
    //     let id = Id::arbitrary();
    //     (
    //         id,
    //         self.map
    //             .entry(id)
    //             .and_modify(|_| panic!("hash collision in IdMap"))
    //             .or_insert(val),
    //     )
    // }

    pub fn remove(&mut self, id: Id<T>) -> Option<V> {
        // if let Some(ref mut events) = self.events {
        //     events.push(TrackingMapEvent::Delete(id));
        // }
        self.map.remove(&id)
    }
    pub fn take(&mut self, id: Id<T>) -> V {
        self.remove(id)
            .unwrap_or_else(|| panic!("nonexistent id: {id:?}"))
    }

    pub fn keys(&self) -> hash_map::Keys<'_, Id<T>, V> {
        self.map.keys()
    }
    pub fn iter(&self) -> hash_map::Iter<'_, Id<T>, V> {
        self.map.iter()
    }
    pub fn iter_mut(&mut self) -> hash_map::IterMut<'_, Id<T>, V> {
        self.map.iter_mut()
    }

    // pub fn track(&mut self, tracking: &IdMap<T>) {
    //     for event in tracking
    //         .events()
    //         .expect("IdMap::track called with non-tracking map")
    //     {
    //         match event {
    //             TrackingMapEvent::Delete(id) => {
    //                 assert!(self.remove(*id).is_some());
    //             }
    //             _ => (),
    //         }
    //     }
    // }

    // pub fn events(&self) -> Option<&[TrackingMapEvent<T>]> {
    //     self.events.as_deref()
    // }
    // pub fn clear_events(&mut self) {
    //     let Some(ref mut events) = self.events else {
    //         panic!("IdMap::clear_events called on non-tracking map");
    //     };
    //     events.clear();
    // }
}

impl<T, V> Default for IdMap<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, V> IntoIterator for IdMap<T, V> {
    type IntoIter = std::collections::hash_map::IntoIter<Id<T>, V>;
    type Item = (Id<T>, V);

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}
impl<T, V: Debug> Debug for IdMap<T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}

pub type IdSet<T> = HashSet<Id<T>>;
