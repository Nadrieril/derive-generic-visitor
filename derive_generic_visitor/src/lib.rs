//! Infrastructure for automatically deriving visitors.
//!
//! Premise: to visit a type means to call a function on each of its fields. The `Visitor` and
//! `Drive` traits of this module provide the simplest interface for this: a type that implements a
//! bunch of `Visit<...>` is like a bundle of `FnMut` closures, and `drive_inner` on a type `T`
//! calls `<V as Visit<FieldTy>>::visit` on each field of `T`.
//!
//! A derive macro in the `macros` crate implements `Drive` automatically for a type. The output
//! looks like:
//! ```ignore
//! #[derive(Drive)]
//! enum MyList {
//!     Empty,
//!     Cons(String, Box<MyList>),
//! }
//! ```
//! ```rust
//! # use derive_generic_visitor::{Drive, Visitor, Visit};
//! # use std::ops::ControlFlow;
//! # enum MyList {
//! #     Empty,
//! #     Cons(String, Box<MyList>),
//! # }
//! impl<'s, V> Drive<'s, V> for MyList
//! where
//!     V: Visitor,
//!     V: Visit<'s, String>,
//!     V: Visit<'s, Box<MyList>>,
//! {
//!     fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
//!         match self {
//!             Self::Empty => {}
//!             Self::Cons(x, y) => {
//!                 v.visit(x)?;
//!                 v.visit(y)?;
//!             }
//!         }
//!         ControlFlow::Continue(())
//!     }
//! }
//! ```
//!
//! Note how this is not recursive in any way: `x.drive_inner(v)` simply calls `v.visit()` on each
//! field of `x`; it is up to the visitor to recurse into nested structures if it wishes. There is
//! in general more work needed to get a useful visitor from this. What this provides is the
//! boilerplate-y core, on top of which visitors can be built.
#![cfg_attr(feature = "nightly", feature(associated_type_defaults))]
pub use derive_generic_visitor_macros::{Drive, DriveMut, Visit, VisitMut};
pub use std::convert::Infallible;
pub use std::ops::ControlFlow;
pub use ControlFlow::{Break, Continue};

mod basic_impls;
#[cfg(feature = "dynamic")]
pub mod dynamic;

/// A visitor.
pub trait Visitor {
    /// The type used for early-return, if the visitor supports it. Use an empty type like
    /// `std::convert::Infallible` if the visitor does not short-circuit.
    #[cfg(not(feature = "nightly"))]
    type Break;
    #[cfg(feature = "nightly")]
    type Break = Infallible;
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

    /// Convenience when the visitor does not early-return.
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
