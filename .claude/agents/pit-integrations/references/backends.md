# pit-lang-generic Backend Type Mappings

## C Backend

Generates `#define` macros using the `vfunc`/`interface` macro pattern for vtables.

```c
#define <prefix><hex>_t_IFACE_methodName(CUR, METH) vfunc(CUR, METH, ...)
#define interface_<prefix><hex>_t(IMPL) interface(...)
```

The `C` struct has one config field: `pub prefix: String` (default `""`). The C backend is entirely `Display`-only — no heap allocations at all. `src/c.rs` (~155 lines) is the canonical example of the Display-only rendering pattern.

No type mapping table — C uses raw macro parameters.

## Go Backend

Generates Go interface types.

| PIT | Go |
|-----|-----|
| `I32` | `uint32` |
| `I64` | `uint64` |
| `F32` | `float32` |
| `F64` | `float64` |
| `R<id>` (owned) | `P<hex>` |
| `R<id>&` (borrowed) | `*P<hex>` |
| `R<id>n` (nullable) | pointer or nil-able type |

Cross-package resource references use `opts.rewrites`: `rid_bytes → "pkg.TypeName"`.

Generated form:
```go
type P0123cdef interface {
    P0123cdef_methodName(p0 uint32) (uint64, uint64)
}
```

## Haxe Backend

Generates Haxe interface definitions.

| PIT | Haxe |
|-----|------|
| `I32` | `haxe.Int32` |
| `I64` | `haxe.Int32` ⚠️ same as I32 — known limitation |
| `F32` | `Float` |
| `F64` | `Float` |

Cross-package resource references use `opts.rewrites`.

Generated form:
```haxe
interface P0123cdef {
    P0123cdef_methodName(p0: haxe.Int32): {r0: Float}
}
```

## TypeScript Backend

Generates exported TypeScript type objects. All numeric types map to `number`.

Generated form:
```ts
export type P0123cdef = {
    P0123cdef_methodName(p0: number): [number]
}
```

## TypeScriptAsync Backend

Same as TypeScript but:
- Return types are `[T] | Promise<[T]>`
- Type names prefixed `AP<hex>` instead of `P<hex>`

## Swift Backend

Generates Swift protocols with `open` modifier.

| PIT | Swift |
|-----|-------|
| `I32` | `UInt32` |
| `I64` | `UInt64` |
| `F32` | `Float` |
| `F64` | `Double` |

Generated form:
```swift
open protocol P0123cdef {
    open func P0123cdef_methodName(p0 _: UInt32) -> (r0 _: UInt64)
}
```

---

# pit-rust-generic Type Mappings

## Primitive Types

| PIT | Rust |
|-----|------|
| `I32` | `u32` |
| `I64` | `u64` |
| `F32` | `f32` |
| `F64` | `f64` |

## Resource Arguments → Trait Bounds

Resource arguments become trait bounds rather than concrete types, preserving PIT's ownership semantics in Rust's type system:

| PIT arg | Rust bound |
|---------|-----------|
| `R<id>` owned, non-null | `impl P<id_hex><'bound, Error = Self::Error> + 'bound` |
| `R<id>&` borrowed, non-null | `impl DerefMut<Target = impl P<id_hex><'bound, ...>> + 'bound` |
| `R<id>n` owned, nullable | `Option<impl P<id_hex><'bound, ...> + 'bound>` |
| `R<id>n&` borrowed, nullable | `Option<impl DerefMut<...> + 'bound>` |

The `'bound` lifetime is the trait's lifetime parameter, tying resource lifetimes to the trait impl.

## Generated Structure

```rust
// Error type
pub struct P<hex>Error {}
impl core::fmt::Debug   for P<hex>Error { ... }
impl core::fmt::Display for P<hex>Error { ... }
impl core::error::Error for P<hex>Error {}

// Trait
pub trait P<hex><'bound> {
    type Error;
    fn P<hex>_methodName(&mut self, p0: u32) -> Result<u64, Self::Error>;
    // with asyncness: Some(_):
    async fn P<hex>_methodName(&mut self, p0: u32) -> Result<u64, Self::Error>;
}

// With specialization: true — default impl for unimplemented types
default impl<'bound, T> P<hex><'bound> for T {
    type Error = P<hex>Error;
    fn P<hex>_methodName(&mut self, ...) -> Result<..., Self::Error> {
        Err(P<hex>Error{})
    }
}
```
