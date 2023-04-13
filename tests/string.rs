use tucan::{gc, Intern, Interned};

#[test]
pub fn test_interner() {
    let a = "hello".intern();
    let b = "hello".intern();
    let c = "world".intern();

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);

    assert_eq!(a, "hello");
    assert_eq!(b, "hello");
    assert_eq!(c, "world");

    assert_eq!(a, "hello".intern());
    assert_eq!(b, "hello".intern());
    assert_eq!(c, "world".intern());

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

    assert_eq!(tucan::len(), 2);

    gc();

    assert_eq!(tucan::len(), 0);
}
