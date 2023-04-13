use tucan::{gc, Intern};

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

    assert_eq!(a.strong_count(), 3);
    assert_eq!(b.strong_count(), 3);
    assert_eq!(c.strong_count(), 2);

    let aa = a.clone();
    let bb = b.clone();
    let cb = c.clone();

    assert_eq!(a.strong_count(), 5);
    assert_eq!(b.strong_count(), 5);
    assert_eq!(c.strong_count(), 3);

    drop(aa);
    drop(bb);
    drop(cb);

    assert_eq!(a.strong_count(), 3);
    assert_eq!(b.strong_count(), 3);
    assert_eq!(c.strong_count(), 2);

    drop(a);

    assert_eq!(b.strong_count(), 2);
    assert_eq!(c.strong_count(), 2);

    drop(b);
    drop(c);

    assert_eq!(tucan::len(), 2);

    gc();

    assert_eq!(tucan::len(), 0);
}
