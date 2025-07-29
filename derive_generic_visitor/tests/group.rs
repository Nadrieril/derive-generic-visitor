use derive_generic_visitor::*;

#[derive(Drive, DriveMut)]
enum List {
    Nil,
    Cons(Node),
}

#[derive(Drive, DriveMut)]
struct Node {
    #[drive(skip)]
    val: u32,
    next: Box<List>,
}

impl List {
    fn cons(self, val: u32) -> Self {
        Self::Cons(Node {
            val,
            next: Box::new(self),
        })
    }

    fn from_list(slice: &[u32]) -> Self {
        let mut ret = List::Nil;
        for x in slice.iter().copied().rev() {
            ret = ret.cons(x);
        }
        ret
    }
}

#[visitable_group(
    visitor(drive_list(&ListVisitor)),
    visitor(drive_list_mut(&mut ListVisitorMut)),
    drive(List, for<T: ListVisitable> Box<T>),
    override(Node),
)]
trait ListVisitable {}

#[test]
fn test_visitor_wrapper() {
    /// Wraps a visitor to also track list depth so far.
    struct DepthWrapper<'a, V>(&'a mut V);
    trait VisitorWithDepth {
        fn depth_mut(&mut self) -> &mut usize;
    }

    impl<'a, V: Visitor> Visitor for DepthWrapper<'a, V> {
        type Break = V::Break;
    }
    impl<'a, V: ListVisitor + VisitorWithDepth> ListVisitor for DepthWrapper<'a, V> {
        fn visit_inner<T>(&mut self, x: &T) -> ControlFlow<Self::Break>
        where
            T: for<'s> derive_generic_visitor::Drive<'s, ListVisitableWrapper<Self>>
                + ListVisitable,
        {
            // This calls the appropriate method of the inner visitor on `x`.
            x.drive_list(self.0)
        }

        // `visit_node` and all the other default methods will then call `visit_inner` above.
        fn enter_node(&mut self, _: &Node) {
            *self.0.depth_mut() += 1;
        }
        fn exit_node(&mut self, _: &Node) {
            *self.0.depth_mut() -= 1;
        }
    }

    /// Wraps a visitor to also track list sum so far.
    struct SumWrapper<'a, V>(&'a mut V);
    trait VisitorWithSum {
        fn sum_mut(&mut self) -> &mut u32;
    }

    impl<'a, V: Visitor> Visitor for SumWrapper<'a, V> {
        type Break = V::Break;
    }
    impl<'a, V: VisitorWithDepth> VisitorWithDepth for SumWrapper<'a, V> {
        fn depth_mut(&mut self) -> &mut usize {
            self.0.depth_mut()
        }
    }
    impl<'a, V: ListVisitor + VisitorWithSum> ListVisitor for SumWrapper<'a, V> {
        fn visit_inner<T>(&mut self, x: &T) -> ControlFlow<Self::Break>
        where
            T: for<'s> derive_generic_visitor::Drive<'s, ListVisitableWrapper<Self>>
                + ListVisitable,
        {
            // This calls the appropriate method of the inner visitor on `x`.
            x.drive_list(self.0)
        }

        // `visit_node` and all the other default methods will then call `visit_inner` above.
        fn enter_node(&mut self, x: &Node) {
            *self.0.sum_mut() += x.val;
        }
    }

    #[derive(Default, Visitor)]
    struct MyVisitor {
        depth: usize,
        sum: u32,
        total: u32,
    }
    impl VisitorWithDepth for MyVisitor {
        fn depth_mut(&mut self) -> &mut usize {
            &mut self.depth
        }
    }
    impl VisitorWithSum for MyVisitor {
        fn sum_mut(&mut self) -> &mut u32 {
            &mut self.sum
        }
    }
    impl ListVisitor for MyVisitor {
        fn visit<'a, T: ListVisitable>(&'a mut self, x: &T) -> ControlFlow<Self::Break> {
            DepthWrapper(&mut SumWrapper(self)).visit(x)
        }

        fn visit_node(&mut self, x: &Node) -> ControlFlow<Self::Break> {
            self.total += x.val * self.depth as u32;
            self.visit_inner(x)
        }
    }

    let slice = &[0, 1, 2, 3, 4, 5, 6];
    let list = List::from_list(slice);
    let visitor = MyVisitor::default().visit_by_val_infallible(&list);
    assert_eq!(visitor.sum, slice.iter().sum());
    assert_eq!(
        visitor.total,
        slice
            .iter()
            .enumerate()
            .map(|(i, val)| (i as u32 + 1) * val)
            .sum()
    );
}
