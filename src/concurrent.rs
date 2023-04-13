use std::{
    any::{Any, TypeId},
    fmt::Debug,
    hash::{BuildHasherDefault, Hash},
    ops::Deref,
    ptr::addr_of,
    sync::Arc,
};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use siphasher::sip128::{Hasher128, SipHasher13};
use twox_hash::XxHash64;

type Map<K, V> = DashMap<K, V, BuildHasherDefault<XxHash64>>;

static TUCAN: Lazy<Tucan> = Lazy::new(Tucan::new);

/// A unique ID for a value within the interner.
#[derive(Clone)]
pub struct AInterned<T: Hash + Send + Sync + ?Sized>(Arc<T>);

pub trait ConcurrentIntern: Any + Hash + Send + Sync {
    fn intern(self) -> AInterned<Self>;
}

impl<T> ConcurrentIntern for T
where
    T: Any + Hash + Send + Sync + Sized,
{
    fn intern(self) -> AInterned<Self> {
        concurrent_intern(self)
    }
}

impl<T> Hash for AInterned<T>
where
    T: Any + Hash + Send + Sync + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> AInterned<T>
where
    T: Any + Hash + Send + Sync,
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
    T: Any + Hash + Send + Sync + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Interned").field(&self.as_ref()).finish()
    }
}

impl<T> AsRef<T> for AInterned<T>
where
    T: Any + Hash + Send + Sync,
{
    fn as_ref(&self) -> &T {
        self.0.deref()
    }
}

impl<T> Deref for AInterned<T>
where
    T: Any + Hash + Send + Sync,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> PartialEq for AInterned<T>
where
    T: Any + Hash + Send + Sync,
{
    #[allow(clippy::ptr_eq /* false positive; suggestion loop with vtable_address_comparisons */)]
    fn eq(&self, other: &Self) -> bool {
        addr_of!(*self.0).cast::<()>() == addr_of!(*other.0).cast::<()>()
    }
}

impl<T> PartialEq<T> for AInterned<T>
where
    T: Any + Hash + Send + Sync + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other
    }
}

impl<T> PartialOrd for AInterned<T>
where
    T: Any + Hash + Send + Sync + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<T> PartialOrd<T> for AInterned<T>
where
    T: Any + Hash + Send + Sync + PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other)
    }
}

struct Tucan(Map<(TypeId, u128), Arc<dyn Any + Send + Sync>>);

impl Tucan {
    /// Creates a new interner.
    fn new() -> Self {
        Self(Map::default())
    }

    /// Cleans up the values that are interned but no longer referenced.
    fn gc(&self) {
        self.0.retain(|_, item| Arc::strong_count(item) > 1);
    }

    /// Interns a value.
    fn intern<T>(&self, value: T) -> AInterned<T>
    where
        T: ConcurrentIntern,
    {
        let type_id = TypeId::of::<T>();
        let hash = hash128(&value);

        if let Some(item) = self.0.get(&(type_id, hash)) {
            AInterned(Arc::clone(item.value()).downcast().unwrap())
        } else {
            let ptr: Arc<T> = Arc::new(value);
            self.0.insert((type_id, hash), Arc::clone(&ptr) as Arc<dyn Any + Send + Sync>);
            AInterned(ptr)
        }
    }
}

/// Cleans up the values that are interned but no longer referenced.
pub fn concurrent_gc() {
    TUCAN.gc();
}

/// Clears the interner but does not free the memory.
pub fn concurrent_clear() {
    TUCAN.0.clear();
}

/// Returns the number of values interned.
#[must_use]
pub fn concurrent_len() -> usize {
    TUCAN.0.len()
}

/// Interns a value.
pub fn concurrent_intern<T>(value: T) -> AInterned<T>
where
    T: ConcurrentIntern,
{
    TUCAN.intern(value)
}

fn hash128<T: Hash>(value: &T) -> u128 {
    let mut hasher = SipHasher13::new();
    value.hash(&mut hasher);
    hasher.finish128().as_u128()
}
