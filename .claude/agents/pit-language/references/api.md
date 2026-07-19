# pit-core Rust API Reference

`pit-core` — CC0-1.0, `no_std` + `alloc`, Rust 2024 edition.

## Core Types

```rust
// Key-value metadata pair
struct Attr { name: String, value: String }

// Resource type identifier
enum ResTy {
    None,
    Of([u8; 32]),  // References another interface by its RID
    This,           // Self-reference
}

// Argument type
enum ArgTy {
    I32, I64, F32, F64,
    Resource {
        ty: ResTy,
        nullable: bool,
        take: bool,  // true = owned, false = borrowed (counterintuitive name!)
    }
}

// Argument with annotations
struct Arg { ty: ArgTy, ann: Vec<Attr> }

// Method signature
struct Sig { ann: Vec<Attr>, params: Vec<Arg>, rets: Vec<Arg> }

// Complete interface definition
struct Interface { methods: BTreeMap<String, Sig>, ann: Vec<Attr> }
```

## Parsing Functions (nom-based)

All return `IResult<&str, T>` and consume only as much input as needed.

```rust
fn parse_interface(input: &str) -> IResult<&str, Interface>
fn parse_sig(input: &str)       -> IResult<&str, Sig>
fn parse_arg(input: &str)       -> IResult<&str, Arg>
fn parse_attrs(input: &str)     -> IResult<&str, Vec<Attr>>
fn parse_resty(input: &str)     -> IResult<&str, ResTy>
fn ident(input: &str)           -> IResult<&str, &str>
```

## Interface Methods

```rust
impl Interface {
    fn rid(&self) -> [u8; 32]    // SHA3-256 of canonical Display output
    fn rid_str(&self) -> String  // 64-char lowercase hex
}
```

## Utility Functions

```rust
// Merge two attribute lists; same-name keys: last wins
fn merge(a: Vec<Attr>, b: Vec<Attr>) -> Vec<Attr>

// Wrap a list of Args as methods on a new Interface (for tupling)
fn retuple(args: Vec<Arg>) -> Interface
```

## Info API

```rust
// Top-level container: maps RID bytes → InfoEntry
struct Info(BTreeMap<[u8; 32], InfoEntry>)

struct InfoEntry {
    ann: Vec<Attr>,
    methods: BTreeMap<String, MethEntry>,
}

struct MethEntry {
    ann: Vec<Attr>,
}

impl Info {
    fn parse(input: &str) -> IResult<&str, Info>
    fn merge(self, other: Info) -> Info  // Last-wins merge
}

// With doc-attrs feature:
impl InfoEntry {
    fn name(&self) -> Option<&str>
    fn doc(&self) -> Option<&str>
}
impl MethEntry {
    fn name(&self) -> Option<&str>
    fn doc(&self) -> Option<&str>
}
```

## Attr Helpers (with `doc-attrs` feature)

```rust
impl Attr {
    fn as_name(&self) -> Option<&str>
    fn as_doc(&self) -> Option<&str>
    fn as_brief(&self) -> Option<&str>
    fn from_name(v: impl Into<String>) -> Self
    fn from_doc(v: impl Into<String>) -> Self
    fn from_brief(v: impl Into<String>) -> Self
}
```

## Feature Flags

| Flag | Effect |
|------|--------|
| `doc-attrs` | Attr helper methods (`as_name`, `from_doc`, etc.) and InfoEntry accessors |
| `unstable-generics` | `Arity` struct and `Mangle` trait for generic parameter encoding |
| `unstable-pcode` | `PExpr` expression tree enum (data model only, no parser/evaluator yet) |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `nom` | 8 | Parser combinators |
| `sha3` | 0.10 | SHA3-256 for RID computation |
| `base64` | 0.22 | Base64 RID encoding (`ridFmtVer=1`) |
| `hex` | 0.4 | Hex encoding/decoding |
| `derive_more` | 2 | `#[derive(Display)]` |
| `instability` | 0.3 | `#[unstable]` feature gates |

## WriteUpdate Bridge

Internally, RID computation streams UTF-8 bytes from the interface's `Display` output directly into a SHA3-256 hasher via a `WriteUpdate` bridge in `src/util.rs` — no intermediate heap allocation.
