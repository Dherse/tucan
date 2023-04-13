# Tucan: a simple interner with garbage collection

Tucan is a very basic interner with garbage collection. It adds an `Intern` trait that allows you to intern any type that implements `Hash`, `Send`, `Sync` and `Eq`. Keys to interned elements are called `Interned` and are wrapper around a `Arc<T>`. They implement `Deref` for `T` and `Clone`. When calling the `gc()` function, tucan looks for all entries in the interner that are not referenced by any `Interned` and removes them.

## Note
Tucan uses [sip-hash](https://docs.rs/siphasher/latest/siphasher/) for checking whether to values are the same. Therefore values with colliding hashes will be interned as the same values if they are of the same type. This means that conflicts are possible. This is a concious design decision to keep the interner simple and fast with very loose requirements on the types that can be interned.