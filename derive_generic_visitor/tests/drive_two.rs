use derive_generic_visitor::*;

#[derive(Drive, DriveTwo)]
struct Pair {
    x: u64,
    y: u32,
}

#[derive(Drive, DriveTwo)]
enum MyList {
    Empty,
    Cons(u64, Box<MyList>),
}

/// A visitor that checks two trees are equal.
struct EqVisitor;

impl Visitor for EqVisitor {
    type Break = ();
}

impl<'a> VisitTwo<'a, u64> for EqVisitor {
    fn visit(&mut self, a: &'a u64, b: &'a u64) -> ControlFlow<()> {
        if a != b {
            Break(())
        } else {
            Continue(())
        }
    }
}

impl<'a> VisitTwo<'a, u32> for EqVisitor {
    fn visit(&mut self, a: &'a u32, b: &'a u32) -> ControlFlow<()> {
        if a != b {
            Break(())
        } else {
            Continue(())
        }
    }
}

impl<'a> VisitTwo<'a, Box<MyList>> for EqVisitor {
    fn visit(&mut self, a: &'a Box<MyList>, b: &'a Box<MyList>) -> ControlFlow<()> {
        a.drive_two_inner(b, self)
    }
}

impl<'a> VisitTwo<'a, MyList> for EqVisitor {
    fn visit(&mut self, a: &'a MyList, b: &'a MyList) -> ControlFlow<()> {
        a.drive_two_inner(b, self)
    }
}

#[test]
fn test_struct_equal() {
    let a = Pair { x: 1, y: 2 };
    let b = Pair { x: 1, y: 2 };
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

#[test]
fn test_struct_not_equal() {
    let a = Pair { x: 1, y: 2 };
    let b = Pair { x: 1, y: 3 };
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_break());
}

#[test]
fn test_enum_same_variant() {
    let a = MyList::Cons(1, Box::new(MyList::Empty));
    let b = MyList::Cons(1, Box::new(MyList::Empty));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

#[test]
fn test_enum_different_values() {
    let a = MyList::Cons(1, Box::new(MyList::Empty));
    let b = MyList::Cons(2, Box::new(MyList::Empty));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_break());
}

#[test]
fn test_enum_variant_mismatch() {
    let a = MyList::Empty;
    let b = MyList::Cons(1, Box::new(MyList::Empty));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_break());
}

#[test]
fn test_enum_both_empty() {
    let a = MyList::Empty;
    let b = MyList::Empty;
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

#[test]
fn test_enum_deep_equal() {
    let a = MyList::Cons(1, Box::new(MyList::Cons(2, Box::new(MyList::Empty))));
    let b = MyList::Cons(1, Box::new(MyList::Cons(2, Box::new(MyList::Empty))));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

#[test]
fn test_enum_deep_not_equal() {
    let a = MyList::Cons(1, Box::new(MyList::Cons(2, Box::new(MyList::Empty))));
    let b = MyList::Cons(1, Box::new(MyList::Cons(3, Box::new(MyList::Empty))));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_break());
}

#[test]
fn test_enum_different_length() {
    let a = MyList::Cons(1, Box::new(MyList::Cons(2, Box::new(MyList::Empty))));
    let b = MyList::Cons(1, Box::new(MyList::Empty));
    let mut v = EqVisitor;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_break());
}

/// Test with #[drive(skip)] on a field.
#[test]
fn test_skip_field() {
    #[derive(DriveTwo)]
    struct WithSkip {
        val: u64,
        #[drive(skip)]
        #[expect(unused)]
        ignored: String,
    }

    let a = WithSkip {
        val: 42,
        ignored: "hello".into(),
    };
    let b = WithSkip {
        val: 42,
        ignored: "world".into(),
    };
    let mut v = EqVisitor;
    // Should be equal because the String field is skipped.
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

/// Test with tuple struct (unnamed fields).
#[test]
fn test_tuple_struct() {
    #[derive(DriveTwo)]
    struct Tup(u64, u32);

    let a = Tup(1, 2);
    let b = Tup(1, 2);
    let mut v = EqVisitor;
    assert!(a.drive_two_inner(&b, &mut v).is_continue());

    let c = Tup(1, 3);
    let mut v = EqVisitor;
    assert!(a.drive_two_inner(&c, &mut v).is_break());
}

/// Test with generics.
#[test]
fn test_generic_struct() {
    #[derive(DriveTwo)]
    struct Wrapper<T> {
        inner: T,
    }

    let a = Wrapper { inner: 42u64 };
    let b = Wrapper { inner: 42u64 };
    let mut v = EqVisitor;
    assert!(a.drive_two_inner(&b, &mut v).is_continue());

    let c = Wrapper { inner: 99u64 };
    let mut v = EqVisitor;
    assert!(a.drive_two_inner(&c, &mut v).is_break());
}

/// Test with unit struct.
#[test]
fn test_unit_struct() {
    #[derive(DriveTwo)]
    struct Unit;

    let a = Unit;
    let b = Unit;
    let mut v = EqVisitor;
    assert!(a.drive_two_inner(&b, &mut v).is_continue());
}
