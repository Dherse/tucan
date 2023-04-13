use std::{
    fmt::Debug,
    hash::{BuildHasherDefault, Hash},
    ops::Deref,
    ptr::addr_of,
    sync::Arc,
};

use dashmap::DashMap;
use siphasher::sip128::{Hasher128, SipHasher13};
use twox_hash::XxHash64;

type Map<K, V> = DashMap<K, V, BuildHasherDefault<XxHash64>>;

/// A unique ID for a value within the interner.
pub struct AInterned<T: Hash + Send + Sync + ?Sized>(Arc<T>);

pub trait ConcurrentIntern<T: Hash + Send + Sync + ?Sized = Self>: Hash + Send + Sync {
    fn intern(self, interner: &Tucan<T>) -> AInterned<T>;
}

impl ConcurrentIntern<str> for &str {
    fn intern(self, interner: &Tucan<str>) -> AInterned<str> {
        interner.intern_str(self)
    }
}

default impl<T> ConcurrentIntern for T
where
    T: Hash + Send + Sync + Sized,
{
    fn intern(self, interner: &Tucan<Self>) -> AInterned<Self> {
        interner.intern(self)
    }
}

impl<T> Clone for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Hash for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    /// Returns the number of strong references to this value.
    #[inline]
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        Arc::strong_count(&this.0)
    }
}

impl<T> Debug for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Interned").field(&self.as_ref()).finish()
    }
}

impl<T> AsRef<T> for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> PartialEq for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized,
{
    #[allow(clippy::ptr_eq /* false positive; suggestion loop with vtable_address_comparisons */)]
    fn eq(&self, other: &Self) -> bool {
        addr_of!(*self.0).cast::<()>() == addr_of!(*other.0).cast::<()>()
    }
}

impl<T> PartialEq<T> for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<&str> for AInterned<str>
{
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl<T: Hash + Send + Sync + Sized + PartialEq> PartialEq<&[T]> for AInterned<[T]>
{
    fn eq(&self, other: &&[T]) -> bool {
        self.as_ref() == *other
    }
}

impl<T> PartialOrd for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<T> PartialOrd<T> for AInterned<T>
where
    T: Hash + Send + Sync + ?Sized + PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other)
    }
}

pub struct Tucan<T: Hash + Send + Sync + ?Sized>(Map<u128, Arc<T>>);

impl<T: Hash + Send + Sync + ?Sized> Default for Tucan<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Hash + Send + Sync + ?Sized> Tucan<T> {
    /// Creates a new interner.
    #[must_use]
    pub fn new() -> Self {
        Self(Map::default())
    }

    /// Cleans up the values that are interned but no longer referenced.
    pub fn gc(&self) {
        self.0.retain(|_, item| Arc::strong_count(item) > 1);
    }

    /// Clears the interner but does not free the memory.
    pub fn clear(&self) {
        self.0.clear();
    }

    /// Returns the number of values interned.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the interner is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Interns a value.
    pub fn intern(&self, value: T) -> AInterned<T>
    where
        T: Sized,
    {
        let hash = hash128(&value);

        if let Some(item) = self.0.get(&hash) {
            AInterned(Arc::clone(item.value()))
        } else {
            let ptr: Arc<T> = Arc::new(value);
            self.0.insert(hash, Arc::clone(&ptr));
            AInterned(ptr)
        }
    }
}

impl Tucan<str> {
    /// Interns a string.
    #[must_use]
    pub fn intern_str(&self, value: &str) -> AInterned<str> {
        let hash = hash128(&value);

        if let Some(item) = self.0.get(&hash) {
            AInterned(Arc::clone(item.value()))
        } else {
            let ptr: Arc<str> = Arc::from(value);
            self.0.insert(hash, Arc::clone(&ptr));
            AInterned(ptr)
        }
    }
}

impl<T: Sized + Hash + Clone + Send + Sync> Tucan<[T]> {
    /// Interns a slice.
    pub fn intern_slice(&self, value: &[T]) -> AInterned<[T]> {
        let hash = hash128(&value);

        if let Some(item) = self.0.get(&hash) {
            AInterned(Arc::clone(item.value()))
        } else {
            let ptr: Arc<[T]> = Arc::<[T]>::from(value);
            self.0.insert(hash, Arc::clone(&ptr));
            AInterned(ptr)
        }
    }
}

fn hash128<T: Hash>(value: &T) -> u128 {
    let mut hasher = SipHasher13::new();
    value.hash(&mut hasher);
    hasher.finish128().as_u128()
}
