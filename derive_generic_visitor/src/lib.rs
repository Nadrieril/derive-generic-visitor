//! Boilerplate for building visitors, inspired by `derive-visitor`.
//!
//! # Driving a visitor
//!
//! The premise of this crate is that to visit a type means to call a function on each of its
//! fields. The `Visit[Mut]` and `Drive[Mut]` traits of this module provide the simplest interface
//! for this: a type that implements a `Visit<...>` for a bunch of types is like a bundle of
//! `FnMut` closures, and `drive_inner` on a type `T` calls `<V as Visit<FieldTy>>::visit` on each
//! field of `T`.
//!
//! The `Drive`/`DriveMut` derive macros implement these types automatically for a type. With that
//! boilerplate out of the way, it becomes easy to define flexible visitors.
//!
//! The output of the derive macros looks like:
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
//! As you can see, this is not recursive in any way: `x.drive_inner(v)` simply calls `v.visit()`
//! on each field of `x`; it is up to the visitor to recurse into nested structures if it wishes.
//!
//! # Defining useful visitors
//!
//! A visitor is a type that implements `Visit<T>`/`VisitMut<T>` for a set of types `T`. An
//! implementation of `Visit[Mut]` typically involves calling `x.drive_inner(self)` to recurse into
//! the type's contents, with some work done before or after that call. The `Visit` and `VisitMut`
//! derive macros make such usage straightforward.
//!
//! ```rust
//! # use derive_generic_visitor::*;
//! #[derive(Drive)]
//! enum MyList {
//!     Empty,
//!     Cons(MyNode),
//! }
//! #[derive(Drive)]
//! struct MyNode {
//!     val: String,
//!     next: Box<MyList>
//! }
//!
//! #[derive(Default, Visitor, Visit)]
//! #[visit(drive(MyList))] // recurse without custom behavior
//! #[visit(drive(for<T> Box<T>))] // recurse without custom behavior
//! #[visit(enter(MyNode))] // call `self.enter_my_node` before recursing
//! #[visit(skip(String))] // do nothing on a string
//! struct ConcatVisitor(String);
//!
//! impl ConcatVisitor {
//!     fn enter_my_node(&mut self, node: &MyNode) {
//!         self.0 += &node.val;
//!     }
//! }
//!
//! /// Concatenate all the strings in this list.
//! pub fn concat_list(x: &MyList) -> String {
//!     ConcatVisitor::default().visit_by_val_infallible(x).0
//! }
//! ```
//!
//! This expands to:
//! ```rust
//! # use derive_generic_visitor::*;
//! # #[derive(Drive)]
//! # enum MyList {
//! #     Empty,
//! #     Cons(MyNode),
//! # }
//! # #[derive(Drive)]
//! # struct MyNode {
//! #     val: String,
//! #     next: Box<MyList>
//! # }
//! # impl ConcatVisitor {
//! #     fn enter_my_node(&mut self, node: &MyNode) {
//! #         self.0 += &node.val;
//! #     }
//! # }
//! #[derive(Default)]
//! struct ConcatVisitor(String);
//!
//! impl Visitor for ConcatVisitor {
//!     type Break = Infallible;
//! }
//! // Recurse without custom behavior
//! impl<'s> Visit<'s, MyList> for ConcatVisitor {
//!     fn visit(&mut self, x: &'s MyList) -> ControlFlow<Self::Break> {
//!         x.drive_inner(self)
//!     }
//! }
//! // Recurse without custom behavior
//! impl<'s, T> Visit<'s, Box<T>> for ConcatVisitor
//! where
//!     Self: Visit<'s, T>,
//! {
//!     fn visit(&mut self, x: &'s Box<T>) -> ControlFlow<Self::Break> {
//!         x.drive_inner(self)
//!     }
//! }
//! // Call `self.enter_my_node` before recursing
//! impl<'s> Visit<'s, MyNode> for ConcatVisitor {
//!     fn visit(&mut self, x: &'s MyNode) -> ControlFlow<Self::Break> {
//!         self.enter_my_node(x);
//!         x.drive_inner(self)?;
//!         ControlFlow::Continue(())
//!     }
//! }
//! // Do nothing on a string
//! impl<'s> Visit<'s, String> for ConcatVisitor {
//!     fn visit(&mut self, x: &'s String) -> ControlFlow<Self::Break> {
//!         ControlFlow::Continue(())
//!     }
//! }
//! ```
//!
//! The options available are:
//! - `enter(Ty)`: call `self.enter_ty(x)` before recursing with `drive_inner`.
//! - `exit(Ty)`: call `self.exit_ty(x)` after recursing with `drive_inner`.
//! - `override(Ty)`: call `self.visit_ty(x)?`, which may or may not recurse if it wishes to.
//! - `drive(Ty)`: recurse with `drive_inner`.
//! - `skip(Ty)`: do nothing.
//! - `Ty`: alias for `override(Ty)`
//!
//! Instead of `Ty`, one can always write `for<A, B, C> Ty<A, B, C>` to make a generic impl. For
//! `enter`, `exit` and `override`, one may also write `name: Ty` so that `visit_name` etc is
//! called instead of `visit_ty`.
pub use derive_generic_visitor_macros::{Drive, DriveMut, Visit, VisitMut, Visitor};
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
