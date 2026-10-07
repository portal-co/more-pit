# `pit-core` API reference

Use the local `pit-core` source as the final authority. This is a navigation reference for the public Rust model and its checked-in parser/renderer.

## Core model (`pit_core`)

```rust
pub struct Attr { pub name: String, pub value: String }
pub struct Arity { pub to_fill: BTreeMap<String, Arity> }

pub enum ResTy { None, Of([u8; 32]), This }
pub enum ArgTy {
    I32, I64, F32, F64,
    Resource { ty: ResTy, nullable: bool, take: bool },
}
pub struct Arg { pub ty: ArgTy, pub ann: Vec<Attr> }
pub struct Sig { pub ann: Vec<Attr>, pub params: Vec<Arg>, pub rets: Vec<Arg> }
pub struct Interface { pub methods: BTreeMap<String, Sig>, pub ann: Vec<Attr> }
```

`ArgTy` and `ResTy` are non-exhaustive. `take=true` means the resource is taken/owned; `take=false` means borrowed. The core model records these flags but does not prescribe runtime behavior.

`Interface::rid()` returns `[u8; 32]`, SHA3-256 over canonical `Display` output; `rid_str()` returns lowercase hexadecimal. Interface methods are ordered by name; argument and return order remains positional. Standalone argument/resource display uses hex RIDs; interface rendering consults `ridFmtVer` for resource-ID encoding.

## Parsing and construction

The main parsers are public free functions returning `nom::IResult<&str, T>`:

```rust
ident(&str)
parse_balanced(&str)
parse_attr(&str)
parse_attrs(&str)
parse_resty(&str)
parse_arg(&str)
parse_sig(&str)
parse_interface(&str)
```

They parse a prefix and return unconsumed input; callers that require a complete document must check that the remainder is empty (allowing only the whitespace their format permits). `Arg` provides primitive/resource constructors and `with_attr`; `ArgTy` provides `into_arg` and `with_attrs`. `merge(Vec<Attr>, Vec<Attr>)` is last-wins by attribute name and returns name-sorted attributes. `retuple(Vec<Arg>)` creates `v0`, `v1`, … methods, each returning one argument.

## Generic metadata

`Arity` is always part of the core model. Encoding helpers are under `pit_core::generics` with the `unstable-generics` feature. Relevant items include `Mangle`, `Mangled`, `Param`, `ARITY_KEY`, `GENERIC_KEY`, `resolve_interface_generics`, `resolve_sig_generics`, and `resolve_instance`.

The modern arity/generic encodings are carried by attributes. Do not substitute a legacy `generics`/`instance` encoding unless the consuming implementation explicitly requires it.

## Out-of-band `Info` (`pit_core::info`)

```rust
pub struct Info { pub interfaces: BTreeMap<[u8; 32], InfoEntry> }
pub struct InfoEntry { pub attrs: Vec<Attr>, pub methods: BTreeMap<String, MethEntry> }
pub struct MethEntry {
    pub attrs: Vec<Attr>,
    pub params: BTreeMap<usize, ParamEntry>,
    pub returns: BTreeMap<usize, ParamEntry>,
}
pub struct ParamEntry { pub attrs: Vec<Attr> }
```

`Info` is keyed by interface RID. `InfoEntry` applies root attributes; each `MethEntry` applies method attributes and indexed 0-based parameter/return attributes. All levels provide `merge`; attributes use the core last-wins merge behavior. `Info`, `InfoEntry`, and the entry types implement parsing/display as documented in `pit-core/src/info.rs`.

With `doc-attrs`, accessors such as `name()`, `doc()`, `brief()`, `deprecated()`, `llm_context()`, `llm_intent()`, `category()`, `since()`, and `get_attr()` are available on the corresponding metadata types. `Attr` helper constructors/accessors are also feature-gated.

## Feature flags

| Feature | Surface |
|---|---|
| `doc-attrs` | Documentation attribute helpers/accessors |
| `unstable-generics` | Generic mangling and resolution module |
| `unstable-pcode` | Pcode expression data model |

PIT core is `no_std` + `alloc`. For exact feature gates and method signatures, inspect `pit-core/Cargo.toml`, `src/lib.rs`, `src/info.rs`, and `src/generics.rs`.
