#![no_std]
use core::fmt::{Display, Formatter};
extern crate alloc;
pub trait Capnp {
    fn capnp(&self, f: &mut Formatter<'_>) -> core::fmt::Result;
}
#[derive(Clone, Copy)]
pub struct ViaCapnp<'a>(pub &'a (dyn Capnp + 'a));
impl Display for ViaCapnp<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.capnp(f)
    }
}
