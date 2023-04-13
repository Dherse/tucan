#![feature(downcast_unchecked)]

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasherDefault, Hash},
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
};

use parking_lot::RwLock;
use siphasher::sip128::{Hasher128, SipHasher13};
use twox_hash::XxHash64;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<XxHash64>>;

lazy_static::lazy_static! {
    static ref TUCAN: Tucan = Tucan::new();
}

/// A unique ID for a value within the interner.
///
///
#[derive(Clone)]
pub struct Interned<T: Intern>(Arc<dyn Any>, PhantomData<T>);

unsafe impl<T> Send for Interned<T> where T: Intern {}
unsafe impl<T> Sync for Interned<T> where T: Intern {}

impl<T> Hash for Interned<T>
where
    T: Intern + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> Interned<T>
where
    T: Intern,
{
    /// Returns the number of strong references to this value.
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }
}

impl<T> Debug for Interned<T>
where
    T: Intern + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Interned").field(&self.as_ref()).finish()
    }
}

impl<T> AsRef<T> for Interned<T>
where
    T: Intern,
{
    fn as_ref(&self) -> &T {
        // Safety: we know that the `Arc<dyn Any>` is actually an `Arc<T>`.
        unsafe { self.0.downcast_ref_unchecked::<T>() }
    }
}

impl<T> Deref for Interned<T>
where
    T: Intern,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> PartialEq for Interned<T>
where
    T: Intern,
{
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> PartialEq<T> for Interned<T>
where
    T: Intern,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other
    }
}

impl<T> PartialOrd for Interned<T>
where
    T: Intern + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<T> PartialOrd<T> for Interned<T>
where
    T: Intern + PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other)
    }
}

pub trait Intern: Any + Hash + PartialEq + Send + Sync + Sized {
    fn intern(self) -> Interned<Self>;
}

impl<T> Intern for T
where
    T: Any + Hash + PartialEq + Send + Sync + Sized,
{
    fn intern(self) -> Interned<Self> {
        intern(self)
    }
}

struct Tucan(RwLock<Map<(TypeId, u128), Arc<dyn Any>>>);

unsafe impl Sync for Tucan {}
unsafe impl Send for Tucan {}

impl Tucan {
    /// Creates a new interner.
    fn new() -> Self {
        Self(RwLock::new(HashMap::default()))
    }

    /// Cleans up the values that are interned but no longer referenced.
    fn gc(&self) {
        let mut map = self.0.write();
        map.retain(|_, item| Arc::strong_count(item) > 1);
    }

    /// Interns a value.
    fn intern<T>(&self, value: T) -> Interned<T>
    where
        T: Intern,
    {
        let type_id = TypeId::of::<T>();
        let hash = hash128(&value);

        let mut map = self.0.write();
        let Some(item) = map.get(&(type_id, hash)) else {
            let ptr: Arc<dyn Any> = Arc::new(value);
            map.insert((type_id, hash), Arc::clone(&ptr));
            return Interned(ptr, PhantomData);
        };

        Interned(Arc::clone(item), PhantomData)
    }
}

/// Cleans up the values that are interned but no longer referenced.
pub fn gc() {
    TUCAN.gc();
}

/// Clears the interner.
/// Does not free the memory.
pub fn clear() {
    let mut map = TUCAN.0.write();
    map.clear();
}

/// Returns the number of values interned.
pub fn len() -> usize {
    let map = TUCAN.0.read();
    map.len()
}

/// Interns a value.
pub fn intern<T>(value: T) -> Interned<T>
where
    T: Intern,
{
    TUCAN.intern(value)
}

fn hash128<T: Hash>(value: &T) -> u128 {
    let mut hasher = SipHasher13::new();
    value.hash(&mut hasher);
    hasher.finish128().as_u128()
}
