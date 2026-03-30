use derive_generic_visitor::*;

#[test]
fn infallible_visitable_group() {
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
        // Declares an infallible visitor: its interface hides away `ControlFlow`s.
        visitor(drive(
            /// Documentation. Or any attribute, really.
            &AstVisitor
        ), infallible),
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

#[test]
fn visitable_group_with_super_bounds() {
    use std::collections::HashMap;

    trait HasEnv {
        fn env(&self) -> &HashMap<String, usize>;
    }

    #[derive(Drive, DriveMut)]
    struct Id(String);
    #[derive(Drive, DriveMut)]
    enum Expr {
        Literal(usize),
        Var(Id),
    }

    #[visitable_group(
        visitor(drive_mut(&mut AstVisitor), infallible, bounds(HasEnv)),
        skip(usize, String),
        override(Expr),
        override_skip(Id),
    )]
    trait AstVisitable {}

    /// Inlines variables found in the environment as literals.
    struct InlineVars {
        env: HashMap<String, usize>,
    }
    impl HasEnv for InlineVars {
        fn env(&self) -> &HashMap<String, usize> {
            &self.env
        }
    }
    impl AstVisitor for InlineVars {
        fn exit_expr(&mut self, expr: &mut Expr) {
            if let Expr::Var(Id(name)) = expr {
                if let Some(&val) = self.env().get(name.as_str()) {
                    *expr = Expr::Literal(val);
                }
            }
        }
    }

    let mut expr = Expr::Var(Id("x".into()));
    let mut visitor = InlineVars {
        env: HashMap::from([("x".into(), 42)]),
    };
    visitor.visit(&mut expr);
    assert!(matches!(expr, Expr::Literal(42)));
}
