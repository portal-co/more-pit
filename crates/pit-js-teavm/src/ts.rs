//! TypeScript adapter emission for pit-js-teavm.

use pit_core::{ArgTy, Interface, Sig};
use pit_lang_generic::{Opts, Syntax, TypeScript};

use crate::guest::{dep_rid_hexs, method_key, peer_registry_key, resource_rid_hex, ts_ty};

pub fn emit_runtime() -> String {
    r#"// Shared pit-js-teavm runtime (object-to-object bridge, no WASM wire).
export type PitJsObject = Record<string, unknown>;

export interface PeerType<T> {
  toJs(value: T): PitJsObject;
  fromJs(obj: PitJsObject): T;
}

const registry = new Map<string, PeerType<unknown>>();

export function registerPeer<T>(key: string, peer: PeerType<T>): void {
  registry.set(key, peer as PeerType<unknown>);
}

export function requirePeer<T>(key: string): PeerType<T> {
  const peer = registry.get(key);
  if (!peer) {
    throw new Error(`missing JsPeer registration for ${key}`);
  }
  return peer as PeerType<T>;
}

export function toJsPeer<T>(key: string, impl: T): PitJsObject {
  return requirePeer<T>(key).toJs(impl);
}

export function fromJsPeer<T>(key: string, obj: PitJsObject): T {
  return requirePeer<T>(key).fromJs(obj);
}

function isNullish(v: unknown): boolean {
  return v === null || v === undefined;
}

export function nullablePeer<T>(inner: PeerType<T>): PeerType<T | undefined> {
  return {
    toJs(value) {
      if (isNullish(value)) {
        return { __pit_null: true };
      }
      return inner.toJs(value as T);
    },
    fromJs(obj) {
      if (isNullish(obj) || (obj as { __pit_null?: boolean }).__pit_null === true) {
        return undefined;
      }
      return inner.fromJs(obj);
    },
  };
}
"#
    .to_string()
}

pub fn emit_interface_js(iface: &Interface) -> String {
    let rid = iface.rid_str();
    let iface_rid = iface.rid();
    let generics = 0u32;
    let peer_key = format!("P{rid}");

    let mut imports = format!(
        "import type {{ P{rid} }} from \"./P{rid}\";\nimport {{\n  registerPeer,\n  toJsPeer,\n  fromJsPeer,\n  nullablePeer,\n  requirePeer,\n  type PitJsObject,\n  type PeerType,\n}} from \"./pit-js-runtime\";\n"
    );

    for dep_hex in dep_rid_hexs(iface) {
        imports.push_str(&format!(
            "import type {{ P{dep_hex} }} from \"./P{dep_hex}\";\nimport {{ toJs as toJsP{dep_hex}, fromJs as fromJsP{dep_hex} }} from \"./P{dep_hex}_js\";\n"
        ));
    }

    let to_js_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_ts_to_js_method(&rid, name, sig, iface_rid, generics))
        .collect::<Vec<_>>()
        .join(",\n");

    let from_js_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_ts_from_js_method(&rid, name, sig, iface_rid, generics))
        .collect::<Vec<_>>()
        .join(",\n");

    let peer_registrations = emit_ts_peer_registrations(iface, generics);

    format!(
        r#"{imports}
const PEER_KEY = "{peer_key}";

export function toJs(impl: P{rid}): PitJsObject {{
  return toJsPeer(PEER_KEY, impl);
}}

export function fromJs(obj: PitJsObject): P{rid} {{
  return fromJsPeer(PEER_KEY, obj);
}}

const peer: PeerType<P{rid}> = {{
  toJs(impl) {{
    return {{
{to_js_methods}
    }};
  }},
  fromJs(obj) {{
    return {{
{from_js_methods}
    }} as P{rid};
  }},
}};

registerPeer(PEER_KEY, peer);
{peer_registrations}
"#
    )
}

fn emit_ts_peer_registrations(iface: &Interface, generics: u32) -> String {
    let rid = iface.rid_str();
    let mut lines = Vec::new();
    for sig in iface.methods.values() {
        for arg in sig.params.iter().chain(sig.rets.iter()) {
            if let ArgTy::Resource { nullable: true, .. } = &arg.ty {
                let key = peer_registry_key(arg, &rid, generics);
                if let Some(dep_hex) = resource_rid_hex(arg, &rid) {
                    if dep_hex != rid {
                        lines.push(format!(
                            "registerPeer(\"{key}\", nullablePeer(requirePeer(\"P{dep_hex}\")));"
                        ));
                    } else {
                        lines.push(format!(
                            "registerPeer(\"{key}\", nullablePeer(peer));"
                        ));
                    }
                }
            }
        }
    }
    lines.sort();
    lines.dedup();
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    }
}

