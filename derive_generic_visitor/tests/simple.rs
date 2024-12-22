use derive_generic_visitor::*;

#[test]
fn test_derive() {
    #[derive(Drive, DriveMut)]
    struct Foo {
        x: u64,
        y: u64,
        #[drive(skip)]
        #[expect(unused)]
        z: u64,
    }
    let foo = Foo {
        x: 41,
        y: 1,
        z: 100,
    };

    struct SumVisitor {
        sum: u64,
    }
    impl Visitor for SumVisitor {
        type Break = Infallible;
    }
    impl Visit<'_, u64> for SumVisitor {
        fn visit(&mut self, x: &u64) -> ControlFlow<Infallible> {
            self.sum += *x;
            Continue(())
        }
    }
    impl Visit<'_, Foo> for SumVisitor {
        fn visit(&mut self, x: &Foo) -> ControlFlow<Infallible> {
            x.drive_inner(self)?;
            Continue(())
        }
    }

    let sum = (SumVisitor { sum: 0 })
        .visit_by_val(&foo)
        .continue_value()
        .unwrap()
        .sum;
    assert_eq!(sum, 42);
}
