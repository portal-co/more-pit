use pit_core::generics::{resolve_interface_generics, ResolvedArity};
use pit_core::{Arg, ArgTy, Interface, ResTy, Sig};
use pit_lang_generic::{Java, Opts, Scala, Syntax};

pub fn iface_generics_strings(iface: &Interface) -> (String, String, u32) {
    let iface_generics = resolve_interface_generics(&iface.ann);
    let generics = iface_generics.params.len() as u32;
    let bounds = (0..generics)
        .map(|a| format!("T{a}: JsPeer"))
        .collect::<Vec<_>>()
        .join(",");
    let params = (0..generics)
        .map(|a| format!("T{a}"))
        .collect::<Vec<_>>()
        .join(",");
    let bounds = if bounds.is_empty() {
        String::new()
    } else {
        format!("[{bounds}]")
    };
    let params = if params.is_empty() {
        String::new()
    } else {
        format!("[{params}]")
    };
    (bounds, params, generics)
}

pub fn guest_ty_java(arg: &Arg, rid: &str) -> String {
    format!("{}", Opts::<Java>::default().ty(arg, hex_rid(rid)))
}

pub fn guest_ty_scala(arg: &Arg, rid: &str, iface_generics: u32) -> String {
    let _ = iface_generics;
    format!("{}", Opts::<Scala>::default().ty(arg, parse_rid(rid)))
}

