[![crates.io](https://img.shields.io/crates/v/derive_generic_visitor)](https://crates.io/crates/derive_generic_visitor)
[![docs.rs](https://img.shields.io/docsrs/derive_generic_visitor/latest)](https://docs.rs/derive_generic_visitor/latest/derive_generic_visitor/)

<!-- start-doc -->

Boilerplate for building visitors, inspired by
[`derive-visitor`](https://docs.rs/derive-visitor/latest/derive_visitor/).

## Driving a visitor

The premise of this crate is that to visit a type means to call a function on each of its
fields. The `Visit[Mut]` and `Drive[Mut]` traits of this module provide the simplest interface
for this: a type that implements a `Visit<...>` for a bunch of types is like a bundle of
`FnMut` closures, and `drive_inner` on a type `T` calls `<V as Visit<FieldTy>>::visit` on each
field of `T`.

The `Drive`/`DriveMut` derive macros implement these types automatically for a type. With that
boilerplate out of the way, it becomes easy to define flexible visitors.

The output of the derive macros looks like:
```ignore
#[derive(Drive)]
enum MyList {
    Empty,
    Cons(String, Box<MyList>),
}
```
```rust
# use derive_generic_visitor::{Drive, Visitor, Visit};
# use std::ops::ControlFlow;
# enum MyList {
#     Empty,
#     Cons(String, Box<MyList>),
# }
impl<'s, V> Drive<'s, V> for MyList
where
    V: Visitor,
    V: Visit<'s, String>,
    V: Visit<'s, Box<MyList>>,
{
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        match self {
            Self::Empty => {}
            Self::Cons(x, y) => {
                v.visit(x)?;
                v.visit(y)?;
            }
        }
        ControlFlow::Continue(())
    }
}
```

As you can see, this is not recursive in any way: `x.drive_inner(v)` simply calls `v.visit()`
on each field of `x`; it is up to the visitor to recurse into nested structures if it wishes.


## Defining useful visitors

A visitor is a type that implements `Visit<T>`/`VisitMut<T>` for a set of types `T`. An
implementation of `Visit[Mut]` typically involves calling `x.drive_inner(self)` to recurse into
the type's contents, with some work done before or after that call. The `Visit` and `VisitMut`
derive macros make such usage straightforward.

```rust
# use derive_generic_visitor::*;
#[derive(Drive)]
enum MyList {
    Empty,
    Cons(MyNode),
}
#[derive(Drive)]
struct MyNode {
    val: String,
    next: Box<MyList>
}

#[derive(Default, Visitor, Visit)]
#[visit(drive(MyList))] // recurse without custom behavior
#[visit(drive(for<T> Box<T>))] // recurse without custom behavior
#[visit(enter(MyNode))] // call `self.enter_my_node` before recursing
#[visit(skip(String))] // do nothing on a string
struct ConcatVisitor(String);

impl ConcatVisitor {
    fn enter_my_node(&mut self, node: &MyNode) {
        self.0 += &node.val;
    }
}

/// Concatenate all the strings in this list.
pub fn concat_list(x: &MyList) -> String {
    ConcatVisitor::default().visit_by_val_infallible(x).0
}
```

This expands to:
```rust
# use derive_generic_visitor::*;
# #[derive(Drive)]
# enum MyList {
#     Empty,
#     Cons(MyNode),
# }
# #[derive(Drive)]
# struct MyNode {
#     val: String,
#     next: Box<MyList>
# }
# impl ConcatVisitor {
#     fn enter_my_node(&mut self, node: &MyNode) {
#         self.0 += &node.val;
#     }
# }
#[derive(Default)]
struct ConcatVisitor(String);

impl Visitor for ConcatVisitor {
    type Break = Infallible;
}
// Recurse without custom behavior
impl<'s> Visit<'s, MyList> for ConcatVisitor {
    fn visit(&mut self, x: &'s MyList) -> ControlFlow<Self::Break> {
        x.drive_inner(self)
    }
}
// Recurse without custom behavior
impl<'s, T> Visit<'s, Box<T>> for ConcatVisitor
where
    Self: Visit<'s, T>,
{
    fn visit(&mut self, x: &'s Box<T>) -> ControlFlow<Self::Break> {
        x.drive_inner(self)
    }
}
// Call `self.enter_my_node` before recursing
impl<'s> Visit<'s, MyNode> for ConcatVisitor {
    fn visit(&mut self, x: &'s MyNode) -> ControlFlow<Self::Break> {
        self.enter_my_node(x);
        x.drive_inner(self)?;
        ControlFlow::Continue(())
    }
}
// Do nothing on a string
impl<'s> Visit<'s, String> for ConcatVisitor {
    fn visit(&mut self, x: &'s String) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }
}
```

The options available are:
- `enter(Ty)`: call `self.enter_ty(x)` before recursing with `drive_inner`.
- `exit(Ty)`: call `self.exit_ty(x)` after recursing with `drive_inner`.
- `override(Ty)`: call `self.visit_ty(x)?`, which may or may not recurse if it wishes to.
- `drive(Ty)`: recurse with `drive_inner`.
- `skip(Ty)`: do nothing.
- `Ty`: alias for `override(Ty)`

Instead of `Ty`, one can always write `for<A, B, C> Ty<A, B, C>` to make a generic impl. For
`enter`, `exit` and `override`, one may also write `name: Ty` so that `visit_name` etc is
called instead of `visit_ty`.


## Reusable visitors

For more complex scenarios where one-off visitor structs would be tedious, this crate provides
a final macro: `visitable_group`. Given a set of types of interest, this generates a pair of
traits: a `Visitable` trait implemented by all these types, and a `Visitor` trait with default
methods that defines visitors over these types.

This is a reusable version of the one-off visitor structs we saw in the previous section: the
`enter_foo`/`exit_foo`/`visit_foo` methods are now trait methods, in such a way that many
visitors can be defined for that same set of types.

```rust
# use derive_generic_visitor::*;
#[derive(Drive)]
enum List {
    Empty,
    Cons(Node),
}
#[derive(Drive)]
struct Node {
    val: String,
    next: Box<List>
}

#[visitable_group(
    visitor(drive_list(&ListVisitor)), // also available: `&mut`
    drive(List, for<T: ListVisitable> Box<T>),
    skip(String),
    override(Node),
)]
trait ListVisitable {}

#[derive(Visitor)]
struct SomeVisitor;

impl ListVisitor for SomeVisitor {
    // Here, methods `enter_node`, `exit_node` and `visit_node` are available to override.
    // Calling `self.visit(&list)` will explore the list.
}
```

The generated visitor trait has methods much like those from the `Visit[Mut]` derives, that can
be overriden freely. The result is:

```rust
# use derive_generic_visitor::*;
# #[derive(Drive)]
# enum List {
#     Empty,
#     Cons(Node),
# }
# #[derive(Drive)]
# struct Node {
#     val: String,
#     next: Box<List>
# }
/// Implementation detail: wrapper that implements `Visit[Mut]<T>` for `T: ListVisitable`,
/// and delegates all the visiting to our trait's `drive[_mut]`. Used in the implementation of
/// `visit_inner`
#[repr(transparent)]
pub struct ListVisitableWrapper<V: ?Sized>(V);
impl<V: ?Sized> ListVisitableWrapper<V> {
    fn wrap(x: &mut V) -> &mut Self {
        unsafe { std::mem::transmute(x) }
    }
}
impl<V: Visitor> Visitor for ListVisitableWrapper<V> {
    type Break = V::Break;
}
impl<'s, V: ListVisitor, T: ListVisitable> Visit<'s, T> for ListVisitableWrapper<V> {
    fn visit(&mut self, x: &'s T) -> ControlFlow<Self::Break> {
        self.0.visit(x)
    }
}

trait ListVisitable {
    /// Recursively visit this type with the provided visitor. This calls the visitor's `visit_$any`
    /// method if it exists, otherwise `visit_inner`.
    fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break>;
}

trait ListVisitor: Visitor + Sized {
    /// Visit a visitable type. This calls the appropriate method of this trait on `x`
    /// (`visit_$ty` if it exists, `visit_inner` if not).
    fn visit<'a, T: ListVisitable>(
        &'a mut self,
        x: &T,
    ) -> ControlFlow<Self::Break> {
        x.drive_list(self)
    }
    /// Visit the contents of `x`. This calls `self.visit()` on each field of `T`. This
    /// is available for any `ListVisitable` type whose contents are all `ListVisitable`.
    fn visit_inner<T>(&mut self, x: &T) -> ControlFlow<Self::Break>
    where
        T: for<'s> Drive<'s, ListVisitableWrapper<Self>> + ListVisitable,
    {
        x.drive_inner(ListVisitableWrapper::wrap(self))
    }

    /// Overrideable method called when visiting a `$ty`. When overriding this method,
    /// call `self.visit_inner(x)` to keep recursively visiting the type, or don't call
    /// it if the contents of `x` should not be visited.
    ///
    /// The default implementation calls `enter_$ty` then `visit_inner` then `exit_$ty`.
    fn visit_node(&mut self, x: &Node) -> ControlFlow<Self::Break> {
        self.enter_node(x);
        self.visit_inner(x)?;
        self.exit_node(x);
        Continue(())
    }
    /// Called when starting to visit a `$ty` (unless `visit_$ty` is overriden).
    fn enter_node(&mut self, x: &Node) {}
    /// Called when finished visiting a `$ty` (unless `visit_$ty` is overriden).
    fn exit_node(&mut self, x: &Node) {}
}

impl ListVisitable for List {
    fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit_inner(self)
    }
}
impl<T: ListVisitable> ListVisitable for Box<T> {
    fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit_inner(self)
    }
}
impl ListVisitable for String {
    fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
        ControlFlow::Continue(())
    }
}
impl ListVisitable for Node {
    fn drive_list<V: ListVisitor>(&self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit_node(self)
    }
}
```

To illustrate, the typical visit loop would look like, given a `MyVisitor: ListVisitor`:
- `<MyVisitor as ListVisitor>::visit(v, x)`
- `<Node as GroupVisitable>::drive_list(x, v)` // assuming `x: Node`
- `<MyVisitor as ListVisitor>::visit_node(v, x)` // here lives custom behavior
- `<MyVisitor as ListVisitor>::visit_inner(v, x)`
- `<Node as Drive>::drive_inner(ListVisitorWrapper(v))`
- calls `<MyVisitor as ListVisitor>::visit(v, &x.field)` on each field of `x`

The options available for the `visitable_group` macro are:
- `visitor(drive_method_name(&[mut]TraitName)[, infaillible])`: derive a visitor trait named `TraitName`.
  - the presence of `mut` determines whether the `TraitName` visitor will operate on mutable or immutable borrows.
  - the optional `infaillible` flag enables an infaillible-style interface for the visitor:, where its methods `visit_$ty` return `()` instead of `ControlFlow<_>`.
- `override_skip(Ty)`: similar to `override(Ty)`, but the default implementation does nothing, and no `enter_Ty` or `exit_Ty` methods are generated.
- `override(Ty)`, `drive(Ty)` and `skip(Ty)`: behave the same as their counterparts in the `Visit` and `VisitMut` derives.

Notes:
 - the `visitable_group` interface makes it possible to write composable visitor
wrappers that provide reusable functionality. For an example, see
[`derive_generic_visitor/tests/visitable_group_wrapper.rs`].
