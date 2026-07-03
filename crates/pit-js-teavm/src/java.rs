//! Java TeaVM JS interop glue emission.

use pit_core::{Interface, Sig};

use crate::guest::{
    collect_peer_keys, guest_ret_java, guest_ty_java, method_key, param_from_js_java,
    param_to_js_java, peer_registry_key, peer_require_java, ret_from_js_java, ret_to_js_java,
};
use crate::JsTeavmContext;

pub fn emit_handler(pkg: &str) -> String {
    format!(
        r#"package {pkg};

import org.teavm.jso.JSObject;
import java.util.HashMap;
import java.util.Map;

/** Object-identity peer registry for pit-js-teavm (no WASM handles). */
public final class JsHandler {{
    private JsHandler() {{}}

    public interface PeerType<T> {{
        JSObject toJs(T value);
        T fromJs(JSObject obj);
        void finalize(T value);
    }}

    public static final class PeerRegistry {{
        private static final Map<String, PeerType<?>> BY_KEY = new HashMap<>();

        private PeerRegistry() {{}}

        public static <T> void register(String key, PeerType<T> peer) {{
            BY_KEY.put(key, peer);
        }}

        @SuppressWarnings("unchecked")
        public static <T> PeerType<T> require(String key) {{
            PeerType<?> p = BY_KEY.get(key);
            if (p == null) {{
                throw new IllegalStateException("missing JsPeer registration for " + key);
            }}
            return (PeerType<T>) p;
        }}
    }}

    public static <T> PeerType<T> nullablePeer(final PeerType<T> inner) {{
        return new PeerType<T>() {{
            @Override
            public JSObject toJs(T value) {{
                if (value == null) {{
                    return JsRuntime.nullMarker();
                }}
                return inner.toJs(value);
            }}

            @Override
            public T fromJs(JSObject obj) {{
                if (obj == null || JsRuntime.isNullMarker(obj)) {{
                    return null;
                }}
                return inner.fromJs(obj);
            }}

            @Override
            public void finalize(T value) {{
                if (value != null) {{
                    inner.finalize(value);
                }}
            }}
        }};
    }}
}}
"#
    )
}

pub fn emit_runtime(pkg: &str) -> String {
    format!(
        r#"package {pkg};

import org.teavm.jso.JSBody;
import org.teavm.jso.JSObject;
import org.teavm.jso.JSExport;

/** Thin JSObject dispatch helpers for pit-js-teavm. */
public final class JsRuntime {{
    private JsRuntime() {{}}

    @JSBody(script = "return {{ __pit_null: true }};")
    public static native JSObject nullMarker();

    @JSBody(params = {{"obj"}}, script = "return obj && obj.__pit_null === true;")
    public static native boolean isNullMarker(JSObject obj);

    @JSBody(params = {{"obj", "name", "args"}}, script = "return obj[name].apply(obj, args);")
    private static native Object callRaw(JSObject obj, String name, Object[] args);

    public static void callVoid(JSObject obj, String name, Object... args) {{
        callRaw(obj, name, args);
    }}

    public static int callInt(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        if (r instanceof Number) {{
            return ((Number) r).intValue();
        }}
        if (r instanceof Object[]) {{
            Object[] arr = (Object[]) r;
            if (arr.length > 0 && arr[0] instanceof Number) {{
                return ((Number) arr[0]).intValue();
            }}
        }}
        return 0;
    }}

    public static long callLong(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        if (r instanceof Number) {{
            return ((Number) r).longValue();
        }}
        if (r instanceof Object[]) {{
            Object[] arr = (Object[]) r;
            if (arr.length > 0 && arr[0] instanceof Number) {{
                return ((Number) arr[0]).longValue();
            }}
        }}
        return 0L;
    }}

    public static float callFloat(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        if (r instanceof Number) {{
            return ((Number) r).floatValue();
        }}
        if (r instanceof Object[]) {{
            Object[] arr = (Object[]) r;
            if (arr.length > 0 && arr[0] instanceof Number) {{
                return ((Number) arr[0]).floatValue();
            }}
        }}
        return 0f;
    }}

    public static double callDouble(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        if (r instanceof Number) {{
            return ((Number) r).doubleValue();
        }}
        if (r instanceof Object[]) {{
            Object[] arr = (Object[]) r;
            if (arr.length > 0 && arr[0] instanceof Number) {{
                return ((Number) arr[0]).doubleValue();
            }}
        }}
        return 0d;
    }}

    public static JSObject callObject(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        return (JSObject) r;
    }}

    public static Object[] callArray(JSObject obj, String name, Object... args) {{
        Object r = callRaw(obj, name, args);
        return (Object[]) r;
    }}

    @JSBody(params = {{"v"}}, script = "return v;")
    public static native Object toJsLong(long v);

    @JSBody(params = {{"v"}}, script = "return typeof v === 'bigint' ? Number(v) : v;")
    public static native long fromJsLong(Object v);

    @JSBody(params = {{"v"}}, script = "return typeof v === 'number' ? v | 0 : v;")
    public static native int fromJsInt(Object v);

    @JSBody(params = {{"v"}}, script = "return +v;")
    public static native float fromJsFloat(Object v);

    @JSBody(params = {{"v"}}, script = "return +v;")
    public static native double fromJsDouble(Object v);
}}
"#
    )
}

