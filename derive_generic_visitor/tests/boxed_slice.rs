use std::{convert::Infallible, ops::ControlFlow};

use derive_generic_visitor::{Drive, Visit, Visitor};

#[derive(Drive)]
struct StaticList {
    items: Box<[Item]>,
}

#[derive(Drive)]
struct Item {
    x: usize,
    y: i16,
}

#[test]
fn test_boxed_slice() {
    #[derive(Default, Visit, Visitor)]
    #[visit(drive(StaticList))]
    #[visit(drive(Box<[Item]>))]
    #[visit(drive([Item]))]
    #[visit(drive(Item))]
    #[visit(usize)]
    #[visit(i16)]
    #[derive(Debug, PartialEq, Eq)]
    struct Summation {
        sum_x: usize,
        sum_y: isize,
    }

    impl Summation {
        fn visit_usize(&mut self, x: &usize) -> ControlFlow<Infallible> {
            self.sum_x += x;
            ControlFlow::Continue(())
        }

        fn visit_i16(&mut self, y: &i16) -> ControlFlow<Infallible> {
            self.sum_y += *y as isize;
            ControlFlow::Continue(())
        }
    }

    let list = StaticList {
        items: vec![
            Item { x: 1, y: 2 },
            Item { x: 2, y: 4 },
            Item { x: 3, y: 6 },
        ]
        .into_boxed_slice(),
    };
    let sum = Summation::default().visit_by_val_infallible(&list);
    assert_eq!(
        sum,
        Summation {
            sum_x: 6,
            sum_y: 12
        }
    );
}
