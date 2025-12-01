# Common PIT Interface Definitions

This directory contains reusable PIT (Portal Interface Types) interface definitions for common I/O patterns.

## Files

### buffer.pit

Defines a 32-bit addressed byte buffer interface with basic read/write operations.

```pit
{
    read8(I32) -> (I32);     // Read a byte at the given 32-bit offset
    write8(I32,I32) -> ();   // Write a byte at the given 32-bit offset
    size() -> (I32)          // Get the buffer size as a 32-bit integer
}
```

**Use case**: Memory buffers, small file I/O, embedded systems with 32-bit addressing.

### buffer64.pit

Defines a 64-bit addressed byte buffer interface, similar to `buffer.pit` but with 64-bit addressing.

```pit
{
    read8(I64) -> (I32);     // Read a byte at the given 64-bit offset
    write8(I64,I32) -> ();   // Write a byte at the given 64-bit offset
    size() -> (I64)          // Get the buffer size as a 64-bit integer
}
```

**Use case**: Large files, memory-mapped I/O, 64-bit systems.

### reader.pit

Defines a reader interface that produces buffers.

```pit
{
    read(I32) -> (R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5);
    read64(I64) -> (R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d)
}
```

The resource IDs reference:
- `867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5` - The 32-bit buffer interface
- `68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d` - The 64-bit buffer interface

**Use case**: Stream reading, file input, network I/O.

### writer.pit

Defines a writer interface that consumes buffers.

```pit
{
    write(R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5) -> (I32);
    write64(R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d) -> (I64)
}
```

The resource IDs reference the same buffer interfaces as `reader.pit`.

**Use case**: Stream writing, file output, network I/O.

## PIT Format Reference

For more information about the PIT format, see:
- [pit-core README](https://github.com/portal-co/pit-core)
- [PIT Specification](https://github.com/portal-co/pit-core/blob/main/SPEC.md)
