//! Boilerplate for building visitors, inspired by
//! [`derive-visitor`](https://docs.rs/derive-visitor/latest/derive_visitor/).
//!
//! ## Visitors and drivers
//!
//! The basic purpose of this crate is to provide a simple derive macro that does "call a function on
//! each field of this type". This building block can then be used for a ton of things, and I'm hoping
//! to save people work by providing it.
//!
//! In this crate, "visiting a type" is the name we give to "calling a function on each of its fields".
//! The `Visit[Mut]` and `Drive[Mut]` traits of this module provide the simplest interface for this:
//! a type that implements a `Visit<A> + Visit<B>` for a bunch of types is like a bundle of `FnMut(&mut
//! A)`, `FnMut(&mut B)` closures, and `<T as Drive>::drive_inner(v)` on a type `T` calls `<V as
//! Visit<FieldTy>>::visit` on each field of `T`.
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
//! As you can see, this is not recursive in any way: `x.drive_inner(v)` simply calls `v.visit()` on
//! each field of `x`; it is up to the visitor to recurse into nested structures if it wishes to do so.
//!
//!
//! ## Defining useful visitors
//!
//! On top of the `Drive`/`DriveMut` derive macros, this crate provide more opinionated utilities to get
//! simple visitor architectures started.
//!
//! A visitor is a type that implements `Visit<T>`/`VisitMut<T>` for a set of types `T`. An
//! implementation of `Visit[Mut]` typically involves doing some work then calling `x.drive_inner(self)`
//! to recurse into the type's contents. The crate provides `Visit` and `VisitMut` derive macros to make
//! such usage straightforward.
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
//! #[visit(drive(for<T> Box<T>))] // recurse on a polymorphic type without custom behavior
//! #[visit(enter(MyNode))] // call `self.enter_my_node` before recursing
//! #[visit(skip(String))] // do nothing when visiting a string
//! struct ConcatVisitor(String);
//!
//! impl ConcatVisitor {
//!     // This will be called when the visitor reaches a `MyNode`, before it recurses.
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
//! - `override(Ty)`: call `self.visit_ty(x)?`, which may or may not recurse if it wishes to and can
//!     also early-return.
//! - `drive(Ty)`: recurse with `drive_inner`.
//! - `skip(Ty)`: do nothing.
//! - `Ty`: alias for `override(Ty)`
//!
//! Instead of `Ty`, one can always write `for<A, B, C> Ty<A, B, C>` to make a generic impl. For
//! `enter`, `exit` and `override`, one may also write `other_name: Ty` so that `visit_other_name` is
//! called instead of `visit_ty`.
//!
//!
//! ## Overrideable visitor architecture via traits
//!
//! For more complex scenarios where one-off visitor structs would be tedious, this crate provides
//! a final macro: `visitable_group`. Given a set of types of interest, this generates a pair of
//! traits: a `Visitable` trait implemented by all these types, and a `Visitor` trait with default
//! methods that defines visitors over these types.
//!
//! This is a reusable version of the one-off visitor structs we saw in the previous section: the
//! `enter_foo`/`exit_foo`/`visit_foo` methods are now trait methods, in such a way that many
//! visitors can be defined for that same set of types.
//!
//! ```rust
//! # use derive_generic_visitor::*;
//! #[derive(Drive)]
//! enum List {
//!     Empty,
//!     Cons(Node),
//! }
//! #[derive(Drive)]
//! struct Node {
//!     val: String,
//!     next: Box<List>
//! }
//!
//! #[visitable_group(
//!     // Specifies the trait to generate. Also available: `&mut ListVisitor` for mutating visitors.
//!     visitor(drive_list(&ListVisitor)),
//!     drive(List, for<T: ListVisitable> Box<T>), // simply recurse through these, no override available
//!     skip(String), // simply skip these, no override available
//!     override(Node), // define overrideable `enter_node`, `exit_node` and `visit_node` methods
//! )]
//! trait ListVisitable {}
//!
//! #[derive(Visitor)]
//! struct SomeVisitor;
//!
//! // The `ListVisitor` trait was generated by the `visitable_group` macro.
//! // Calling `visitor.visit(&list)` will run the visitor on the list.
//! impl ListVisitor for SomeVisitor {
//!     // Here, methods `enter_node`, `exit_node` and `visit_node` are available to override.
//!     fn enter_node(&mut self, node: &Node) {
//!         // ...
//!     }
//! }
//! ```
//!
//! The generated visitor trait has methods much like those from the `Visit[Mut]` derives, that can
//! be overridden freely. The result is:
//!
//! ```rust
//! # use derive_generic_visitor::*;
//! # #[derive(Drive)]
//! # enum List {
//! #     Empty,
//! #     Cons(Node),
//! # }
//! # #[derive(Drive)]
//! # struct Node {
//! #     val: String,
//! #     next: Box<List>
//! # }
//! /// Implementation detail: wrapper that implements `Visit[Mut]<T>` for `T: ListVisitable`,
//! /// and delegates all the visiting to our trait's `drive[_mut]`. Used in the implementation of
//! /// `visit_inner`
//! #[repr(transparent)]
//! pub struct ListVisitableWrapper<V: ?Sized>(V);
//! impl<V: ?Sized> ListVisitableWrapper<V> {
//!     fn wrap(x: &mut V) -> &mut Self {
//!         unsafe { std::mem::transmute(x) }
//!     }
//! }
//! impl<V: Visitor> Visitor for ListVisitableWrapper<V> {
//!     type Break = V::Break;
//! }
//! impl<'s, V: ListVisitor, T: ListVisitable> Visit<'s, T> for ListVisitableWrapper<V> {
//!     fn visit(&mut self, x: &'s T) -> ControlFlow<Self::Break> {
//!         self.0.visit(x)
//!     }
//! }
//!
//! trait ListVisitable {
//!     /// Recursively visit this type with the provided visitor. This calls the visitor's `visit_$any`
//!     /// method if it exists, otherwise `visit_inner`.
//!     fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break>;
//! }
//!
//! trait ListVisitor: Visitor + Sized {
//!     /// Visit a visitable type. This calls the appropriate method of this trait on `x`
//!     /// (`visit_$ty` if it exists, `visit_inner` if not).
//!     fn visit<'a, T: ListVisitable>(
//!         &'a mut self,
//!         x: &T,
//!     ) -> ControlFlow<Self::Break> {
//!         x.drive_list(self)
//!     }
//!     /// Visit the contents of `x`. This calls `self.visit()` on each field of `T`. This
//!     /// is available for any `ListVisitable` type whose contents are all `ListVisitable`.
//!     fn visit_inner<T>(&mut self, x: &T) -> ControlFlow<Self::Break>
//!     where
//!         T: for<'s> Drive<'s, ListVisitableWrapper<Self>> + ListVisitable,
//!     {
//!         x.drive_inner(ListVisitableWrapper::wrap(self))
//!     }
//!
//!     /// Overrideable method called when visiting a `$ty`. When overriding this method,
//!     /// call `self.visit_inner(x)` to keep recursively visiting the type, or don't call
//!     /// it if the contents of `x` should not be visited.
//!     ///
//!     /// The default implementation calls `enter_$ty` then `visit_inner` then `exit_$ty`.
//!     fn visit_node(&mut self, x: &Node) -> ControlFlow<Self::Break> {
//!         self.enter_node(x);
//!         self.visit_inner(x)?;
//!         self.exit_node(x);
//!         Continue(())
//!     }
//!     /// Called when starting to visit a `$ty` (unless `visit_$ty` is overriden).
//!     fn enter_node(&mut self, x: &Node) {}
//!     /// Called when finished visiting a `$ty` (unless `visit_$ty` is overriden).
//!     fn exit_node(&mut self, x: &Node) {}
//! }
//!
//! impl ListVisitable for List {
//!     fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
//!         v.visit_inner(self)
//!     }
//! }
//! impl<T: ListVisitable> ListVisitable for Box<T> {
//!     fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
//!         v.visit_inner(self)
//!     }
//! }
//! impl ListVisitable for String {
//!     fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
//!         ControlFlow::Continue(())
//!     }
//! }
//! impl ListVisitable for Node {
//!     fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
//!         v.visit_node(self)
//!     }
//! }
//! ```
//!
//! To illustrate, the typical visit loop would look like, given a `MyVisitor: ListVisitor`:
//! - `<MyVisitor as ListVisitor>::visit(v, x)` // entrypoint
//! - `<Node as GroupVisitable>::drive_list(x, v)` // assuming `x: Node`
//! - `<MyVisitor as ListVisitor>::visit_node(v, x)` // here lives custom behavior
//! - `<MyVisitor as ListVisitor>::visit_inner(v, x)` // assuming `visit_node` was not overridden
//! - `<Node as Drive>::drive_inner(ListVisitorWrapper(v))`
//! - calls `<MyVisitor as ListVisitor>::visit(v, &x.field)` on each field of `x`, completing the loop.
//!
//! The options available for the `visitable_group` macro are:
//! - `visitor(drive_method_name(&[mut]TraitName)[, infallible])`: derive a visitor trait named `TraitName`.
//!   - the presence of `mut` determines whether the `TraitName` visitor will operate on mutable or immutable borrows.
//!   - the optional `infallible` flag enables an infallible-style interface for the visitor:, where its methods `visit_$ty` return `()` instead of `ControlFlow<_>`.
//! - `drive(Ty)` and `skip(Ty)`: behave the same as their counterparts in the `Visit` and `VisitMut`
//!     derives described above.
//! - `override(Ty)`: generates `enter_ty` and `exit_ty` methods that do nothing, and a `visit_ty`
//!     method that calls `enter_ty`, recurses with `self.visit_inner()?`, then calls `exit_ty`.
//! - `override_skip(Ty)`: similar to `override(Ty)`, but the default implementation does nothing, and no `enter_Ty` or `exit_Ty` methods are generated.
//!
//! Note: the `visitable_group` interface makes it possible to write composable
//! visitor wrappers that provide reusable functionality. For an example, see
//! [`derive_generic_visitor/tests/visitable_group_wrapper.rs`].
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
