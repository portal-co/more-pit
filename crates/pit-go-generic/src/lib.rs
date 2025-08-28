#![no_std]

use alloc::{collections::btree_map::BTreeMap, format, string::String, vec::Vec};
use pit_core::{Arg, Interface, Sig};
extern crate alloc;
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct GoOpts {
    pub rewrites: BTreeMap<[u8; 32], String>,
}
impl GoOpts {
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        match t {
            Arg::I32 => format!("uint32"),
            Arg::I64 => format!("uint64"),
            Arg::F32 => format!("float32"),
            Arg::F64 => format!("float64"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match ty {
                pit_core::ResTy::None => format!("interface{{}}"),
                pit_core::ResTy::Of(a) => format!(
                    "{}.P{}",
                    match self.rewrites.get(a) {
                        None => format!("pit{}", hex::encode(a)),
                        Some(b) => b.clone(),
                    },
                    hex::encode(a)
                ),
                pit_core::ResTy::This => format!("P{}", hex::encode(this)),
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        format!(
            "({}) ({})",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a} {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            s.rets
                .iter()
                .map(|x| self.ty(x, this))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        format!(
            "type P{} interface{{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