fn emit_ts_to_js_method(
    rid: &str,
    name: &str,
    sig: &Sig,
    iface_rid: [u8; 32],
    generics: u32,
) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("p{i}: {}", ts_ty(a, iface_rid)))
        .collect::<Vec<_>>()
        .join(", ");

    let args = (0..sig.params.len())
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(", ");

    let call = if args.is_empty() {
        format!("impl.{key}()")
    } else {
        format!("impl.{key}({args})")
    };

    if sig.rets.is_empty() {
        return format!("      {key}: ({params}) => {{ {call}; }}");
    }

    if sig.rets.len() == 1 {
        let ret = &sig.rets[0];
        if matches!(ret.ty, ArgTy::Resource { .. }) {
            if let Some(dep_hex) = resource_rid_hex(ret, rid) {
                let to_js = if dep_hex == rid {
                    "toJs".to_string()
                } else {
                    format!("toJsP{dep_hex}")
                };
                if let ArgTy::Resource { nullable: true, .. } = &ret.ty {
                    return format!(
                        "      {key}: ({params}) => {{ const ret = {call}; return ret[0] === undefined ? undefined : {to_js}(ret[0]); }}"
                    );
                }
                return format!(
                    "      {key}: ({params}) => {{ const ret = {call}; return {to_js}(ret[0]); }}"
                );
            }
        }
        return format!("      {key}: ({params}) => {call}");
    }

    format!("      {key}: ({params}) => {call}")
}

fn emit_ts_from_js_method(
    rid: &str,
    name: &str,
    sig: &Sig,
    iface_rid: [u8; 32],
    generics: u32,
) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("p{i}: {}", ts_ty(a, iface_rid)))
        .collect::<Vec<_>>()
        .join(", ");

    let args: Vec<String> = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| ts_param_to_js_call(a, rid, generics, &format!("p{i}")))
        .collect();

    let call_args = args.join(", ");
    let fn_access = format!("(obj as Record<string, Function>)['{key}']");
    let call = if call_args.is_empty() {
        format!("{fn_access}()")
    } else {
        format!("{fn_access}({call_args})")
    };

    if sig.rets.is_empty() {
        return format!("      {key}: ({params}) => {{ {call}; }}");
    }

    if sig.rets.len() == 1 {
        let ret = &sig.rets[0];
        if matches!(ret.ty, ArgTy::Resource { .. }) {
            if let Some(dep_hex) = resource_rid_hex(ret, rid) {
                let from = if dep_hex == rid {
                    format!("fromJs({call} as PitJsObject)")
                } else {
                    format!("fromJsP{dep_hex}({call} as PitJsObject)")
                };
                if let ArgTy::Resource { nullable: true, .. } = &ret.ty {
                    return format!(
                        "      {key}: ({params}) => [{call} === undefined || {call} === null ? undefined : {from}]"
                    );
                }
                return format!("      {key}: ({params}) => [{from}]");
            }
        }
        return format!("      {key}: ({params}) => {call}");
    }

    format!("      {key}: ({params}) => {call}")
}

fn ts_param_to_js(arg: &pit_core::Arg, rid: &str, generics: u32, expr: &str) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        if let Some(dep_hex) = resource_rid_hex(arg, rid) {
            let key = peer_registry_key(arg, rid, generics);
            if dep_hex == rid {
                return format!("toJsPeer(\"{key}\", {expr})");
            }
            return format!("toJsP{dep_hex}({expr})");
        }
    }
    expr.to_string()
}

fn ts_param_to_js_call(arg: &pit_core::Arg, rid: &str, _generics: u32, expr: &str) -> String {
    if matches!(arg.ty, ArgTy::Resource { .. }) {
        if let Some(dep_hex) = resource_rid_hex(arg, rid) {
            let to_js = if dep_hex == rid {
                "toJs".to_string()
            } else {
                format!("toJsP{dep_hex}")
            };
            if let ArgTy::Resource { nullable: true, .. } = &arg.ty {
                return format!("{expr} === undefined ? undefined : {to_js}({expr})");
            }
            return format!("{to_js}({expr})");
        }
    }
    if matches!(arg.ty, ArgTy::I64) {
        return format!("BigInt({expr})");
    }
    expr.to_string()
}
