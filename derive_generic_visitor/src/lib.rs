#![doc = include_str!(concat!(env!("OUT_DIR"), "/docs.md"))]
pub use derive_generic_visitor_macros::{
    visitable_group, Drive, DriveMut, Visit, VisitMut, Visitor,
};
pub use std::convert::Infallible;
pub use std::ops::ControlFlow;
pub use ControlFlow::{Break, Continue};

mod basic_impls;
#[cfg(feature = "dynamic")]
pub mod dynamic;

/// A visitor.
///
/// This trait provides the `Break` type used by its two child traits `Visit` and `VisitMut`. All
/// visitors can abort visitation early by returning `ControlFlow::Break`. For the common case of
/// visitors that never return early, use `std::convert::Infallible`. This is the default type used
/// by `derive(Visitor)`.
pub trait Visitor {
    /// The type used for early-return, if the visitor supports it. Use an empty type like
    /// `std::convert::Infallible` if the visitor does not short-circuit.
    type Break;
}

/// A visitor that can visit a type `T`.
pub trait Visit<'a, T: ?Sized>: Visitor {
    /// Visit this value.
    fn visit(&mut self, _: &'a T) -> ControlFlow<Self::Break>;

    /// Convenience alias for method chaining.
    fn visit_by_val(mut self, x: &'a T) -> ControlFlow<Self::Break, Self>
    where
        Self: Sized,
    {
        self.visit(x)?;
        Continue(self)
    }

    /// Convenience when the visitor does not return early.
    fn visit_by_val_infallible(self, x: &'a T) -> Self
    where
        Self: Visitor<Break = Infallible> + Sized,
    {
        match self.visit_by_val(x) {
            Continue(x) => x,
        }
    }
}

/// A visitor that can mutably visit a type `T`.
pub trait VisitMut<'a, T: ?Sized>: Visitor {
    /// Visit this value.
    fn visit(&mut self, _: &'a mut T) -> ControlFlow<Self::Break>;

    /// Convenience alias for method chaining.
    fn visit_by_val(mut self, x: &'a mut T) -> ControlFlow<Self::Break, Self>
    where
        Self: Sized,
    {
        self.visit(x)?;
        Continue(self)
    }
}

/// A type that can be visited.
pub trait Drive<'s, V: Visitor> {
    /// Call `v.visit()` on the immediate contents of `self`.
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break>;
}

/// A type that can be visited mutably.
pub trait DriveMut<'s, V: Visitor> {
    /// Call `v.visit()` on the immediate contents of `self`.
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break>;
}

/// Drive through an iterable type. Useful for collections in third-party crates for which there
/// isn't a `Drive` impl.
pub fn drive_iter<'a, C, T, V>(iterable: C, v: &mut V) -> ControlFlow<<V as Visitor>::Break>
where
    C: IntoIterator<Item = &'a T>,
    V: Visit<'a, T>,
    T: 'a,
{
    for x in iterable {
        v.visit(x)?;
    }
    Continue(())
}
/// Drive through an iterable type. Useful for collections in third-party crates for which there
/// isn't a `Drive` impl.
pub fn drive_iter_mut<'a, C, T, V>(iterable: C, v: &mut V) -> ControlFlow<<V as Visitor>::Break>
where
    C: IntoIterator<Item = &'a mut T>,
    V: VisitMut<'a, T>,
    T: 'a,
{
    for x in iterable {
        v.visit(x)?;
    }
    Continue(())
}
