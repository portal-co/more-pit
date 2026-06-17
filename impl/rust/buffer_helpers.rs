//! Pure buffer helper functions (pit-rust-generic style).

/// Copy bytes between two 32-bit buffers.
pub fn copy<'a, B: P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>(
    a: &mut B,
    ai: u32,
    b: &mut B,
    bi: u32,
) -> Result<(), B::Error> {
    let l = (a.size()? - ai).min(b.size()? - bi);
    for i in 0..l {
        a.write8(ai + i, b.read8(bi + i)?)?;
    }
    Ok(())
}

/// Iterator over buffer bytes.
pub fn slice<'a, B: P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>(
    x: &'a mut B,
) -> impl Iterator<Item = u8> + 'a {
    (0..x.size().unwrap_or(0)).map(|i| (x.read8(i).unwrap_or(0) & 0xff) as u8)
}

pub trait P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 {
    type Error;
    fn read8(&mut self, offset: u32) -> Result<u32, Self::Error>;
    fn write8(&mut self, offset: u32, value: u32) -> Result<(), Self::Error>;
    fn size(&mut self) -> Result<u32, Self::Error>;
}
