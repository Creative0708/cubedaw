use std::{
    collections::hash_map,
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    num::{NonZero, NonZeroU64},
    thread::ThreadId,
};

use ahash::{AHasher, HashMap, HashSet, RandomState};

fn new_hasher() -> AHasher {
    static RANDOM_STATE: std::sync::LazyLock<RandomState> =
        std::sync::LazyLock::new(RandomState::new);

    RANDOM_STATE.build_hasher()
}

// Due to this being a u64, birthday attacks are _technically_ possible but fairly unlikely.
// If this becomes an issue at any point in the future, switch this to a u128.
type IdInner = u64;

// The <T> is used to prevent accidental misuse of an Id<whatever> as an Id<something else>.
// This is definitely _not_ what generics are meant to be used for, but it's convenient soooo......
// Also this may result in unneeded generic impls for stuff like IdMap. :shrug:
#[repr(transparent)]
// pointer because... reasons
pub struct Id<T = ()>(NonZero<IdInner>, PhantomData<*const T>);

// SAFETY: Id<T> doesn't actually store a T, it's a NonZero<u64> which is Send and Sync
unsafe impl<T> Send for Id<T> {}
unsafe impl<T> Sync for Id<T> {}

fn new_impl(source: impl Hash) -> NonZero<IdInner> {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    NonZero::<IdInner>::new(hasher.finish()).expect("hash collision to 0")
}

fn with_impl(source: NonZero<IdInner>, child: impl Hash) -> NonZero<IdInner> {
    let mut hasher = new_hasher();
    source.hash(&mut hasher);
    child.hash(&mut hasher);
    NonZero::<IdInner>::new(hasher.finish()).expect("hash collision to 0")
}

fn arbitrary_impl() -> NonZero<IdInner> {
    use std::cell::Cell;
    thread_local! {
        // std::thread::current().id() isn't cached for some reason sooooo
        // (change this if/when the id is changed to be cached)
        static THREAD_ID: ThreadId = std::thread::current().id();
        static COUNTER: Cell<u64> = const { Cell::new(0) };
    }

    let val = COUNTER.with(|cell| {
        let val = cell.get() + 1;
        cell.set(val);
        val
    });
    THREAD_ID.with(|id| new_impl((val, id)))
}

impl<T> Id<T> {
    pub fn invalid() -> Self {
        // Random number. Most likely won't be equal to anything, ever.
        Self::from_raw(NonZeroU64::new(0x4fccc597ae63b037).unwrap())
    }

    pub const fn from_raw(raw: NonZero<IdInner>) -> Self {
        Self(raw, PhantomData)
    }
    /// Convenience function.
    pub const fn from_raw_or_panic(raw: IdInner) -> Self {
        Self::from_raw(match NonZero::new(raw) {
            Some(inner) => inner,
            None => panic!("zero passed to Id::from_raw_or_panic"),
        })
    }
    pub const fn raw(self) -> NonZero<IdInner> {
        self.0
    }

    pub fn new(source: impl Hash) -> Self {
        Self::from_raw(new_impl(source))
    }

    pub fn with(self, child: impl Hash) -> Self {
        Id::from_raw(with_impl(self.raw(), child))
    }

    /// Creates an arbitrary `Id<T>`. This is guaranteed to be unique across _all_ threads (unless there's a collision).
    pub fn arbitrary() -> Self {
        Self::from_raw(arbitrary_impl())
    }

