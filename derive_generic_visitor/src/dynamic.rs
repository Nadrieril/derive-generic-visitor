// Re-export the `dyn Any`-based visitor traits.
use crate::*;
pub use derive_visitor::{
    Drive as DriveDyn, DriveMut as DriveMutDyn, Visitor as VisitorDyn, VisitorMut as VisitorMutDyn,
};

/// Compatibility layer with `derive_visitor` visitors. To implement `derive_visitor::Drive[Mut]`,
/// call `dyn_visitor::drive[_mut]` inside the `drive[_mut]` method implementation.
pub mod dyn_visitor {
    use std::any::Any;

    use super::*;

    /// For `V: derive_visitor::Visitor[Mut]`, this implements `Visit[Mut]`. Can be used to
    /// implement `derive_visitor::Drive[Mut]` given implementations for this module's `Drive[Mut]`
    /// traits.
    #[repr(transparent)]
    pub struct DynVisitorAdapter<V>(V);

    impl<V> DynVisitorAdapter<V> {
        pub fn wrap(x: &mut V) -> &mut Self {
            // SAFETY: repr(transparent)
            unsafe { std::mem::transmute(x) }
        }
    }

    /// Walk the dyn visitor over the given visitable value. Use this to implement
    /// `derive_visitor::Drive` for your type.
    pub fn drive<V, T>(x: &T, v: &mut V)
    where
        V: VisitorDyn,
        T: for<'a> Drive<'a, DynVisitorAdapter<V>> + Any,
    {
        v.visit(x, derive_visitor::Event::Enter);
        let _ = x.drive_inner(DynVisitorAdapter::wrap(v));
        v.visit(x, derive_visitor::Event::Exit);
    }

    /// Walk the dyn visitor over the given visitable value. Use this to implement
    /// `derive_visitor::DriveMut` for your type.
    pub fn drive_mut<V, T>(x: &mut T, v: &mut V)
    where
        V: VisitorMutDyn,
        T: for<'a> DriveMut<'a, DynVisitorAdapter<V>> + Any,
    {
        v.visit(x, derive_visitor::Event::Enter);
        let _ = x.drive_inner_mut(DynVisitorAdapter::wrap(v));
        v.visit(x, derive_visitor::Event::Exit);
    }

    impl<V> Visitor for DynVisitorAdapter<V> {
        type Break = Infallible;
    }
    impl<V: VisitorDyn, T: DriveDyn> Visit<'_, T> for DynVisitorAdapter<V> {
        fn visit(&mut self, x: &T) -> ControlFlow<Self::Break> {
            x.drive(&mut self.0);
            Continue(())
        }
    }
    impl<V: VisitorMutDyn, T: DriveMutDyn> VisitMut<'_, T> for DynVisitorAdapter<V> {
        fn visit(&mut self, x: &mut T) -> ControlFlow<Self::Break> {
            x.drive_mut(&mut self.0);
            Continue(())
        }
    }
}
