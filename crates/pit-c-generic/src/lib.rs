//! # pit-c-generic
//!
//! Code generator for C header macros from PIT (Portal Interface Types) interfaces.
//!
//! This `no_std` compatible crate generates C preprocessor macros from PIT interface definitions.
//! The generated C code uses the `vfunc` macro pattern for virtual function tables, enabling
//! runtime polymorphism in C.
//!
//! ## Overview
//!
//! The main types are:
//! - [`C`] - A wrapper that pairs a value with a rendering context (kind)
//! - [`PureC`] - A context type for rendering C code
//! - [`c_disp!`] - A macro for implementing `Display` for wrapped types
//!
//! ## Generated Code Pattern
//!
//! The generated macros define:
//! - `<prefix><hex_id>_t_IFACE_<method>` - Per-method interface macro
//! - `<prefix><hex_id>_t_IFACE` - Combined interface macro
//! - `interface(<prefix><hex_id>_t)` - Interface instantiation
//!
//! ## Example
//!
//! ```ignore
//! use pit_c_generic::{C, PureC};
//! use pit_core::Interface;
//! use std::fmt::Write;
//!
//! let iface: Interface = /* parse from PIT format */;
//! let c = C { value: &iface, kind: PureC { cx: "my_" } };
//! println!("{}", c);
//! ```
//!
//! ## Features
//!
//! - `unstable-sdk` - Enable portal-solutions-sdk integration
//! - `unstable-pcode` - Enable pcode expression support
//! - `unstable-generics` - Enable generic parameter support

#![no_std]
pub mod __ {
    //! Internal module re-exporting core for use in macros.
    pub use core;
}
use core::{fmt::Display, marker::PhantomData};
extern crate alloc;

/// A wrapper that pairs a value with a rendering context.
///
/// Used to implement `Display` for types that need additional context
/// to render properly (such as a prefix or configuration options).
///
/// # Type Parameters
///
/// * `T` - The wrapped value type
/// * `Kind` - The context/configuration type
// #[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct C<T, Kind> {
    /// The wrapped value to be rendered.
    pub value: T,
    /// The rendering context/configuration.
    pub kind: Kind,
}

/// A simple context type for C code generation.
///
/// Contains a configurable prefix used when generating type names.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PureC<T> {
    /// The prefix to use for generated type names.
    pub cx: T,
}

/// Macro for implementing `Display` on [`C`]-wrapped types.
///
/// This macro generates `Display` implementations for both owned and borrowed
/// values wrapped in [`C`], enabling uniform formatting regardless of ownership.
///
/// # Syntax
///
/// ```ignore
/// c_disp!(<T: Display>[Kind] ValueType => |self, fmt, kind| { /* format expression */ });
/// ```
#[macro_export]
macro_rules! c_disp {
    ($(<$($g:ident $(: $gp:path)?),*>)? [$k:ty] $t:ty => |$self:pat_param, $fmt:pat_param, $kind:pat_param|$a:expr) => {
        impl $(<$($g : $($gp)?),*>)? $crate::__::core::fmt::Display for $crate::C<$t, $k> {
            fn fmt(
                &self,
                f: &mut $crate::__::core::fmt::Formatter<'_>,
            ) -> $crate::__::core::result::Result<(), $crate::__::core::fmt::Error> {
                match &self.value {
                    $self => match f {
                        $fmt => match &self.kind{
                            $kind => $a,
                        },
                    },
                }
            }
        }
        impl<'a $(,$($g : $($gp)?),*)?> $crate::__::core::fmt::Display for $crate::C<&'a $t, $k> {
            fn fmt(
                &self,
                f: &mut $crate::__::core::fmt::Formatter<'_>,
            ) -> $crate::__::core::result::Result<(), $crate::__::core::fmt::Error> {
                match &*self.value {
                    $self => match f {
                        $fmt => match &self.kind{
                            $kind => $a,
                        },
                    },
                }
            }
        }
    };
}
c_disp!(<T: Display>[PureC<T>]pit_core::Arg => |this,f,kind|match this{
    pit_core::Arg::I32 => write!(f,"uint32_t"),
    pit_core::Arg::I64 => write!(f,"uint64_t"),
    pit_core::Arg::F32 => write!(f,"float"),
    pit_core::Arg::F64 => write!(f,"double"),
    pit_core::Arg::Resource { ty, nullable, take, ann } => match ty{
        pit_core::ResTy::None => write!(f,"Any_T"),
        pit_core::ResTy::Of(a) => write!(f,"{}{}_t",&kind.cx,hex::encode(a)),
        pit_core::ResTy::This => write!(f,"CUR"),
        _ => todo!(),
    },
    _ => todo!(),
});
c_disp!(<T: Display>[PureC<T>]pit_core::Sig => |this,f,kind|{
    write!(f,"vfunc(struct{{")?;
    for (i,r) in this.rets.iter().enumerate(){
        write!(f,"{} r{i}",C{value:r,kind:PureC{cx:&kind.cx}})?;
    }
    write!(f,"}},METH,VSelf,")?;
    for (i,p) in this.params.iter().enumerate(){
        write!(f,"{} p{i}",C{value:p,kind:PureC{cx:&kind.cx}})?;
    };
    Ok(())
});
c_disp!(<T: Display>[PureC<T>]pit_core::Interface => |this,f,kind|{
    let cx = &kind.cx;
    // write!(f,"#define CUR {cx}{}_t\n",hex::encode(this.rid()))?;
    for (k,m) in this.methods.iter(){
        // write!(f,"#define METH {k}\n")?;
        write!(f,"#define {cx}{}_t_IFACE_{k}(CUR,METH) {}\n",hex::encode(this.rid()),C{value:m,kind:PureC{cx:&kind.cx}})?;
    }
    write!(f,"#define {cx}{}_t_IFACE ",hex::encode(this.rid()))?;
    for k in this.methods.keys(){
        write!(f,"{cx}{}_t_IFACE_{k}({cx}{0}_t,{k}) ",hex::encode(this.rid()))?;
    }
    write!(f,"\n")?;
    write!(f,"interface({cx}{}_t)\n",hex::encode(this.rid()))?;
    Ok(())
});
