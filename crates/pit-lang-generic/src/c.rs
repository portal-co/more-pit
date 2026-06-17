//! # C backend (`pit_lang_generic::c`)
//!
//! Code generator for C header macros from PIT (Portal Interface Types) interfaces.
//!
//! Unlike the four OOP-style backends this module uses a `Display`-based rendering
//! pipeline driven by the [`C`] wrapper type and the [`PureC`] context.
//!
//! ## Generated code pattern
//!
//! Compatible with [interface99](https://github.com/hirrolot/interface99).
//!
//! ```c
//! #define <prefix><hex_id>_t_IFACE_<method>(CUR,METH) \
//!     vfunc(RetType, METH, VSelf, T0 p0, T1 p1)
//! #define <prefix><hex_id>_t_IFACE \
//!     <prefix><hex_id>_t_IFACE_m1(<prefix><hex_id>_t, m1) \
//!     <prefix><hex_id>_t_IFACE_m2(<prefix><hex_id>_t, m2)
//! interface(<prefix><hex_id>_t)
//! ```
//!
//! Return-type rules:
//! - 0 returns → `void`
//! - 1 return  → the C type directly
//! - N returns → `struct { T r0; T r1; }` (anonymous struct)
//!
//! Borrow / ownership flags on resource arguments are ignored; borrowless
//! languages use the resource type name directly.  Nullable resources are
//! represented as pointers (`T *`).
//!
//! ## Example
//!
//! ```ignore
//! use pit_lang_generic::c::{C, PureC};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* … */;
//! println!("{}", C { value: &iface, kind: PureC { cx: "my_" } });
//! ```

use core::fmt::Display;

pub mod __ {
    //! Internal re-export used by the [`c_disp!`] macro.
    pub use core;
}

// ─────────────────────────────────────────────────────────────────────────────
// Core types
// ─────────────────────────────────────────────────────────────────────────────

/// A wrapper that pairs a value `T` with a rendering context `Kind`.
///
/// Implement [`core::fmt::Display`] via the [`c_disp!`] macro.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct C<T, Kind> {
    /// The wrapped value to render.
    pub value: T,
    /// The rendering context / configuration.
    pub kind: Kind,
}

/// Context for C code generation.
///
/// `cx` is prepended to every generated type name, e.g. `"my_"` → `my_<hex>_t`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PureC<T> {
    /// Prefix string for generated type names.
    pub cx: T,
}

// ─────────────────────────────────────────────────────────────────────────────
// c_disp! macro
// ─────────────────────────────────────────────────────────────────────────────

/// Implement `Display` on a [`C`]-wrapped type for both owned and borrowed values.
///
/// # Syntax
///
/// ```ignore
/// c_disp!(<T: Display>[Kind] ValueType => |self_pat, fmt_pat, kind_pat| { expr });
/// ```
#[macro_export]
macro_rules! c_disp {
    ($(<$($g:ident $(: $gp:path)?),*>)? [$k:ty] $t:ty => |$self:pat_param, $fmt:pat_param, $kind:pat_param| $a:expr) => {
        impl $(<$($g : $($gp)?),*>)? $crate::c::__::core::fmt::Display
            for $crate::c::C<$t, $k>
        {
            fn fmt(
                &self,
                f: &mut $crate::c::__::core::fmt::Formatter<'_>,
            ) -> $crate::c::__::core::result::Result<(), $crate::c::__::core::fmt::Error> {
                match &self.value {
                    $self => match f {
                        $fmt => match &self.kind {
                            $kind => $a,
                        },
                    },
                }
            }
        }
        impl<'a $(,$($g : $($gp)?),*)?> $crate::c::__::core::fmt::Display
            for $crate::c::C<&'a $t, $k>
        {
            fn fmt(
                &self,
                f: &mut $crate::c::__::core::fmt::Formatter<'_>,
            ) -> $crate::c::__::core::result::Result<(), $crate::c::__::core::fmt::Error> {
                match &*self.value {
                    $self => match f {
                        $fmt => match &self.kind {
                            $kind => $a,
                        },
                    },
                }
            }
        }
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// Display implementations
// ─────────────────────────────────────────────────────────────────────────────

// Arg
//
// Borrow flags are ignored: all resource types appear by value/pointer.
// Nullable resources are rendered as `T *` (pointer, implying nullability).
c_disp!(<T: Display>[PureC<T>] pit_core::Arg => |this, f, kind| match &this.ty {
    pit_core::ArgTy::I32 => write!(f, "uint32_t"),
    pit_core::ArgTy::I64 => write!(f, "uint64_t"),
    pit_core::ArgTy::F32 => write!(f, "float"),
    pit_core::ArgTy::F64 => write!(f, "double"),
    pit_core::ArgTy::Resource { ty, nullable, .. } => {
        // `take` (borrow vs owned) is deliberately ignored — abstract C
        // interface definitions do not distinguish ownership.
        let ptr = if *nullable { " *" } else { "" };
        match ty {
            pit_core::ResTy::None  => write!(f, "void{ptr}"),
            pit_core::ResTy::Of(a) => write!(f, "{}{}_t{ptr}", &kind.cx, hex::encode(a)),
            pit_core::ResTy::This  => write!(f, "CUR{ptr}"),
            _                      => todo!(),
        }
    }
    _ => todo!(),
});

// Sig
c_disp!(<T: Display>[PureC<T>] pit_core::Sig => |this, f, kind| {
    // ── vfunc( ────────────────────────────────────────────────────────────────────────
    write!(f, "vfunc(")?;
    // ── return type ──────────────────────────────────────────────────────────
    match this.rets.as_slice() {
        // 0 returns → void
        [] => write!(f, "void")?,
        // 1 return → the C type directly (no wrapping struct)
        [r] => write!(f, "{}", C { value: r, kind: PureC { cx: &kind.cx } })?,
        // N returns → anonymous struct with semicolon-separated fields
        rs => {
            write!(f, "struct{{")?;
            for (i, r) in rs.iter().enumerate() {
                write!(f, "{} r{i};", C { value: r, kind: PureC { cx: &kind.cx } })?;
            }
            write!(f, "}}")?;
        }
    }
    // ── method name placeholder + self ───────────────────────────────────────
    write!(f, ",METH,VSelf")?;
    // ── parameters (comma-prefixed so VSelf has no trailing comma) ───────────
    for (i, p) in this.params.iter().enumerate() {
        write!(f, ",{} p{i}", C { value: p, kind: PureC { cx: &kind.cx } })?;
    }
    write!(f, ")")?;
    Ok(())
});

// Interface
c_disp!(<T: Display>[PureC<T>] pit_core::Interface => |this, f, kind| {
    let cx = &kind.cx;
    let hex = hex::encode(this.rid());
    for (meth, sig) in this.methods.iter() {
        write!(
            f,
            "#define {cx}{hex}_t_IFACE_{meth}(CUR,METH) {}\n",
            C { value: sig, kind: PureC { cx: &kind.cx } }
        )?;
    }
    write!(f, "#define {cx}{hex}_t_IFACE ")?;
    for meth in this.methods.keys() {
        write!(f, "{cx}{hex}_t_IFACE_{meth}({cx}{hex}_t,{meth}) ")?;
    }
    write!(f, "\n")?;
    write!(f, "interface({cx}{hex}_t)\n")
});
