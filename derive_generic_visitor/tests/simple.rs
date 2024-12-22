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

    #[derive(Visit)]
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

#[test]
fn test_generic_list() {
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

    #[derive(Default, Visit)]
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

    #[derive(Default, Visit)]
    #[visit(Node<T>)]
    #[visit(drive(List<T>, for<U> Box<U>))]
    struct CollectVisitor2<T: Clone> {
        vec: Vec<T>,
    }
    impl<T: Clone> CollectVisitor2<T> {
        fn visit_node(&mut self, x: &Node<T>) -> ControlFlow<Infallible> {
            self.vec.push(x.val.clone());
            // Instead of using `drive_inner` (which requires `Visit<T>` which clashes with the
            // generic `Box<U>` visit), we visit everything but the `T` case with a new visitor.
            // This is overengineered here but demonstrates the flexibility of our interface.
            #[derive(Visit)]
            #[visit(skip(T))]
            #[visit(drive(Box<List<T>>))]
            #[visit(override(List<T>))]
            struct InnerVisitor<'a, T: Clone>(&'a mut CollectVisitor2<T>);
            impl<'a, T: Clone> InnerVisitor<'a, T> {
                fn visit_list(&mut self, l: &List<T>) -> ControlFlow<Infallible> {
                    self.0.visit(l)
                }
            }
            x.drive_inner(&mut InnerVisitor(self))
        }
    }

    let contents = CollectVisitor2::default()
        .visit_by_val_infallible(&list)
        .vec;
    assert_eq!(contents, vec![1, 42]);
}
