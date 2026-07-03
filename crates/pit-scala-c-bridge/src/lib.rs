//! Scala ↔ C native FFI backend (no TeaVM / WASM).
//!
//! Renders C struct headers and Scala `@extern` stubs from [`pit_lang_generic::wire`] layouts.

use pit_core::Interface;
use pit_lang_generic::wire::{
    c_ffi_ty, interface_wire, EmittedFile, FfiBackend, InterfaceWire, MethodFfi, WireScalar,
};

pub struct ScalaNativeContext<'a> {
    pub scala_pkg: &'a str,
    pub c_prefix: &'a str,
}

pub struct ScalaNativeBackend;

impl FfiBackend<ScalaNativeContext<'_>> for ScalaNativeBackend {
    fn emit_files(&self, iface: &Interface, ctx: &ScalaNativeContext<'_>) -> Vec<EmittedFile> {
        let rid = iface.rid_str();
        let wire = interface_wire(iface);
        vec![
            EmittedFile {
                path: format!("P{rid}_native.h"),
                content: emit_c_header(ctx, &wire),
            },
            EmittedFile {
                path: format!("P{rid}Native.scala"),
                content: emit_scala_native(ctx, iface, &wire),
            },
        ]
    }
}

fn emit_c_header(ctx: &ScalaNativeContext<'_>, wire: &InterfaceWire) -> String {
    let mut structs = String::new();
    for m in &wire.methods {
        if m.fres.fields.is_empty() {
            continue;
        }
        let fields = m
            .fres
            .fields
            .iter()
            .map(|f| format!("    {} {};", c_ffi_ty(f.ty), f.name))
            .collect::<Vec<_>>()
            .join("\n");
        structs.push_str(&format!(
            "typedef struct {prefix}{name}_fres {{\n{fields}\n}} {prefix}{name}_fres;\n\n",
            prefix = ctx.c_prefix,
            name = m.name,
            fields = fields,
        ));
    }
    format!(
        "#ifndef {guard}\n#define {guard}\n#include <stdint.h>\n\n{structs}#endif\n",
        guard = format!(
            "{}P{}_NATIVE_H",
            ctx.c_prefix.to_uppercase(),
            wire.rid_hex.to_uppercase()
        ),
        structs = structs,
    )
}

fn emit_scala_native(
    ctx: &ScalaNativeContext<'_>,
    iface: &Interface,
    wire: &InterfaceWire,
) -> String {
    let rid = iface.rid_str();
    let externs = wire
        .methods
        .iter()
        .map(|m| emit_scala_extern(ctx, &rid, m))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "package {pkg}\n\nimport scala.scalanative._\nimport scala.scalanative.libc._\n\n/** Native C FFI for P{rid} (wire layout from pit-lang-generic). */\nobject P{rid}Native {{\n{externs}\n}}\n",
        pkg = ctx.scala_pkg,
        rid = rid,
        externs = externs,
    )
}

fn emit_scala_extern(ctx: &ScalaNativeContext<'_>, rid: &str, m: &MethodFfi) -> String {
    let link_name = format!("p{rid}_{}", m.name);
    let params = m
        .params
        .iter()
        .map(|f| format!("{}: {}", f.name, scala_wire_ty(f.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    if m.fres.fields.is_empty() {
        let sig = if params.is_empty() {
            "(): Unit".to_string()
        } else {
            format!("({params}): Unit")
        };
        return format!(
            "  @extern(\"{link_name}\")\n  def {name}{sig} = ()",
            name = m.name,
            sig = sig,
        );
    }
    let fres = format!("{prefix}{name}_fres", prefix = ctx.c_prefix, name = m.name);
    let sig = if params.is_empty() {
        format!("(): {fres}")
    } else {
        format!("({params}): {fres}")
    };
    format!(
        "  @extern(\"{link_name}\")\n  def {name}{sig} = ???",
        name = m.name,
        sig = sig,
    )
}

fn scala_wire_ty(scalar: WireScalar) -> &'static str {
    match scalar {
        WireScalar::I32 | WireScalar::Handle => "Int",
        WireScalar::I64 => "Long",
        WireScalar::F32 => "Float",
        WireScalar::F64 => "Double",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pit_core::parse_interface;

    #[test]
    fn emits_header_and_scala_for_buffer_read8() {
        let src = "{ read8(I32) -> (I32); write8(I32,I32) -> (); size() -> (I32) }";
        let (_, iface) = parse_interface(src).unwrap();
        let ctx = ScalaNativeContext {
            scala_pkg: "pit.native",
            c_prefix: "P",
        };
        let files = ScalaNativeBackend.emit_files(&iface, &ctx);
        assert_eq!(files.len(), 2);
        assert!(files[0].content.contains("r0"));
        assert!(files[1].content.contains("@extern"));
        assert!(files[1].content.contains("read8"));
    }
}
