use derive_generic_visitor::*;

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