pub fn emit_interface_js(iface: &Interface, ctx: &JsTeavmContext<'_>) -> String {
    let rid = iface.rid_str();
    let pkg = ctx.java_pkg;
    let generics = 0u32;
    let peer_inits = emit_java_peer_registry_inits(iface, generics);
    let export_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_java_export_method(&rid, name, sig, generics))
        .collect::<Vec<_>>()
        .join("\n\n");
    let proxy_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_java_proxy_method(&rid, name, sig, generics))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"package {pkg};

import org.teavm.jso.JSExport;
import org.teavm.jso.JSObject;

/** TeaVM JS object bridge for P{rid} (no WASM wire). */
public final class P{rid}Js {{
    public static final JsHandler.PeerType<P{rid}> PEER = new JsHandler.PeerType<P{rid}>() {{
        @Override
        public JSObject toJs(P{rid} value) {{
            return wrap(value);
        }}

        @Override
        public P{rid} fromJs(JSObject obj) {{
            return JsProxy.fromJs(obj);
        }}

        @Override
        public void finalize(P{rid} value) {{
        }}
    }};

    static {{
        JsHandler.PeerRegistry.register("P{rid}", PEER);
{peer_inits}
    }}

    @JSExport
    public static JSObject wrap(P{rid} impl) {{
        return (JSObject) (Object) new ExportObject(impl);
    }}

    public static P{rid} fromJs(JSObject obj) {{
        return JsProxy.fromJs(obj);
    }}

    @JSExport
    public static final class ExportObject extends JSObject {{
        final P{rid} impl;

        public ExportObject(P{rid} impl) {{
            this.impl = impl;
        }}

{export_methods}
    }}

    public static final class JsProxy {{
        private JsProxy() {{}}

        public static P{rid} fromJs(final JSObject obj) {{
            return new P{rid}() {{
{proxy_methods}
            }};
        }}
    }}
}}
"#
    )
}

