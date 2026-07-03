//! Language-neutral FFI wire layout for PIT interfaces.
//!
//! Describes parameter and return field layouts aligned with [`pit-c`](../../pit/crates/pit-c)
//! `{method}_fres` structs. Backend crates (TeaVM WASM, Scala-native/C, etc.) implement
//! [`FfiBackend`] to render target-specific syntax from these layouts.

use alloc::{format, string::String, string::ToString, vec::Vec};
use pit_core::{Arg, ArgTy, Interface, ResTy, Sig};

/// Scalar type at the FFI wire boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireScalar {
    I32,
    I64,
    F32,
    F64,
    /// Resource handle (`int` at the WASM/native boundary).
    Handle,
}

/// A named field in a wire struct or parameter list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireField {
    pub name: String,
    pub ty: WireScalar,
}

/// Wire struct for method returns (mirrors `R{rid}_{method}_fres` in pit-c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireStruct {
    pub name: String,
    pub fields: Vec<WireField>,
}

/// Wire description of one interface method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodFfi {
    pub name: String,
    pub params: Vec<WireField>,
    pub fres: WireStruct,
}

/// Wire layout for an entire interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceWire {
    pub rid_hex: String,
    pub methods: Vec<MethodFfi>,
}

/// A generated source file from an [`FfiBackend`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedFile {
    pub path: String,
    pub content: String,
}

/// Backend-specific rendering from wire layouts to source files.
pub trait FfiBackend<Ctx> {
    fn emit_files(&self, iface: &Interface, ctx: &Ctx) -> Vec<EmittedFile>;
}

pub fn wire_scalar(arg: &Arg) -> WireScalar {
    match &arg.ty {
        ArgTy::I32 => WireScalar::I32,
        ArgTy::I64 => WireScalar::I64,
        ArgTy::F32 => WireScalar::F32,
        ArgTy::F64 => WireScalar::F64,
        ArgTy::Resource { .. } => WireScalar::Handle,
        _ => WireScalar::Handle,
    }
}

/// Map a PIT argument to a wire field with the given name prefix (`p` or `r`).
pub fn wire_field_from_arg(arg: &Arg, name: &str) -> WireField {
    WireField {
        name: name.to_string(),
        ty: wire_scalar(arg),
    }
}

/// Build the `{method}_fres`-style return struct for a signature.
pub fn wire_fres_struct(rid_hex: &str, method: &str, sig: &Sig) -> WireStruct {
    let fields = if sig.rets.is_empty() {
        Vec::new()
    } else {
        sig.rets
            .iter()
            .enumerate()
            .map(|(idx, arg)| wire_field_from_arg(arg, &format!("r{idx}")))
            .collect()
    };
    WireStruct {
        name: format!("P{rid_hex}_{method}_fres"),
        fields,
    }
}

/// Build wire layout for one method.
pub fn method_wire(rid_hex: &str, name: &str, sig: &Sig) -> MethodFfi {
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(idx, arg)| wire_field_from_arg(arg, &format!("p{idx}")))
        .collect();
    let fres = wire_fres_struct(rid_hex, name, sig);
    MethodFfi {
        name: name.to_string(),
        params,
        fres,
    }
}

/// Build wire layout for an interface.
pub fn interface_wire(iface: &Interface) -> InterfaceWire {
    let rid_hex = hex::encode(iface.rid());
    let methods = iface
        .methods
        .iter()
        .map(|(name, sig)| method_wire(&rid_hex, name, sig))
        .collect();
    InterfaceWire { rid_hex, methods }
}

/// C type name for a wire scalar (FFI boundary, matches pit-c `FFI` kind).
pub fn c_ffi_ty(scalar: WireScalar) -> &'static str {
    match scalar {
        WireScalar::I32 | WireScalar::Handle => "int32_t",
        WireScalar::I64 => "int64_t",
        WireScalar::F32 => "float",
        WireScalar::F64 => "double",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pit_core::parse_interface;

    #[test]
    fn buffer_read8_fres_layout() {
        let src = "{ read8(I32) -> (I32); write8(I32,I32) -> (); size() -> (I32) }";
        let (_, iface) = parse_interface(src).unwrap();
        let wire = interface_wire(&iface);
        assert_eq!(wire.methods.len(), 3);
        let read8 = &wire.methods[0];
        assert_eq!(read8.name, "read8");
        assert_eq!(read8.params.len(), 1);
        assert_eq!(read8.params[0].ty, WireScalar::I32);
        assert_eq!(read8.fres.fields.len(), 1);
        assert_eq!(read8.fres.fields[0].name, "r0");
        assert_eq!(read8.fres.fields[0].ty, WireScalar::I32);
    }

    #[test]
    fn multi_return_fres_fields() {
        let src = "{ scan(R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5&,R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5&) -> (I32,F64) }";
        let (_, iface) = parse_interface(src).unwrap();
        let wire = interface_wire(&iface);
        let scan = &wire.methods[0];
        assert_eq!(scan.fres.fields.len(), 2);
        assert_eq!(scan.fres.fields[0].ty, WireScalar::I32);
        assert_eq!(scan.fres.fields[1].ty, WireScalar::F64);
    }
}
