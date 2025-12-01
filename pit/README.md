# PIT Interface Definitions

This directory contains PIT (Portal Interface Types) interface definition files (`.pit` files).

## Directory Structure

- `common/` - Reusable interface definitions for common I/O patterns (buffers, readers, writers)

## PIT File Format

PIT files define interfaces using a compact, parseable syntax. Each file typically contains a single interface definition.

### Basic Syntax

```pit
[attribute=value]{
    method_name(ParamType1, ParamType2) -> (ReturnType1, ReturnType2);
}
```

### Primitive Types

- `I32` - 32-bit integer
- `I64` - 64-bit integer
- `F32` - 32-bit floating point
- `F64` - 64-bit floating point

### Resource Types

- `R<hex_id>` - Reference to another interface by its 32-byte hex ID
- `Rthis` - Self-reference to the current interface

### Resource Modifiers

- `n` suffix - Nullable (can be null/undefined)
- `&` suffix - Borrowed reference (not owned)

## Computing Interface IDs

Interface IDs are 32-byte SHA3-256 hashes of the canonical interface representation. These IDs are used for:
- Cross-interface references
- Runtime type checking
- Interface versioning

## Related Documentation

- [pit-core](https://github.com/portal-co/pit-core) - Core PIT parsing library
- [PIT Specification](https://github.com/portal-co/pit-core/blob/main/SPEC.md) - Full format specification
