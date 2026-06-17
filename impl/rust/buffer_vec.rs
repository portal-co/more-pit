//! Pure `P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5` implementations.
//! Canonical source for pit `R{rid}` adapters via rice splice.

use std::sync::Arc;

/// 32-bit buffer trait (pit-rust-generic naming).
pub trait P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 {
    type Error;
    fn read8(&mut self, offset: u32) -> Result<u32, Self::Error>;
    fn write8(&mut self, offset: u32, value: u32) -> Result<(), Self::Error>;
    fn size(&mut self) -> Result<u32, Self::Error>;
}

macro_rules! buffer_slice_impl {
    ($t:ty) => {
        impl P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for $t {
            type Error = ();
            fn read8(&mut self, p0: u32) -> Result<u32, Self::Error> {
                Ok(self[p0 as usize].into())
            }
            fn write8(&mut self, p0: u32, p1: u32) -> Result<(), Self::Error> {
                self[p0 as usize] = (p1 & 0xff) as u8;
                Ok(())
            }
            fn size(&mut self) -> Result<u32, Self::Error> {
                Ok(self.len().try_into().unwrap())
            }
        }
    };
}

buffer_slice_impl!(Vec<u8>);
buffer_slice_impl!(Box<[u8]>);
buffer_slice_impl!(&'static mut [u8]);

macro_rules! buffer_ro_slice_impl {
    ($t:ty) => {
        impl P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for $t {
            type Error = ();
            fn read8(&mut self, p0: u32) -> Result<u32, Self::Error> {
                Ok(self[p0 as usize].into())
            }
            fn write8(&mut self, _p0: u32, _p1: u32) -> Result<(), Self::Error> {
                Ok(())
            }
            fn size(&mut self) -> Result<u32, Self::Error> {
                Ok(self.len().try_into().unwrap())
            }
        }
    };
}

buffer_ro_slice_impl!(Arc<[u8]>);
buffer_ro_slice_impl!(&'static [u8]);
