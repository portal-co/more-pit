//! Pure buffer helper functions (pit-rust-generic style).

use super::P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5;

/// Copy bytes between two 32-bit buffers.
pub fn copy<'a, B: P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5<'a>>(
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
pub fn slice<'a, B: P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5<'a>>(
    x: &mut B,
) -> impl Iterator<Item = u8> + '_ {
    (0..x.size().unwrap_or(0)).map(|i| (x.read8(i).unwrap_or(0) & 0xff) as u8)
}