    /// Casts the `Id<T>` into an `Id<U>`, preserving the value.
    pub const fn cast<U>(self) -> Id<U> {
        Id::from_raw(self.raw())
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // // TODO: replace when impl specialization stabilizes
        // // https://rust-lang.github.io/rfcs/1210-impl-specialization.html
        // use std::any::Any;
        // if let Some(&key_id) = (self as &dyn Any).downcast_ref::<Id<crate::ResourceKey>>() {
        //     static DEFAULT_REGISTRY: std::sync::LazyLock<crate::NodeRegistry> =
        //         std::sync::LazyLock::new(crate::NodeRegistry::new);

        //     return match DEFAULT_REGISTRY.get(key_id) {
        //         None => f.write_str("Id(<invalid ResourceKey>)"),
        //         Some(entry) => write!(f, "Id({:?})", entry.key),
        //     };
        // }

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
pub struct IdMap<T, V = T> {
    // use an Id<()> to not generate impls of HashMap for separate Id<T>s
    map: HashMap<Id, V>,
    _marker: PhantomData<Id<T>>,
}

impl<T, V> IdMap<T, V> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
            _marker: PhantomData,
        }
    }

    pub fn has(&self, id: Id<T>) -> bool {
        self.map.contains_key(&id.cast())
    }

    pub fn get(&self, id: Id<T>) -> Option<&V> {
        self.map.get(&id.cast())
    }
    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut V> {
        self.map.get_mut(&id.cast())
    }
    pub fn get_mut_or_insert(&mut self, id: Id<T>, f: impl FnOnce() -> V) -> &mut V {
        self.map.entry(id.cast()).or_insert_with(f)
    }
    pub fn get_mut_or_insert_default(&mut self, id: Id<T>) -> &mut V
    where
        V: Default,
    {
        self.get_mut_or_insert(id, Default::default)
    }
    pub fn get_mut_or_default(&mut self, id: Id<T>) -> &mut V
    where
        V: Default,
    {
        self.map.entry(id.cast()).or_default()
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
        if self.map.insert(id.cast(), val).is_some() {
            panic!("tried to insert already existing id into IdMap");
        }
    }
    pub fn replace(&mut self, id: Id<T>, val: V) -> Option<V> {
        self.map.insert(id.cast(), val)
    }
    pub fn insert_and_get_mut(&mut self, id: Id<T>, val: V) -> &mut V {
        // TODO currently there's no way to not have two hashmap accesses, change this when entry_insert is stabilized
        self.insert(id, val);
        self.get_mut(id).unwrap_or_else(|| unreachable!())
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<V> {
        self.map.remove(&id.cast())
    }
    pub fn remove_or_default(&mut self, id: Id<T>) -> V
    where
        V: Default,
    {
        self.remove(id).unwrap_or_default()
    }
    pub fn take(&mut self, id: Id<T>) -> V {
        self.remove(id)
            .unwrap_or_else(|| panic!("nonexistent id: {id:?}"))
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }

    // TODO make these functions give Id<T> instead of &Id<T>
    pub fn keys(&self) -> impl Iterator<Item = Id<T>> + '_ {
        self.map.keys().map(|id| id.cast())
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.map.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.map.values_mut()
    }
    pub fn iter(&self) -> Iter<'_, T, V> {
        Iter(self.map.iter(), PhantomData)
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T, V> {
        IterMut(self.map.iter_mut(), PhantomData)
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

impl<T, V: Debug> Debug for IdMap<T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}

// TODO make this a wrapper type
pub type IdSet<T> = HashSet<Id<T>>;

pub struct IntoIter<T, V>(hash_map::IntoIter<Id, V>, PhantomData<Id<T>>);
impl<T, V> Iterator for IntoIter<T, V> {
    type Item = (Id<T>, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, v)| (id.cast(), v))
    }
}
impl<T, V> IntoIterator for IdMap<T, V> {
    type IntoIter = IntoIter<T, V>;
    type Item = (Id<T>, V);

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.map.into_iter(), PhantomData)
    }
}
pub struct Iter<'a, T, V>(hash_map::Iter<'a, Id, V>, PhantomData<Id<T>>);
impl<'a, T, V> Iterator for Iter<'a, T, V> {
    type Item = (Id<T>, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, v)| (id.cast(), v))
    }
}
impl<'a, T, V> IntoIterator for &'a IdMap<T, V> {
    type IntoIter = Iter<'a, T, V>;
    type Item = (Id<T>, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
pub struct IterMut<'a, T, V>(hash_map::IterMut<'a, Id, V>, PhantomData<Id<T>>);
impl<'a, T, V> Iterator for IterMut<'a, T, V> {
    type Item = (Id<T>, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, v)| (id.cast(), v))
    }
}
impl<'a, T, V> IntoIterator for &'a mut IdMap<T, V> {
    type IntoIter = IterMut<'a, T, V>;
    type Item = (Id<T>, &'a mut V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// utilities for specific cases
impl IdMap<crate::Track> {
    pub fn force_get_section(&self, id: Id<crate::Track>) -> &crate::SectionTrack {
        self.force_get(id)
            .inner
            .section()
            .expect("expected section track")
    }
    pub fn force_get_section_mut(&mut self, id: Id<crate::Track>) -> &mut crate::SectionTrack {
        self.force_get_mut(id)
            .inner
            .section_mut()
            .expect("expected section track")
    }
}
