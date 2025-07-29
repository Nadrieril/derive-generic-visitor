use derive_generic_visitor::*;

#[test]
fn test_derive() {
    #[derive(Drive, DriveMut)]
    struct Foo {
        x: u64,
        y: u32,
        #[drive(skip)]
        #[expect(unused)]
        z: u64,
        nested: Option<Box<Foo>>,
    }
    let foo = Foo {
        x: 1,
        y: 10,
        z: 100,
        nested: Some(Box::new(Foo {
            x: 1000,
            y: 0,
            z: 0,
            nested: None,
        })),
    };

    #[derive(Visitor, Visit)]
    #[visit(u64)]
    #[visit(enter(u32))]
    #[visit(drive(Foo), drive(for<T> Option<T>, for<T> Box<T>))]
    struct SumVisitor {
        sum: u64,
    }
    impl SumVisitor {
        fn visit_u64(&mut self, x: &u64) -> ControlFlow<Infallible> {
            self.sum += *x;
            Continue(())
        }
        fn enter_u32(&mut self, x: &u32) {
            self.sum += *x as u64;
        }
    }

    let sum = (SumVisitor { sum: 0 })
        .visit_by_val(&foo)
        .continue_value()
        .unwrap()
        .sum;
    assert_eq!(sum, 1011);
}

#[derive(Drive, DriveMut)]
enum List<T> {
    Nil,
    Cons(Node<T>),
}

#[derive(Drive, DriveMut)]
struct Node<T> {
    val: T,
    next: Box<List<T>>,
}

impl<T> List<T> {
    fn cons(self, val: T) -> Self {
        Self::Cons(Node {
            val,
            next: Box::new(self),
        })
    }
}

#[test]
fn test_generic_list() {
    #[derive(Default, Visitor, Visit)]
    /// We drive blindly through `Node`, so we need to handle the `T` case. This prevents us from
    /// having a generic `Box` visitor, as that would clash if `T = Box<_>`.
    #[visit(elem: T)]
    #[visit(drive(List<T>, Node<T>, Box<List<T>>))]
    struct CollectVisitor<T: Clone> {
        vec: Vec<T>,
    }
    impl<T: Clone> CollectVisitor<T> {
        fn visit_elem(&mut self, x: &T) -> ControlFlow<Infallible> {
            self.vec.push(x.clone());
            Continue(())
        }
    }

    let list: List<u64> = List::Nil.cons(42).cons(1);
    let contents = CollectVisitor::default().visit_by_val_infallible(&list).vec;
    assert_eq!(contents, vec![1, 42]);
}

#[test]
fn test_generic_list2() {
    #[derive(Default, Visitor, Visit)]
    // We don't drive blindly through `Node`: we have a custom visit function, so we don't need
    // `CollectVisitor<T>: Visit<T>`.
    #[visit(Node<T>)]
    #[visit(drive(List<T>, for<U> Box<U>))]
    struct CollectVisitor<T: Clone> {
        vec: Vec<T>,
    }
    impl<T: Clone> CollectVisitor<T> {
        fn visit_node(&mut self, x: &Node<T>) -> ControlFlow<Infallible> {
            self.vec.push(x.val.clone());
            // Instead of using `drive_inner` (which requires `CollectVisitor<T>: Visit<T>` which
            // clashes with the generic `Box<U>` visit), we visit everything but the `T` case with
            // a new visitor. This is overengineered here but demonstrates the flexibility of our
            // interface.
            #[derive(Visitor, Visit)]
            #[visit(skip(T))]
            #[visit(drive(Box<List<T>>))]
            #[visit(override(List<T>))]
            struct InnerVisitor<'a, T: Clone>(&'a mut CollectVisitor<T>);
            impl<'a, T: Clone> InnerVisitor<'a, T> {
                fn visit_list(&mut self, l: &List<T>) -> ControlFlow<Infallible> {
                    self.0.visit(l)
                }
            }
            x.drive_inner(&mut InnerVisitor(self))
        }
    }

    let list: List<u64> = List::Nil.cons(42).cons(1);
    let contents = CollectVisitor::default().visit_by_val_infallible(&list).vec;
    assert_eq!(contents, vec![1, 42]);
}

#[test]
fn test_early_exit() {
    struct Negative;

    #[derive(Default, Visit)]
    #[visit(elem: i32)]
    #[visit(drive(List<i32>, Node<i32>, Box<List<i32>>))]
    struct SumVisitor {
        sum: i32,
    }

    impl Visitor for SumVisitor {
        type Break = Negative;
    }
    impl SumVisitor {
        fn visit_elem(&mut self, x: &i32) -> ControlFlow<Negative> {
            if *x < 0 {
                Break(Negative)
            } else {
                self.sum += x;
                Continue(())
            }
        }
    }

    let list: List<i32> = List::Nil.cons(42).cons(1);
    assert!(SumVisitor::default().visit_by_val(&list).is_continue());
    let list: List<i32> = List::Nil.cons(42).cons(-1);
    assert!(SumVisitor::default().visit_by_val(&list).is_break());
}

#[test]
fn infaillible_visitable_group() {
    #[derive(Drive, DriveMut)]
    struct Id(String);
    #[derive(Drive, DriveMut)]
    enum Expr {
        Literal(usize),
        Let {
            lhs: Pat,
            rhs: Box<Expr>,
            body: Box<Expr>,
        },
    }
    #[derive(Drive, DriveMut)]
    enum Pat {
        Var(Id),
    }

    #[visitable_group(
        // Declares an infaillible visitor: its interface hides away `ControlFlow`s.
        visitor(drive(&AstVisitor), infaillible),
        skip(usize, String),
        drive(for<T: AstVisitable> Box<T>),
        override(Pat, Expr),
        override_skip(Id),
    )]
    trait AstVisitable {}

    struct SumLiterals(usize);
    impl AstVisitor for SumLiterals {
        fn enter_expr(&mut self, expr: &Expr) {
            if let Expr::Literal(n) = expr {
                self.0 += n
            }
        }
    }

    let mut sum = SumLiterals(0);
    sum.visit(&Expr::Let {
        lhs: Pat::Var(Id("hello".into())),
        rhs: Box::new(Expr::Literal(12)),
        body: Box::new(Expr::Literal(30)),
    });
    assert!(sum.0 == 42);
}
