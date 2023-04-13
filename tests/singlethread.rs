use tucan::{Intern, Interned, Tucan};

#[test]
pub fn test_interner() {
    let interner = Tucan::<str>::new();
    
    let a = "hello".intern(&interner);
    let b = "hello".intern(&interner);
    let c = "world".intern(&interner);

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);

    assert_eq!(a, "hello");
    assert_eq!(b, "hello");
    assert_eq!(c, "world");

    assert_eq!(a, "hello".intern(&interner));
    assert_eq!(b, "hello".intern(&interner));
    assert_eq!(c, "world".intern(&interner));

    assert_eq!(Interned::strong_count(&a), 3);
    assert_eq!(Interned::strong_count(&b), 3);
    assert_eq!(Interned::strong_count(&c), 2);

    let aa = a.clone();
    let bb = b.clone();
    let cb = c.clone();

    assert_eq!(Interned::strong_count(&a), 5);
    assert_eq!(Interned::strong_count(&b), 5);
    assert_eq!(Interned::strong_count(&c), 3);

    drop(aa);
    drop(bb);
    drop(cb);

    assert_eq!(Interned::strong_count(&a), 3);
    assert_eq!(Interned::strong_count(&b), 3);
    assert_eq!(Interned::strong_count(&c), 2);

    drop(a);

    assert_eq!(Interned::strong_count(&b), 2);
    assert_eq!(Interned::strong_count(&c), 2);

    drop(b);
    drop(c);

    assert_eq!(interner.len(), 2);

    interner.gc();

    assert_eq!(interner.len(), 0);
}