fn emit_java_peer_registry_inits(iface: &Interface, generics: u32) -> String {
    let rid = iface.rid_str();
    let keys = collect_peer_keys(iface);
    keys.into_iter()
        .filter(|k| k != &format!("P{rid}"))
        .map(|k| {
            if k.contains(" | null") || k.starts_with("Option[") {
                let inner = k
                    .replace(" | null", "")
                    .trim_start_matches("Option[")
                    .trim_end_matches(']')
                    .to_string();
                format!(
                    "        JsHandler.PeerRegistry.register(\"{k}\", JsHandler.nullablePeer(JsHandler.PeerRegistry.require(\"{inner}\")));"
                )
            } else {
                format!(
                    "        JsHandler.PeerRegistry.register(\"{k}\", P{k}Js.PEER);"
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn param_to_export_java(expr: &str, arg: &pit_core::Arg, rid: &str, generics: u32) -> String {
    if matches!(arg.ty, pit_core::ArgTy::Resource { .. }) {
        format!(
            "({}) {}.fromJs((JSObject) {expr})",
            guest_ty_java(arg, rid),
            peer_require_java(arg, rid, generics)
        )
    } else {
        expr.to_string()
    }
}

fn emit_java_export_method(rid: &str, name: &str, sig: &Sig, generics: u32) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{} p{i}", guest_ty_java(a, rid)))
        .collect::<Vec<_>>()
        .join(", ");
    let java_args = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| param_to_export_java(&format!("p{i}"), a, rid, generics))
        .collect::<Vec<_>>()
        .join(", ");
    let call = if java_args.is_empty() {
        format!("impl.{name}()")
    } else {
        format!("impl.{name}({java_args})")
    };

    if sig.rets.is_empty() {
        return format!(
            r#"            @JSExport
            public void {key}({params}) {{
                {call};
            }}"#
        );
    }

    if sig.rets.len() == 1 {
        let ret_ty = guest_ret_java(sig, rid);
        let ret = &sig.rets[0];
        let return_stmt = if matches!(ret.ty, pit_core::ArgTy::Resource { .. } | pit_core::ArgTy::I64) {
            format!("return {};", ret_to_js_java("ret", ret, rid, generics))
        } else {
            "return ret;".to_string()
        };
        return format!(
            r#"            @JSExport
            public Object {key}({params}) {{
                {ret_ty} ret = {call};
                {return_stmt}
            }}"#
        );
    }

    format!(
        r#"            @JSExport
            public Object {key}({params}) {{
                return {call};
            }}"#
    )
}

fn emit_java_proxy_method(rid: &str, name: &str, sig: &Sig, generics: u32) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{} p{i}", guest_ty_java(a, rid)))
        .collect::<Vec<_>>()
        .join(", ");
    let js_args: Vec<String> = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| param_to_js_java(&format!("p{i}"), a, rid, generics))
        .collect();
    let args_array = if js_args.is_empty() {
        "new Object[0]".to_string()
    } else {
        format!("new Object[] {{ {} }}", js_args.join(", "))
    };

    let ret = guest_ret_java(sig, rid);

    if sig.rets.is_empty() {
        return format!(
            r#"                @Override
                public {ret} {name}({params}) {{
                    JsRuntime.callVoid(obj, "{key}", {args_array});
                }}"#
        );
    }

    if sig.rets.len() == 1 {
        let a = &sig.rets[0];
        let call = match &a.ty {
            pit_core::ArgTy::I32 => format!("JsRuntime.callInt(obj, \"{key}\", {args_array})"),
            pit_core::ArgTy::I64 => format!("JsRuntime.callLong(obj, \"{key}\", {args_array})"),
            pit_core::ArgTy::F32 => format!("JsRuntime.callFloat(obj, \"{key}\", {args_array})"),
            pit_core::ArgTy::F64 => format!("JsRuntime.callDouble(obj, \"{key}\", {args_array})"),
            pit_core::ArgTy::Resource { .. } => {
                format!(
                    "({}) {}",
                    guest_ty_java(a, rid),
                    ret_from_js_java(
                        &format!("JsRuntime.callObject(obj, \"{key}\", {args_array})"),
                        a,
                        rid,
                        generics
                    )
                )
            }
            _ => format!("JsRuntime.callObject(obj, \"{key}\", {args_array})"),
        };
        return format!(
            r#"                @Override
                public {ret} {name}({params}) {{
                    return {call};
                }}"#
        );
    }

    let unpack = sig
        .rets
        .iter()
        .enumerate()
        .map(|(i, a)| {
            format!(
                "{} r{i} = {};",
                guest_ty_java(a, rid),
                ret_from_js_java(&format!("arr[{i}]"), a, rid, generics)
            )
        })
        .collect::<Vec<_>>()
        .join("\n                    ");

    format!(
        r#"                @Override
                public {ret} {name}({params}) {{
                    Object[] arr = JsRuntime.callArray(obj, "{key}", {args_array});
                    {unpack}
                    return new Object[] {{ {} }};
                }}"#,
        (0..sig.rets.len())
            .map(|i| format!("r{i}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
