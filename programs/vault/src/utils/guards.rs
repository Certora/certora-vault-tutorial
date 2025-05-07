#[cfg(not(feature = "certora"))]
mod inner {
    #[rustfmt::skip]
    macro_rules! impl_bin_require {
        ($name: ident, $pred: tt, $err: expr $dollar: tt) => {
            #[macro_export]
            macro_rules! $name {
                    ($lhs: expr, $rhs: expr $dollar(, $desc: literal)? ) => {{
                        if $lhs $pred $rhs { } else { return Err($err); }
                    }};
                }
            pub use $name;
        };
    }
    pub(crate) use impl_bin_require;
}

#[cfg(feature = "certora")]
mod inner {
    #[rustfmt::skip]
    macro_rules! impl_bin_require {
        ($name: ident, $pred: tt, $dollar: tt) => {
            #[macro_export]
            macro_rules! $name {
                    ($lhs: expr, $rhs: expr, $err: expr $dollar(, $desc: literal)? ) => {{
                        if $lhs $pred $rhs { } else { panic!(); }
                    }};
                }
            pub use $name;
        };
    }
    pub(crate) use impl_bin_require;


    macro_rules! require {
        ($cond: expr, $err: expr $(, $desc:literal)? ) => {{
            if $cond  { } else { panic!(); } 

        }};
    }

    pub(crate) use require;
}

pub(crate) use inner::*;
impl_bin_require!(require_gt, >, $);
impl_bin_require!(require_ge, >=, $);
impl_bin_require!(require_eq, ==, $);
impl_bin_require!(require_ne, !=, $);
impl_bin_require!(require_lt, <, $);
impl_bin_require!(require_le, <=, $);
