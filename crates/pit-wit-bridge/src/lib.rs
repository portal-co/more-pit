#![no_std]
extern crate alloc;
use core::fmt::{Display, Formatter};
pub trait ToWIT {
    fn to_wit(&self, f: &mut Formatter<'_>) -> core::fmt::Result;
}
#[derive(Clone, Copy)]
pub struct ViaWIT<'a>(pub &'a (dyn ToWIT + 'a));
impl Display for ViaWIT<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.0.to_wit(f)
    }
}
