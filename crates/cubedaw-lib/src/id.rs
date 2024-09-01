use std::{
    collections::hash_map,
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
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
type IdInner = NonZeroU64;

// The <T> is used to prevent accidental misuse of an Id<whatever> as an Id<something else>.
// This is definitely _not_ what generics are meant to be used for, but it's convenient soooo......
// Also this may result in unneeded generic impls for stuff like IdMap. :shrug:
#[repr(transparent)]
pub struct Id<T: 'static = ()>(IdInner, PhantomData<T>);

fn new_impl(source: impl Hash) -> IdInner {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    IdInner::new(hasher.finish()).expect("hash collision to 0")
}

fn with_impl(source: IdInner, child: impl Hash) -> IdInner {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    child.hash(&mut hasher);
    IdInner::new(hasher.finish()).expect("hash collision to 0")
}

fn arbitrary_impl() -> IdInner {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    new_impl(counter)
}

impl<T> Id<T> {
    pub fn invalid() -> Self {
        // Random number. Most likely won't be equal to anything, ever.
        Self::from_raw(NonZeroU64::new(0x4fccc597ae63b037).unwrap())
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
        Id::from_raw(with_impl(self.raw(), child))
    }

    pub fn arbitrary() -> Self {
        Self::from_raw(arbitrary_impl())
    }

    pub const fn cast<U>(self) -> Id<U> {
        Id::from_raw(self.raw())
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: replace when impl specialization stabilizes
        // https://rust-lang.github.io/rfcs/1210-impl-specialization.html
        use std::any::Any;
        if let Some(&key_id) = (self as &dyn Any).downcast_ref::<Id<crate::ResourceKey>>() {
            static DEFAULT_REGISTRY: std::sync::LazyLock<crate::NodeRegistry> =
                std::sync::LazyLock::new(crate::NodeRegistry::new);

            return match DEFAULT_REGISTRY.get(key_id) {
                None => f.write_str("Id(<invalid ResourceKey>)"),
                Some(entry) => write!(f, "Id({:?})", entry.key),
            };
        }

        if *self == Id::<T>::invalid() {
            return write!(f, "Id::<{}>(<invalid>)", std::any::type_name::<T>());
        }
        write!(f, "Id::<{}>({:#x})", std::any::type_name::<T>(), self.0)
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
#[cfg(feature = "egui")]
impl<T> From<egui::Id> for Id<T> {
    fn from(value: egui::Id) -> Self {
        Self::new(value)
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
#[derive(Clone)]
pub struct IdMap<T: 'static, V = T> {
    map: HashMap<Id<T>, V>,
}

impl<T, V> IdMap<T, V> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
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
    pub fn get_mut_or_insert(&mut self, id: Id<T>, f: impl FnOnce() -> V) -> &mut V {
        self.map.entry(id).or_insert_with(f)
    }
    pub fn get_mut_or_default(&mut self, id: Id<T>) -> &mut V
    where
        V: Default,
    {
        self.map.entry(id).or_default()
    }
    pub fn force_get(&self, id: Id<T>) -> &V {
        match self.get(id) {
            Some(v) => v,
            None => panic!("Nonexistent id: {id:?}"),
        }
    }
    pub fn force_get_mut(&mut self, id: Id<T>) -> &mut V {
        match self.get_mut(id) {
            Some(v) => v,
            None => panic!("Nonexistent id: {id:?}"),
        }
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

    // TODO make these functions give Id<T> instead of &Id<T>
    pub fn keys(&self) -> hash_map::Keys<'_, Id<T>, V> {
        self.map.keys()
    }
    pub fn values(&self) -> hash_map::Values<'_, Id<T>, V> {
        self.map.values()
    }
    pub fn iter(&self) -> hash_map::Iter<'_, Id<T>, V> {
        self.map.iter()
    }
    pub fn iter_mut(&mut self) -> hash_map::IterMut<'_, Id<T>, V> {
        self.map.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<T, V> Default for IdMap<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, V> IntoIterator for IdMap<T, V> {
    type IntoIter = hash_map::IntoIter<Id<T>, V>;
    type Item = (Id<T>, V);

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}
impl<'a, T, V> IntoIterator for &'a IdMap<T, V> {
    type IntoIter = hash_map::Iter<'a, Id<T>, V>;
    type Item = (&'a Id<T>, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T, V> IntoIterator for &'a mut IdMap<T, V> {
    type IntoIter = hash_map::IterMut<'a, Id<T>, V>;
    type Item = (&'a Id<T>, &'a mut V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
impl<T, V: Debug> Debug for IdMap<T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}

pub type IdSet<T> = HashSet<Id<T>>;
