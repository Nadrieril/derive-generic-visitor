use crate::*;

impl<'s, T: ?Sized, V> Drive<'s, V> for Box<T>
where
    V: Visit<'s, T>,
{
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self)
    }
}
impl<'s, T: ?Sized, V> DriveMut<'s, V> for Box<T>
where
    V: VisitMut<'s, T>,
{
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&mut **self)
    }
}
impl<'s, T: ?Sized, V> DriveTwo<'s, V> for Box<T>
where
    V: VisitTwo<'s, T>,
{
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self, &**other)
    }
}

impl<'s, T: ?Sized, V> Drive<'s, V> for &T
where
    V: Visit<'s, T>,
{
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self)
    }
}
impl<'s, T: ?Sized, V> DriveTwo<'s, V> for &T
where
    V: VisitTwo<'s, T>,
{
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self, &**other)
    }
}
impl<'s, T: ?Sized, V> Drive<'s, V> for &mut T
where
    V: Visit<'s, T>,
{
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self)
    }
}
impl<'s, T: ?Sized, V> DriveMut<'s, V> for &mut T
where
    V: VisitMut<'s, T>,
{
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&mut **self)
    }
}
impl<'s, T: ?Sized, V> DriveTwo<'s, V> for &mut T
where
    V: VisitTwo<'s, T>,
{
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        v.visit(&**self, &**other)
    }
}

impl<'s, A, B, V: Visit<'s, A> + Visit<'s, B>> Drive<'s, V> for (A, B) {
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        let (x, y) = self;
        v.visit(x)?;
        v.visit(y)?;
        Continue(())
    }
}
impl<'s, A, B, V: VisitMut<'s, A> + VisitMut<'s, B>> DriveMut<'s, V> for (A, B) {
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
        let (x, y) = self;
        v.visit(x)?;
        v.visit(y)?;
        Continue(())
    }
}
impl<'s, A, B, V: VisitTwo<'s, A> + VisitTwo<'s, B>> DriveTwo<'s, V> for (A, B) {
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        let (sa, sb) = self;
        let (oa, ob) = other;
        v.visit(sa, oa)?;
        v.visit(sb, ob)?;
        Continue(())
    }
}

impl<'s, A, B, C, V: Visit<'s, A> + Visit<'s, B> + Visit<'s, C>> Drive<'s, V> for (A, B, C) {
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        let (x, y, z) = self;
        v.visit(x)?;
        v.visit(y)?;
        v.visit(z)?;
        Continue(())
    }
}
impl<'s, A, B, C, V: VisitMut<'s, A> + VisitMut<'s, B> + VisitMut<'s, C>> DriveMut<'s, V>
    for (A, B, C)
{
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
        let (x, y, z) = self;
        v.visit(x)?;
        v.visit(y)?;
        v.visit(z)?;
        Continue(())
    }
}
impl<'s, A, B, C, V: VisitTwo<'s, A> + VisitTwo<'s, B> + VisitTwo<'s, C>> DriveTwo<'s, V>
    for (A, B, C)
{
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        let (sa, sb, sc) = self;
        let (oa, ob, oc) = other;
        v.visit(sa, oa)?;
        v.visit(sb, ob)?;
        v.visit(sc, oc)?;
        Continue(())
    }
}

impl<'s, A, B, V: Visit<'s, A> + Visit<'s, B>> Drive<'s, V> for Result<A, B> {
    fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
        match self {
            Ok(x) => v.visit(x)?,
            Err(x) => v.visit(x)?,
        }
        Continue(())
    }
}
impl<'s, A, B, V: VisitMut<'s, A> + VisitMut<'s, B>> DriveMut<'s, V> for Result<A, B> {
    fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
        match self {
            Ok(x) => v.visit(x)?,
            Err(x) => v.visit(x)?,
        }
        Continue(())
    }
}
impl<'s, A, B, V: VisitTwo<'s, A> + VisitTwo<'s, B>> DriveTwo<'s, V> for Result<A, B> {
    fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
        match (self, other) {
            (Ok(x), Ok(y)) => v.visit(x, y),
            (Err(x), Err(y)) => v.visit(x, y),
            _ => Break(Default::default()),
        }
    }
}

// Make an impl for an iterable type.
macro_rules! iter_impl {
        (<$($param_or_const:ident $($const_ident:ident : $const_ty:ty)?),*> $ty:ty,
            $iter:ident($iter_ty:ty),
            $iter_mut:ident($iter_mut_ty:ty)
        ) => {
            impl<'s, $($param_or_const $($const_ident : $const_ty)?,)* V> Drive<'s, V> for $ty
            where
                V: Visitor,
                V: Visit<'s, $iter_ty>,
            {
                fn drive_inner(&'s self, v: &mut V) -> ControlFlow<V::Break> {
                    for x in self.$iter() {
                        v.visit(x)?;
                    }
                    Continue(())
                }
            }
            impl<'s, $($param_or_const $($const_ident : $const_ty)?,)* V> DriveMut<'s, V> for $ty
            where
                V: Visitor,
                V: VisitMut<'s, $iter_mut_ty>,
            {
                fn drive_inner_mut(&'s mut self, v: &mut V) -> ControlFlow<V::Break> {
                    for x in self.$iter_mut() {
                        v.visit(x)?;
                    }
                    Continue(())
                }
            }
            impl<'s, $($param_or_const $($const_ident : $const_ty)?,)* V> DriveTwo<'s, V> for $ty
            where
                V: Visitor,
                V: VisitTwo<'s, $iter_ty>,
            {
                fn drive_two_inner(&'s self, other: &'s Self, v: &mut V) -> ControlFlow<V::Break> {
                    crate::drive_iter_two(self, other, v)
                }
            }
        };
    }
iter_impl!(<T> Vec<T>, iter(T), iter_mut(T));
iter_impl!(<T> Option<T>, iter(T), iter_mut(T));
iter_impl!(<T, const N: usize> [T; N], iter(T), iter_mut(T));

// Make an impl for a type without contents to visit.
macro_rules! leaf_impl {
    ($ty:ty, $($rest:tt)*) => {
        leaf_impl!($ty);
        leaf_impl!($($rest)*);
    };
    ($ty:ty) => {
        impl<'s, V: Visitor> Drive<'s, V> for $ty {
            fn drive_inner(&'s self, _: &mut V) -> ControlFlow<V::Break> {
                Continue(())
            }
        }
        impl<'s, V: Visitor> DriveMut<'s, V> for $ty {
            fn drive_inner_mut(&'s mut self, _: &mut V) -> ControlFlow<V::Break> {
                Continue(())
            }
        }
        impl<'s, V: Visitor<Break: Default>> DriveTwo<'s, V> for $ty {
            fn drive_two_inner(&'s self, other: &'s Self, _: &mut V) -> ControlFlow<V::Break> {
                if self == other {
                    Continue(())
                } else {
                    Break(Default::default())
                }
            }
        }
    };
}
leaf_impl!(bool, char, u8, u16, u32, u64, u128, usize);
leaf_impl!(i8, i16, i32, i64, i128, isize);
leaf_impl!((), String);
#[cfg(feature = "extra_impls")]
leaf_impl!(ustr::Ustr);
