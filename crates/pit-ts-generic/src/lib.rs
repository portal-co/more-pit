#![no_std]

use alloc::{collections::btree_map::BTreeMap, format, string::String, vec::Vec};
use pit_core::{Arg, Interface, Sig};
extern crate alloc;
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub struct TsOpts {
    pub r#async: bool,
    // pub rewrites: BTreeMap<[u8; 32], String>,
}
impl TsOpts {
    pub fn ty(&self, t: &Arg, this: [u8; 32]) -> String {
        let m = match self.r#async {
            true => "A",
            false => "",
        };
        match t {
            Arg::I32 => format!("number"),
            Arg::I64 => format!("bigint"),
            Arg::F32 => format!("number"),
            Arg::F64 => format!("number"),
            Arg::Resource {
                ty,
                nullable,
                take,
                ann,
            } => match match ty {
                pit_core::ResTy::None => format!("any"),
                pit_core::ResTy::Of(a) => format!("{m}P{}", hex::encode(a)),
                pit_core::ResTy::This => format!("{m}P{}", hex::encode(this)),
                _ => todo!(),
            }{
                ty => if *nullable{
                    format!("{ty} | undefined")
                }else{
                    ty
                }
            }
            _ => todo!(),
        }
    }
    pub fn meth(&self, s: &Sig, this: [u8; 32]) -> String {
        let j = s
            .rets
            .iter()
            .map(|x| self.ty(x, this))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "({}): [{}]{}",
            s.params
                .iter()
                .enumerate()
                .map(|(a, b)| format!("p{a}: {}", self.ty(b, this)))
                .collect::<Vec<_>>()
                .join(","),
            j,
            if self.r#async {
                format!("| Promise<[{j}]>")
            } else {
                String::default()
            }
        )
    }
    pub fn interface(&self, i: &Interface) -> String {
        let this = i.rid();
        let m = match self.r#async {
            true => "A",
            false => "",
        };
        format!(
            "export type {m}P{} = {{{}}}",
            hex::encode(this),
            i.methods
                .iter()
                .map(|(a, b)| format!("{m}P{}_{a} {}", hex::encode(this), self.meth(b, this)))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}
