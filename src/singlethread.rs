use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasherDefault, Hash},
    marker::PhantomData,
    ops::Deref,
    ptr::addr_of,
    rc::Rc,
};

use siphasher::sip128::{Hasher128, SipHasher13};
use twox_hash::XxHash64;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<XxHash64>>;

thread_local! {
    static TUCAN: Tucan = Tucan::new();
}

/// A unique ID for a value within the interner.
#[derive(Clone)]
pub struct Interned<T: Intern>(Rc<dyn Any>, PhantomData<T>);

pub trait Intern: Any + Hash + Sized {
    fn intern(self) -> Interned<Self>;
}

impl<T> Intern for T
where
    T: Any + Hash + Sized,
{
    fn intern(self) -> Interned<Self> {
        intern(self)
    }
}

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
    #[inline]
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        Rc::strong_count(&this.0)
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
        if cfg!(debug_assertions) {
            self.0
                .downcast_ref()
                .unwrap_or_else(|| unreachable!("wrong type in dyn Any"))
        } else {
            // SAFETY: we know that the `Arc<dyn Any>` is actually an `Arc<T>`.
            unsafe { &*addr_of!(self.0).cast::<T>() }
        }
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
    #[allow(clippy::ptr_eq /* false positive; suggestion loop with vtable_address_comparisons */)]
    fn eq(&self, other: &Self) -> bool {
        addr_of!(*self.0).cast::<()>() == addr_of!(*other.0).cast::<()>()
    }
}

impl<T> Eq for Interned<T> where T: Intern + Eq {}

impl<T> PartialEq<T> for Interned<T>
where
    T: Intern + PartialEq,
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

impl<T> Ord for Interned<T>
where
    T: Intern + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
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

struct Tucan(RefCell<Map<(TypeId, u128), Rc<dyn Any>>>);

impl Tucan {
    /// Creates a new interner.
    fn new() -> Self {
        Self(RefCell::new(Map::default()))
    }

    /// Cleans up the values that are interned but no longer referenced.
    #[inline]
    fn gc(&self) {
        self.0
            .borrow_mut()
            .retain(|_, item| Rc::strong_count(item) > 1);
    }

    /// Clears the interner but does not free the memory.
    #[inline]
    fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    /// Returns the number of values interned.
    #[inline]
    #[must_use]
    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Interns a value.
    #[must_use]
    fn intern<T>(&self, value: T) -> Interned<T>
    where
        T: Intern,
    {
        let type_id = TypeId::of::<T>();
        let hash = hash128(&value);

        let borrow = self.0.borrow();
        if let Some(item) = borrow.get(&(type_id, hash)) {
            Interned(Rc::clone(item), PhantomData::<T>)
        } else {
            drop(borrow);
            let ptr: Rc<dyn Any> = Rc::new(value);
            self.0.borrow_mut().insert((type_id, hash), Rc::clone(&ptr));
            Interned(ptr, PhantomData)
        }
    }
}

/// Cleans up the values that are interned but no longer referenced.
pub fn gc() {
    TUCAN.with(Tucan::gc);
}

/// Clears the interner but does not free the memory.
pub fn clear() {
    TUCAN.with(Tucan::clear);
}

/// Returns the number of values interned.
#[must_use]
pub fn len() -> usize {
    TUCAN.with(Tucan::len)
}

/// Interns a value.
pub fn intern<T>(value: T) -> Interned<T>
where
    T: Intern,
{
    TUCAN.with(|tucan| tucan.intern(value))
}

fn hash128<T: Hash>(value: &T) -> u128 {
    let mut hasher = SipHasher13::new();
    value.hash(&mut hasher);
    hasher.finish128().as_u128()
}
