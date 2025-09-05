#![no_std]

use alloc::{collections::btree_map::BTreeMap, format, string::String, vec::Vec};
use pit_core::{Arg, Interface, Sig};
extern crate alloc;
pub type TsOpts = SwiftOpts;
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct SwiftOpts {
    // pub rewrites: BTreeMap<[u8; 32], String>,
}
impl SwiftOpts {
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        match t {
            Arg::I32 => format!("UInt32"),
            Arg::I64 => format!("UInt64"),
            Arg::F32 => format!("Float"),
            Arg::F64 => format!("Double"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match ty {
                pit_core::ResTy::None => format!("Any"),
                pit_core::ResTy::Of(a) => format!(
                    "any P{}",
                    hex::encode(a)
                ),
                pit_core::ResTy::This => format!("any P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        format!(
            "({}) -> ({})",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a} _: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            s.rets
                .iter()
              .enumerate()
                .map(|(a, b)| format!("r{a} _: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        format!(
            "public protocol P{} {{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
