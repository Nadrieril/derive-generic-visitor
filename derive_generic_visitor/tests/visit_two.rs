use derive_generic_visitor::*;

#[derive(Drive, DriveTwo)]
enum MyList {
    Empty,
    Cons(MyNode),
}

#[derive(Drive, DriveTwo)]
struct MyNode {
    val: String,
    next: Box<MyList>,
}

/// Test `drive` and `skip` kinds.
#[test]
fn test_drive_and_skip() {
    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, MyNode, for<T> Box<T>))]
    #[visit_two(skip(String))]
    struct CountNodes;

    impl Visitor for CountNodes {
        type Break = ();
    }

    let a = MyList::Cons(MyNode {
        val: "a".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "b".into(),
            next: Box::new(MyList::Empty),
        })),
    });
    let b = MyList::Cons(MyNode {
        val: "x".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "y".into(),
            next: Box::new(MyList::Empty),
        })),
    });

    // drive just recurses, skip ignores strings, so this should succeed
    // even though the string values differ.
    let mut v = CountNodes;
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
}

/// Test `enter` kind — calls a method before recursing.
#[test]
fn test_enter() {
    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, for<T> Box<T>))]
    #[visit_two(enter(MyNode))]
    #[visit_two(skip(String))]
    struct PairCollector {
        pairs: Vec<(String, String)>,
    }

    impl Visitor for PairCollector {
        type Break = ();
    }

    impl PairCollector {
        fn enter_my_node(&mut self, a: &MyNode, b: &MyNode) {
            self.pairs.push((a.val.clone(), b.val.clone()));
        }
    }

    let a = MyList::Cons(MyNode {
        val: "hello".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "world".into(),
            next: Box::new(MyList::Empty),
        })),
    });
    let b = MyList::Cons(MyNode {
        val: "foo".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "bar".into(),
            next: Box::new(MyList::Empty),
        })),
    });

    let mut v = PairCollector { pairs: vec![] };
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
    assert_eq!(
        v.pairs,
        vec![
            ("hello".to_string(), "foo".to_string()),
            ("world".to_string(), "bar".to_string()),
        ]
    );
}

/// Test `exit` kind — calls a method after recursing.
#[test]
fn test_exit() {
    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, for<T> Box<T>))]
    #[visit_two(exit(MyNode))]
    #[visit_two(skip(String))]
    struct PostOrderCollector {
        pairs: Vec<(String, String)>,
    }

    impl Visitor for PostOrderCollector {
        type Break = ();
    }

    impl PostOrderCollector {
        fn exit_my_node(&mut self, a: &MyNode, b: &MyNode) {
            self.pairs.push((a.val.clone(), b.val.clone()));
        }
    }

    let a = MyList::Cons(MyNode {
        val: "outer".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "inner".into(),
            next: Box::new(MyList::Empty),
        })),
    });
    let b = MyList::Cons(MyNode {
        val: "A".into(),
        next: Box::new(MyList::Cons(MyNode {
            val: "B".into(),
            next: Box::new(MyList::Empty),
        })),
    });

    let mut v = PostOrderCollector { pairs: vec![] };
    let result = a.drive_two_inner(&b, &mut v);
    assert!(result.is_continue());
    // Post-order: inner node visited first (exit happens after recursing).
    assert_eq!(
        v.pairs,
        vec![
            ("inner".to_string(), "B".to_string()),
            ("outer".to_string(), "A".to_string()),
        ]
    );
}

/// Test `override` kind — calls a custom method that controls recursion.
#[test]
fn test_override() {
    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, for<T> Box<T>))]
    #[visit_two(MyNode)]
    #[visit_two(skip(String))]
    struct MatchChecker {
        matched: bool,
    }

    impl Visitor for MatchChecker {
        type Break = ();
    }

    impl MatchChecker {
        fn visit_my_node(&mut self, a: &MyNode, b: &MyNode) -> ControlFlow<()> {
            if a.val == b.val {
                self.matched = true;
                // Recurse into children.
                a.drive_two_inner(b, self)
            } else {
                Break(())
            }
        }
    }

    let a = MyList::Cons(MyNode {
        val: "same".into(),
        next: Box::new(MyList::Empty),
    });
    let b = MyList::Cons(MyNode {
        val: "same".into(),
        next: Box::new(MyList::Empty),
    });

    let mut v = MatchChecker { matched: false };
    assert!(a.drive_two_inner(&b, &mut v).is_continue());
    assert!(v.matched);

    // Different values: override should break.
    let c = MyList::Cons(MyNode {
        val: "different".into(),
        next: Box::new(MyList::Empty),
    });
    let mut v = MatchChecker { matched: false };
    assert!(a.drive_two_inner(&c, &mut v).is_break());
}

/// Test with a custom method name via `name: Ty` syntax.
#[test]
fn test_named_override() {
    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, for<T> Box<T>))]
    #[visit_two(enter(node: MyNode))]
    #[visit_two(skip(String))]
    struct NamedVisitor {
        count: usize,
    }

    impl Visitor for NamedVisitor {
        type Break = ();
    }

    impl NamedVisitor {
        fn enter_node(&mut self, _a: &MyNode, _b: &MyNode) {
            self.count += 1;
        }
    }

    let a = MyList::Cons(MyNode {
        val: "a".into(),
        next: Box::new(MyList::Empty),
    });
    let b = MyList::Cons(MyNode {
        val: "b".into(),
        next: Box::new(MyList::Empty),
    });

    let mut v = NamedVisitor { count: 0 };
    assert!(a.drive_two_inner(&b, &mut v).is_continue());
    assert_eq!(v.count, 1);
}

/// Test early exit propagation with a non-unit Break type.
#[test]
fn test_early_exit() {
    #[derive(Default)]
    struct Mismatch;

    #[derive(VisitTwo)]
    #[visit_two(drive(MyList, MyNode, for<T> Box<T>))]
    #[visit_two(val: String)]
    struct StringEqVisitor;

    impl Visitor for StringEqVisitor {
        type Break = Mismatch;
    }

    impl StringEqVisitor {
        fn visit_val(&mut self, a: &String, b: &String) -> ControlFlow<Mismatch> {
            if a == b {
                Continue(())
            } else {
                Break(Mismatch)
            }
        }
    }

    let a = MyList::Cons(MyNode {
        val: "same".into(),
        next: Box::new(MyList::Empty),
    });
    let b = MyList::Cons(MyNode {
        val: "same".into(),
        next: Box::new(MyList::Empty),
    });

    let mut v = StringEqVisitor;
    assert!(a.drive_two_inner(&b, &mut v).is_continue());

    let c = MyList::Cons(MyNode {
        val: "other".into(),
        next: Box::new(MyList::Empty),
    });
    let mut v = StringEqVisitor;
    assert!(a.drive_two_inner(&c, &mut v).is_break());
}
