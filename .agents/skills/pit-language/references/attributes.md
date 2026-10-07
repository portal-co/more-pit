# PIT attributes and `Info`

## Syntax and identity

PIT attributes render as `[name=value]`. `pit-core` parses and preserves their strings; it does not validate a universal reserved-name registry or enforce documentation versions. Attribute semantics beyond the parser are conventions of the consuming tool.

Attributes embedded in an `Interface`, `Sig`, or `Arg` are part of canonical interface rendering and therefore affect the RID. Put display-only names, prose, and other documentation in `Info` when documenting an established interface without changing its identity.

## Common documentation conventions

These names are used by the local PIT documentation/tooling conventions. They are not enforced by `pit-core`.

| Scope | Common names |
|---|---|
| Interface | `name`, `doc`, `brief`, `version`, `deprecated`, `since`, `author`, `license`, `see`, `category`, `tags` |
| Method | `name`, `doc`, `brief`, `deprecated`, `since`, `throws`, `async`, `idempotent`, `pure`, `example` |
| Argument/result | `name`, `doc`, `brief`, `default`, `range`, `pattern`, `unit`, `example` |
| LLM context | `llm.context`, `llm.intent`, `llm.constraints`, `llm.examples`, `llm.related` |

`pit-core` exposes helper methods for selected keys behind `doc-attrs`; the presence of a key is not otherwise schema validation. Use the actual fields and accessors in `pit-core/src/lib.rs` and `src/info.rs` as the API authority.

## `Info` locations

`Info` is separate from the interface and keyed by its 32-byte RID. Its entry has:

- root attributes (`InfoEntry.attrs`);
- method attributes (`MethEntry.attrs`);
- indexed 0-based parameter attributes (`MethEntry.params`);
- indexed 0-based return attributes (`MethEntry.returns`).

Text form:

```text
<64-hex-rid>: [
  root [name=Service]
  root [doc=Human-facing documentation]
  method open [name=Open resource]
  param open 0 [name=resource-id]
  return open 0 [doc=The opened resource]
]
```

When merging entries, same-name attributes are last-wins and sorted by name; method and indexed entries merge recursively. Check parser constraints in `pit-core/src/info.rs` before using punctuation in method names in `Info` text.

## PIT-specific attributes

`ridFmtVer` on an interface controls resource RID rendering (`0`/absent = hex; `1` or greater = no-padding base64). Because it is an interface attribute, it contributes to that interface's RID. `wasmAbiVer` has helper conversion methods in `pit-core::Attr`.

Generic metadata has modern keys defined in `pit_core::generics` (`generic_params.modern`, `generics.modern`) when `unstable-generics` is enabled. Generic encodings are not ordinary documentation; preserve the required feature and exact mangling contract.

For SDK lowering, see the companion [`SDK-to-PIT spec`](../../../../../portal-solutions-sdk/docs/sdk-to-pit-spec.md) when that checkout is available. Lowering markers such as `sdk.scalar` are wire-significant and must not be moved to `Info`.
