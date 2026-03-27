# PIT Reserved Attributes Reference

## Interface-Level Attributes

| Attribute | Description |
|-----------|-------------|
| `name` | Human-readable display name |
| `doc` | Full documentation |
| `brief` | Short summary (≤80 chars recommended) |
| `version` | Semantic version |
| `since` | Version when introduced |
| `author` | Maintainer identifier |
| `license` | SPDX license identifier |
| `deprecated` | Deprecation notice + migration path |
| `see` | Related interfaces or documentation |
| `category` | Logical grouping |
| `tags` | Comma-separated classification tags |
| `wasmAbiVer` | WASM ABI compatibility version |
| `ridFmtVer` | RID encoding: `0`=hex (default), `1`=base64 |
| `docAttrVer` | Documentation version: `0`=basic, `1`=extended, `2`=LLM attrs |

## Method-Level Attributes

| Attribute | Description |
|-----------|-------------|
| `name` | Display name |
| `doc` | Full documentation |
| `brief` | Short summary |
| `deprecated` | Deprecation notice |
| `since` | Version when introduced |
| `throws` | Comma-separated error conditions |
| `async` | Async execution flag (`true`/`false`) |
| `idempotent` | Repeated calls have same effect |
| `pure` | No side effects |
| `example` | Usage example (may contain code) |

## Argument-Level Attributes

| Attribute | Description |
|-----------|-------------|
| `name` | Display name |
| `doc` | Documentation |
| `brief` | Short summary |
| `default` | Default value if not provided |
| `range` | Valid numeric range, e.g. `0..100` |
| `pattern` | Validation regex |
| `unit` | Unit of measurement |
| `example` | Example value |

## LLM Attributes (requires `docAttrVer=2`)

| Attribute | Description |
|-----------|-------------|
| `llm.context` | Extended context for LLM understanding |
| `llm.intent` | Intended use case or purpose |
| `llm.constraints` | Constraints the LLM should be aware of |
| `llm.examples` | Multiple examples for few-shot learning |
| `llm.related` | Related concepts or interfaces |

The `llm.` prefix and `pit.` prefix are reserved for system use. Unknown attributes outside reserved prefixes are preserved and passed through unchanged.

## Attribute Versioning

`docAttrVer` on an interface controls which documentation attributes are interpreted:

| Version | Description |
|---------|-------------|
| `0` (default) | Basic: `name`, `doc`, `brief` only |
| `1` | Extended: all standard attributes |
| `2` | LLM attributes enabled |

Example: `[docAttrVer=1][name=My Interface]{...}`

## Best Practices

- Always provide `brief` (≤80 chars) — it's what IDEs and tooling display inline
- Use `doc` for full context, caveats, and worked examples
- Set `name` for human-readable display separate from the method/interface identifier
- Mark deprecations early with a migration path: `[deprecated=Use authenticateV2 instead]`
- Set `ridFmtVer=1` on new interfaces for compact base64 cross-references
- Store rich documentation in Info files, not in the interface definition, to avoid changing the RID