pub fn guest_ret_java(sig: &Sig, rid: &str) -> String {
    if sig.rets.is_empty() {
        "void".to_string()
    } else if sig.rets.len() == 1 {
        guest_ty_java(&sig.rets[0], rid)
    } else {
        sig.rets
            .iter()
            .map(|a| guest_ty_java(a, rid))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn guest_ret_scala(sig: &Sig, rid: &str, iface_generics: u32) -> String {
    if sig.rets.is_empty() {
        "Unit".to_string()
    } else if sig.rets.len() == 1 {
        guest_ty_scala(&sig.rets[0], rid, iface_generics)
    } else {
        format!(
            "({})",
            sig.rets
                .iter()
                .map(|a| guest_ty_scala(a, rid, iface_generics))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn peer_registry_key(arg: &Arg, rid: &str, iface_generics: u32) -> String {
    let guest = guest_ty_scala(arg, rid, iface_generics);
    format!("{guest}")
}

pub fn peer_require_java(arg: &Arg, rid: &str, iface_generics: u32) -> String {
    let inner = format!(
        "JsHandler.PeerRegistry.require(\"{}\")",
        peer_registry_key(arg, rid, iface_generics)
    );
    if let ArgTy::Resource { nullable: true, .. } = &arg.ty {
        format!("JsHandler.nullablePeer({inner})")
    } else {
        inner
    }
}

pub fn peer_summon_scala(arg: &Arg, rid: &str, iface_generics: u32) -> String {
    format!(
        "summon[JsPeer[{}]]",
        peer_registry_key(arg, rid, iface_generics)
    )
}

pub fn param_to_js_java(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.toJs({expr})",
            peer_require_java(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.toJsLong({expr})")
    } else {
        expr.to_string()
    }
}

pub fn param_from_js_java(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "({}) {}.fromJs({expr})",
            guest_ty_java(arg, rid),
            peer_require_java(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.fromJsLong({expr})")
    } else if matches!(arg.ty, ArgTy::I32) {
        format!("JsRuntime.fromJsInt({expr})")
    } else if matches!(arg.ty, ArgTy::F32) {
        format!("JsRuntime.fromJsFloat({expr})")
    } else if matches!(arg.ty, ArgTy::F64) {
        format!("JsRuntime.fromJsDouble({expr})")
    } else {
        expr.to_string()
    }
}

pub fn ret_to_js_java(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.toJs({expr})",
            peer_require_java(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.toJsLong({expr})")
    } else {
        expr.to_string()
    }
}

pub fn ret_from_js_java(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "({}) {}.fromJs({expr})",
            guest_ty_java(arg, rid),
            peer_require_java(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.fromJsLong({expr})")
    } else if matches!(arg.ty, ArgTy::I32) {
        format!("JsRuntime.fromJsInt({expr})")
    } else if matches!(arg.ty, ArgTy::F32) {
        format!("JsRuntime.fromJsFloat({expr})")
    } else if matches!(arg.ty, ArgTy::F64) {
        format!("JsRuntime.fromJsDouble({expr})")
    } else {
        expr.to_string()
    }
}

pub fn param_to_js_scala(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.toJs({expr})",
            peer_summon_scala(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.toJsLong({expr})")
    } else {
        expr.to_string()
    }
}

pub fn param_from_js_scala(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.fromJs({expr})",
            peer_summon_scala(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.fromJsLong({expr})")
    } else if matches!(arg.ty, ArgTy::I32) {
        format!("JsRuntime.fromJsInt({expr})")
    } else if matches!(arg.ty, ArgTy::F32) {
        format!("JsRuntime.fromJsFloat({expr})")
    } else if matches!(arg.ty, ArgTy::F64) {
        format!("JsRuntime.fromJsDouble({expr})")
    } else {
        expr.to_string()
    }
}

pub fn ret_to_js_scala(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.toJs({expr})",
            peer_summon_scala(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.toJsLong({expr})")
    } else {
        expr.to_string()
    }
}

pub fn ret_from_js_scala(expr: &str, arg: &Arg, rid: &str, iface_generics: u32) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        format!(
            "{}.fromJs({expr})",
            peer_summon_scala(arg, rid, iface_generics)
        )
    } else if matches!(arg.ty, ArgTy::I64) {
        format!("JsRuntime.fromJsLong({expr})")
    } else if matches!(arg.ty, ArgTy::I32) {
        format!("JsRuntime.fromJsInt({expr})")
    } else if matches!(arg.ty, ArgTy::F32) {
        format!("JsRuntime.fromJsFloat({expr})"
        )
    } else if matches!(arg.ty, ArgTy::F64) {
        format!("JsRuntime.fromJsDouble({expr})")
    } else {
        expr.to_string()
    }
}

pub fn ts_ty(arg: &Arg, iface_rid: [u8; 32]) -> String {
    Opts::<pit_lang_generic::TypeScript>::default()
        .ty(arg, iface_rid)
        .to_string()
}

pub fn method_key(rid: &str, method: &str) -> String {
    format!("P{rid}_{method}")
}

fn parse_rid(rid: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    hex::decode_to_slice(rid, &mut out).expect("valid rid hex");
    out
}

fn hex_rid(rid: &str) -> [u8; 32] {
    parse_rid(rid)
}

pub fn collect_peer_keys(iface: &Interface) -> Vec<String> {
    let (_, _, generics) = iface_generics_strings(iface);
    let rid = iface.rid_str();
    let mut keys = std::collections::BTreeSet::new();
    keys.insert(format!("P{rid}"));
    for sig in iface.methods.values() {
        for arg in sig.params.iter().chain(sig.rets.iter()) {
            if matches!(arg.ty, ArgTy::Resource { .. }) {
                keys.insert(peer_registry_key(arg, &rid, generics));
            }
        }
    }
    keys.into_iter().collect()
}

pub fn resource_rid_hex(arg: &Arg, iface_rid: &str) -> Option<String> {
    match &arg.ty {
        ArgTy::Resource { ty, .. } => Some(match ty {
            ResTy::Of(x) => hex::encode(x),
            ResTy::This => iface_rid.to_string(),
            _ => return None,
        }),
        _ => None,
    }
}

pub fn dep_rid_hexs(iface: &Interface) -> Vec<String> {
    let this = iface.rid();
    let mut seen = std::collections::BTreeSet::new();
    for sig in iface.methods.values() {
        for arg in sig.params.iter().chain(sig.rets.iter()) {
            if let ArgTy::Resource { ty: ResTy::Of(id), .. } = &arg.ty {
                if *id != this {
                    seen.insert(hex::encode(id));
                }
            }
        }
    }
    seen.into_iter().collect()
}
