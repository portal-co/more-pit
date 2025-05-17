#![no_std]
pub mod __ {
    pub use core;
}
use core::marker::PhantomData;
extern crate alloc;
// #[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct C<T, Kind> {
    pub value: T,
    pub kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PureC;
#[macro_export]
macro_rules! c_disp {
    ($(<$($g:ident $(: $gp:path)?),*>)? [$k:ty] $t:ty => |$self:ident, $fmt:ident, $kind:ident|$a:expr) => {
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
c_disp!([PureC]pit_core::Arg => |this,f,kind|match this{
    pit_core::Arg::I32 => write!(f,"uint32_t"),
    pit_core::Arg::I64 => write!(f,"uint64_t"),
    pit_core::Arg::F32 => write!(f,"float"),
    pit_core::Arg::F64 => write!(f,"double"),
    pit_core::Arg::Resource { ty, nullable, take, ann } => match ty{
        pit_core::ResTy::None => write!(f,"Any_T"),
        pit_core::ResTy::Of(a) => write!(f,"Cx{}_t",hex::encode(a)),
        pit_core::ResTy::This => write!(f,"CUR"),
        _ => todo!(),
    },
    _ => todo!(),
});
c_disp!([PureC]pit_core::Sig => |this,f,kind|{
    write!(f,"vfunc(struct{{")?;
    for (i,r) in this.rets.iter().enumerate(){
        write!(f,"{} r{i}",C{value:r,kind:PureC})?;
    }
    write!(f,"}},METH,VSelf,")?;
    for (i,p) in this.params.iter().enumerate(){
        write!(f,"{} p{i}",C{value:p,kind:PureC})?;
    };
    Ok(())
});
c_disp!([PureC]pit_core::Interface => |this,f,kind|{
    write!(f,"#define CUR Cx{}_t\n",hex::encode(this.rid()))?;
    for (k,m) in this.methods.iter(){
        write!(f,"#define METH {k}\n")?;
        write!(f,"#define Cx{}_t_IFACE_{k} _X({})\n",hex::encode(this.rid()),C{value:m,kind:PureC})?;
    }
    write!(f,"#define Cx{}_t_IFACE _X(",hex::encode(this.rid()))?;
    for k in this.methods.keys(){
        write!(f,"Cx{}_t_IFACE_{k} ",hex::encode(this.rid()))?;
    }
    write!(f,")\n")?;
    write!(f,"interface(Cx{}_t)\n#undef CUR\n#undef METH\n",hex::encode(this.rid()))?;
    for k in this.methods.keys(){
        write!(f,"#undef Cx{}_t_IFACE_{k}\n",hex::encode(this.rid()))?;
    }
    Ok(())
});
