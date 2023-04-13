use std::{
    fmt::Debug,
    hash::{BuildHasherDefault, Hash},
    ops::Deref,
    ptr::addr_of,
    rc::Rc, collections::HashMap, cell::RefCell,
};

use siphasher::sip128::{Hasher128, SipHasher13};
use twox_hash::XxHash64;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<XxHash64>>;

/// A unique ID for a value within the interner.
pub struct Interned<T: Hash + ?Sized>(Rc<T>);

pub trait Intern<T: Hash + ?Sized = Self>: Hash {
    fn intern(self, interner: &Tucan<T>) -> Interned<T>;
}

impl Intern<str> for &str {
    fn intern(self, interner: &Tucan<str>) -> Interned<str> {
        interner.intern_str(self)
    }
}

default impl<T> Intern for T
where
    T: Hash + Sized,
{
    fn intern(self, interner: &Tucan<Self>) -> Interned<Self> {
        interner.intern(self)
    }
}

impl<T> Clone for Interned<T>
where
    T: Hash + ?Sized,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> Hash for Interned<T>
where
    T: Hash + ?Sized,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> Interned<T>
where
    T: Hash + ?Sized,
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
    T: Hash + ?Sized + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Interned").field(&self.as_ref()).finish()
    }
}

impl<T> AsRef<T> for Interned<T>
where
    T: Hash + ?Sized,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for Interned<T>
where
    T: Hash + ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> PartialEq for Interned<T>
where
    T: Hash + ?Sized,
{
    #[allow(clippy::ptr_eq /* false positive; suggestion loop with vtable_address_comparisons */)]
    fn eq(&self, other: &Self) -> bool {
        addr_of!(*self.0).cast::<()>() == addr_of!(*other.0).cast::<()>()
    }
}

impl<T> PartialEq<T> for Interned<T>
where
    T: Hash + ?Sized + PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<&str> for Interned<str>
{
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl<T: Hash + Sized + PartialEq> PartialEq<&[T]> for Interned<[T]>
{
    fn eq(&self, other: &&[T]) -> bool {
        self.as_ref() == *other
    }
}

impl<T> PartialOrd for Interned<T>
where
    T: Hash + ?Sized + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<T> PartialOrd<T> for Interned<T>
where
    T: Hash + ?Sized + PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other)
    }
}

pub struct Tucan<T: Hash + ?Sized>(RefCell<Map<u128, Rc<T>>>);

impl<T: Hash + ?Sized> Default for Tucan<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Hash + ?Sized> Tucan<T> {
    /// Creates a new interner.
    #[must_use]
    pub fn new() -> Self {
        Self(RefCell::new(Map::default()))
    }

    /// Cleans up the values that are interned but no longer referenced.
    pub fn gc(&self) {
        self.0.borrow_mut().retain(|_, item| Rc::strong_count(item) > 1);
    }

    /// Clears the interner but does not free the memory.
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    /// Returns the number of values interned.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Returns `true` if the interner is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    /// Interns a value.
    pub fn intern(&self, value: T) -> Interned<T>
    where
        T: Sized,
    {
        let hash = hash128(&value);

        let borrow = self.0.borrow();
        if let Some(item) = borrow.get(&hash) {
            Interned(Rc::clone(item))
        } else {
            drop(borrow);

            let ptr: Rc<T> = Rc::new(value);
            self.0.borrow_mut().insert(hash, Rc::clone(&ptr));
            Interned(ptr)
        }
    }
}

impl Tucan<str> {
    /// Interns a string.
    #[must_use]
    pub fn intern_str(&self, value: &str) -> Interned<str> {
        let hash = hash128(&value);

        let borrow = self.0.borrow();
        if let Some(item) = borrow.get(&hash) {
            Interned(Rc::clone(item))
        } else {
            drop(borrow);

            let ptr: Rc<str> = Rc::from(value);
            self.0.borrow_mut().insert(hash, Rc::clone(&ptr));
            Interned(ptr)
        }
    }
}

impl<T: Sized + Hash + Clone> Tucan<[T]> {
    /// Interns a slice.
    pub fn intern_slice(&self, value: &[T]) -> Interned<[T]> {
        let hash = hash128(&value);

        let borrow = self.0.borrow();
        if let Some(item) = borrow.get(&hash) {
            Interned(Rc::clone(item))
        } else {
            drop(borrow);

            let ptr: Rc<[T]> = Rc::<[T]>::from(value);
            self.0.borrow_mut().insert(hash, Rc::clone(&ptr));
            Interned(ptr)
        }
    }
}

fn hash128<T: Hash>(value: &T) -> u128 {
    let mut hasher = SipHasher13::new();
    value.hash(&mut hasher);
    hasher.finish128().as_u128()
}
