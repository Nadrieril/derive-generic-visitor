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

/// An arena-based AST where `Expr` is an index into an `ExprKind` arena. The visitor uses
/// `bounds(HasArena)` so that the generated `AstVisitor` trait requires arena access, enabling a
/// manual `AstVisitable` impl for `Expr` that resolves indices through the arena.
#[test]
fn visitable_group_with_super_bounds() {
    use std::collections::HashMap;

    type ExprId = usize;

    #[derive(Clone)]
    struct Expr(ExprId);

    #[derive(Clone, Drive)]
    enum ExprKind {
        Literal(usize),
        Var(String),
        Add(Expr, Expr),
    }

    trait HasArena {
        fn arena(&self) -> &HashMap<ExprId, ExprKind>;
    }

    #[visitable_group(
        visitor(drive(&AstVisitor), infallible, bounds(HasArena)),
        skip(usize, String),
        override(ExprKind),
    )]
    trait AstVisitable {}

    // Manually implement `AstVisitable` for `Expr`: look up the arena to visit the `ExprKind`.
    impl AstVisitable for Expr {
        fn drive<V: AstVisitor>(&self, v: &mut V) {
            if let Some(kind) = v.arena().get(&self.0).cloned() {
                v.visit(&kind);
            }
        }
    }

    /// Collects all variable names encountered in the AST.
    struct CollectVars {
        arena: HashMap<ExprId, ExprKind>,
        vars: Vec<String>,
    }
    impl HasArena for CollectVars {
        fn arena(&self) -> &HashMap<ExprId, ExprKind> {
            &self.arena
        }
    }
    impl AstVisitor for CollectVars {
        fn enter_expr_kind(&mut self, kind: &ExprKind) {
            if let ExprKind::Var(name) = kind {
                self.vars.push(name.clone());
            }
        }
    }

    // Build a small arena: `add(x, add(y, 1))`
    let arena = HashMap::from([
        (0, ExprKind::Var("x".into())),
        (1, ExprKind::Var("y".into())),
        (2, ExprKind::Literal(1)),
        (3, ExprKind::Add(Expr(1), Expr(2))),
        (4, ExprKind::Add(Expr(0), Expr(3))),
    ]);

    let mut visitor = CollectVars {
        arena: arena,
        vars: vec![],
    };
    visitor.visit(&Expr(4));
    visitor.vars.sort();
    assert_eq!(visitor.vars, vec!["x", "y"]);
}

/// Test the `&two` visitor mode for lockstep visiting via `visitable_group`.
#[test]
fn visitable_group_two() {
    #[derive(Drive, DriveTwo)]
    enum Expr {
        Literal(usize),
        Add(Box<Expr>, Box<Expr>),
    }

    #[visitable_group(
        visitor(drive_expr_two(&two ExprTwoVisitor)),
        skip(usize),
        drive(for<T: ExprVisitable> Box<T>),
        override(Expr),
    )]
    trait ExprVisitable {}

    struct PairCollector {
        pairs: Vec<(usize, usize)>,
    }
    impl Visitor for PairCollector {
        type Break = ();
    }
    impl ExprTwoVisitor for PairCollector {
        fn enter_expr(&mut self, a: &Expr, b: &Expr) {
            if let (Expr::Literal(x), Expr::Literal(y)) = (a, b) {
                self.pairs.push((*x, *y));
            }
        }
    }

    let a = Expr::Add(Box::new(Expr::Literal(1)), Box::new(Expr::Literal(2)));
    let b = Expr::Add(Box::new(Expr::Literal(10)), Box::new(Expr::Literal(20)));

    let mut collector = PairCollector { pairs: vec![] };
    let _ = collector.visit(&a, &b);
    assert_eq!(collector.pairs, vec![(1, 10), (2, 20)]);

    // --- Two-visitor: variant mismatch breaks early ---
    let c = Expr::Literal(99);
    let collector = PairCollector { pairs: vec![] };
    let result = collector.visit_by_val(&a, &c);
    assert!(result.is_break());
}

/// Test `&two` visitor with `override_skip`.
#[test]
fn visitable_group_two_override_skip() {
    #[derive(Drive, DriveTwo)]
    struct Wrapper {
        val: u64,
        label: String,
    }

    #[visitable_group(
        visitor(drive_two(&two WrapperTwoVisitor)),
        skip(u64),
        override_skip(String),
        override(Wrapper),
    )]
    trait WrapperVisitable {}

    struct EqChecker {
        called: bool,
    }
    impl Visitor for EqChecker {
        type Break = ();
    }
    impl WrapperTwoVisitor for EqChecker {
        fn visit_wrapper(&mut self, a: &Wrapper, b: &Wrapper) -> ControlFlow<()> {
            self.called = true;
            // Only compare vals, ignore labels.
            if a.val != b.val {
                Break(())
            } else {
                Continue(())
            }
        }
    }

    let a = Wrapper {
        val: 42,
        label: "hello".into(),
    };
    let b = Wrapper {
        val: 42,
        label: "world".into(),
    };
    let mut v = EqChecker { called: false };
    assert!(v.visit(&a, &b).is_continue());
    assert!(v.called);

    let c = Wrapper {
        val: 99,
        label: "hello".into(),
    };
    let mut v = EqChecker { called: false };
    assert!(v.visit(&a, &c).is_break());
    assert!(v.called);
}
