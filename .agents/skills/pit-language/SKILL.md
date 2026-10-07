---
name: pit-language
description: PIT wire syntax, `pit-core` types and parser, canonical rendering and RIDs, resource references, attributes, Info files, or generic metadata. Reach this before editing or explaining a `.pit` definition or PIT identity.
---

# PIT language

PIT keeps the shared contract small: interfaces, ordered method arguments/results, four numeric carriers, resource references, and extensible attributes. That minimal core is what lets unrelated languages and transports interoperate without requiring one runtime or ABI. Ground answers in the checked-in `pit-core` source and `SPEC.md`; use this skill to preserve that property rather than importing assumptions from another IDL.

## Work sequence

1. Identify the relevant PIT construct and read its implementation in `pit-core` when behavior is unclear.
2. Keep the schema change minimal. Distinguish wire-significant metadata from documentation.
3. If changing an interface, render its canonical form and recompute its RID. A change to any RID-bearing attribute or method shape creates a different interface identity.
4. Put documentation for an established interface in `Info` when that avoids changing its RID.
5. Verify examples by parsing/rendering with `pit-core`; do not rely on prose-only assumptions.

## Stable invariants

- A RID is SHA3-256 of canonical `Interface` display output. Attribute order and method order are canonicalized by the data model; changing the represented content changes the identity.
- `R<rid>` references another interface, `Rthis` references the current interface, and bare `R` is the `ResTy::None` generic/opaque case only when accompanied by a defined interpretation.
- `take=true` means owned; `take=false` means borrowed. PIT's core type does not enforce a transport or memory-management strategy.
- `Info` is out-of-band, keyed by RID. It carries root, method, indexed parameter, and indexed return attributes.
- `pit-core` is `no_std` + `alloc`; PIT does not require a particular runtime or interop mechanism.

## Reach references when

- **Attribute names or semantics:** read [`references/attributes.md`](references/attributes.md).
- **Rust data types, parser/display API, feature gates, or `Info` structures:** read [`references/api.md`](references/api.md).
- **A code-generation backend or language mapping:** read [`../pit-integrations/SKILL.md`](../pit-integrations/SKILL.md) and its backend reference.
