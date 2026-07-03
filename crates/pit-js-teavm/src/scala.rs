//! Scala TeaVM JS interop glue emission.

use pit_core::{Interface, Sig};

use crate::guest::{
    collect_peer_keys, guest_ret_scala, guest_ty_scala, method_key, param_from_js_scala,
    param_to_js_scala, peer_registry_key, peer_summon_scala, ret_from_js_scala, ret_to_js_scala,
};
use crate::JsTeavmContext;

pub fn emit_handler(pkg: &str) -> String {
    format!(
        r#"package {pkg}

import org.teavm.jso.JSObject
import scala.collection.mutable

/** Object-identity peer registry for pit-js-teavm (no WASM handles). */
object JsHandler {{
  trait JsPeer[T] {{
    def toJs(value: T): JSObject
    def fromJs(obj: JSObject): T
    def finalize(value: T): Unit
  }}

  object PeerRegistry {{
    private val byKey = mutable.Map.empty[String, JsPeer[?]]

    def register[T](key: String, peer: JsPeer[T]): Unit =
      byKey(key) = peer

    def require[T](key: String): JsPeer[T] =
      byKey.getOrElse(
        key,
        throw new IllegalStateException(s"missing JsPeer registration for $key")
      ).asInstanceOf[JsPeer[T]]
  }}

  def nullablePeer[T](inner: JsPeer[T]): JsPeer[T | Null] = new JsPeer[T | Null] {{
    def toJs(value: T | Null): JSObject =
      if (value == null) JsRuntime.nullMarker() else inner.toJs(value.asInstanceOf[T])

    def fromJs(obj: JSObject): T | Null =
      if (obj == null || JsRuntime.isNullMarker(obj)) null else inner.fromJs(obj)

    def finalize(value: T | Null): Unit =
      if (value != null) inner.finalize(value.asInstanceOf[T])
  }}
}}
"#
    )
}

pub fn emit_runtime(pkg: &str) -> String {
    format!(
        r#"package {pkg}

import org.teavm.jso.{{JSBody, JSObject}}

/** Thin JSObject dispatch helpers for pit-js-teavm. */
object JsRuntime {{
  @JSBody(script = "return {{ __pit_null: true }};")
  def nullMarker(): JSObject = ???

  @JSBody(params = Array("obj"), script = "return obj && obj.__pit_null === true;")
  def isNullMarker(obj: JSObject): Boolean = ???

  @JSBody(params = Array("obj", "name", "args"), script = "return obj[name].apply(obj, args);")
  private def callRaw(obj: JSObject, name: String, args: Array[Object]): Object = ???

  def callVoid(obj: JSObject, name: String, args: Any*): Unit =
    {{ val _ = callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray); () }}

  def callInt(obj: JSObject, name: String, args: Any*): Int = {{
    val r = callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray)
    r match {{
      case n: Number => n.intValue()
      case arr: Array[Object] if arr.length > 0 && arr(0).isInstanceOf[Number] =>
        arr(0).asInstanceOf[Number].intValue()
      case _ => 0
    }}
  }}

  def callLong(obj: JSObject, name: String, args: Any*): Long = {{
    val r = callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray)
    r match {{
      case n: Number => n.longValue()
      case arr: Array[Object] if arr.length > 0 && arr(0).isInstanceOf[Number] =>
        arr(0).asInstanceOf[Number].longValue()
      case _ => 0L
    }}
  }}

  def callFloat(obj: JSObject, name: String, args: Any*): Float = {{
    val r = callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray)
    r match {{
      case n: Number => n.floatValue()
      case arr: Array[Object] if arr.length > 0 && arr(0).isInstanceOf[Number] =>
        arr(0).asInstanceOf[Number].floatValue()
      case _ => 0f
    }}
  }}

  def callDouble(obj: JSObject, name: String, args: Any*): Double = {{
    val r = callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray)
    r match {{
      case n: Number => n.doubleValue()
      case arr: Array[Object] if arr.length > 0 && arr(0).isInstanceOf[Number] =>
        arr(0).asInstanceOf[Number].doubleValue()
      case _ => 0d
    }}
  }}

  def callObject(obj: JSObject, name: String, args: Any*): JSObject =
    callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray).asInstanceOf[JSObject]

  def callArray(obj: JSObject, name: String, args: Any*): Array[Object] =
    callRaw(obj, name, args.map(_.asInstanceOf[Object]).toArray).asInstanceOf[Array[Object]]

  @JSBody(params = Array("v"), script = "return v;")
  def toJsLong(v: Long): Object = ???

  @JSBody(params = Array("v"), script = "return typeof v === 'bigint' ? Number(v) : v;")
  def fromJsLong(v: Object): Long = ???

  @JSBody(params = Array("v"), script = "return typeof v === 'number' ? v | 0 : v;")
  def fromJsInt(v: Object): Int = ???

  @JSBody(params = Array("v"), script = "return +v;")
  def fromJsFloat(v: Object): Float = ???

  @JSBody(params = Array("v"), script = "return +v;")
  def fromJsDouble(v: Object): Double = ???
}}
"#
    )
}

pub fn emit_interface_js(iface: &Interface, ctx: &JsTeavmContext<'_>) -> String {
    let rid = iface.rid_str();
    let pkg = ctx.scala_pkg;
    let generics = 0u32;
    let peer_inits = emit_scala_peer_registry_inits(iface, generics);
    let export_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_scala_export_method(&rid, name, sig, generics))
        .collect::<Vec<_>>()
        .join("\n\n");
    let proxy_methods = iface
        .methods
        .iter()
        .map(|(name, sig)| emit_scala_proxy_method(&rid, name, sig, generics))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"package {pkg}

import org.teavm.jso.{{JSExport, JSObject}}

/** TeaVM JS object bridge for P{rid} (no WASM wire). */
object P{rid}Js {{
  given peer: JsHandler.JsPeer[P{rid}] = new JsHandler.JsPeer[P{rid}] {{
    def toJs(value: P{rid}): JSObject = wrap(value)
    def fromJs(obj: JSObject): P{rid} = JsProxy.fromJs(obj)
    def finalize(value: P{rid}): Unit = ()
  }}

  locally {{
    JsHandler.PeerRegistry.register("P{rid}", peer)
{peer_inits}
  }}

  @JSExport
  def wrap(impl: P{rid}): JSObject = JsExport.wrap(impl)

  def fromJs(obj: JSObject): P{rid} = JsProxy.fromJs(obj)

  final class ExportObject(val impl: P{rid}) extends JSObject {{
{export_methods}
  }}

  object JsExport {{
    def wrap(impl: P{rid}): JSObject = new ExportObject(impl)
  }}

  object JsProxy {{
    def fromJs(obj: JSObject): P{rid} = new P{rid} {{
{proxy_methods}
    }}
  }}
}}
"#
    )
}

fn emit_scala_peer_registry_inits(iface: &Interface, generics: u32) -> String {
    let rid = iface.rid_str();
    let keys = collect_peer_keys(iface);
    keys.into_iter()
        .filter(|k| k != &format!("P{rid}"))
        .map(|k| {
            if k.starts_with("Option[") {
                let inner = k.trim_start_matches("Option[").trim_end_matches(']').to_string();
                format!(
                    "    JsHandler.PeerRegistry.register(\"{k}\", JsHandler.nullablePeer(summon[JsHandler.JsPeer[{inner}]]))"
                )
            } else {
                format!("    JsHandler.PeerRegistry.register(\"{k}\", P{k}Js.peer)")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn param_to_export_scala(expr: &str, arg: &pit_core::Arg, rid: &str, generics: u32) -> String {
    if matches!(arg.ty, pit_core::ArgTy::Resource { .. }) {
        format!(
            "{}.fromJs({expr}.asInstanceOf[JSObject])",
            peer_summon_scala(arg, rid, generics)
        )
    } else {
        expr.to_string()
    }
}

fn emit_scala_export_method(rid: &str, name: &str, sig: &Sig, generics: u32) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("p{i}: {}", guest_ty_scala(a, rid, generics)))
        .collect::<Vec<_>>()
        .join(", ");
    let scala_args = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| param_to_export_scala(&format!("p{i}"), a, rid, generics))
        .collect::<Vec<_>>()
        .join(", ");
    let call = if scala_args.is_empty() {
        format!("impl.{name}()")
    } else {
        format!("impl.{name}({scala_args})")
    };

    if sig.rets.is_empty() {
        return format!(
            r#"      @JSExport
      def {key}({params}): Unit =
        {call}"#
        );
    }

    if sig.rets.len() == 1 {
        let ret_ty = guest_ret_scala(sig, rid, generics);
        let ret = &sig.rets[0];
        let return_expr = if matches!(ret.ty, pit_core::ArgTy::Resource { .. } | pit_core::ArgTy::I64) {
            ret_to_js_scala("ret", ret, rid, generics)
        } else {
            "ret.asInstanceOf[Object]".to_string()
        };
        return format!(
            r#"      @JSExport
      def {key}({params}): Object =
        val ret: {ret_ty} = {call}
        {return_expr}"#
        );
    }

    format!(
        r#"      @JSExport
      def {key}({params}): Array[Object] =
        val arr = {call}.asInstanceOf[Array[Object]]
        arr"#
    )
}

fn emit_scala_proxy_method(rid: &str, name: &str, sig: &Sig, generics: u32) -> String {
    let key = method_key(rid, name);
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| format!("p{i}: {}", guest_ty_scala(a, rid, generics)))
        .collect::<Vec<_>>()
        .join(", ");
    let js_args: Vec<String> = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, a)| param_to_js_scala(&format!("p{i}"), a, rid, generics))
        .collect();
    let args = if js_args.is_empty() {
        "Seq.empty".to_string()
    } else {
        format!("Seq({})", js_args.join(", "))
    };

    let ret = guest_ret_scala(sig, rid, generics);

    if sig.rets.is_empty() {
        return format!(
            r#"      def {name}({params}): Unit =
        JsRuntime.callVoid(obj, "{key}", {args}*)"#
        );
    }

    if sig.rets.len() == 1 {
        let a = &sig.rets[0];
        let call = match &a.ty {
            pit_core::ArgTy::I32 => format!("JsRuntime.callInt(obj, \"{key}\", {args}*)"),
            pit_core::ArgTy::I64 => format!("JsRuntime.callLong(obj, \"{key}\", {args}*)"),
            pit_core::ArgTy::F32 => format!("JsRuntime.callFloat(obj, \"{key}\", {args}*)"),
            pit_core::ArgTy::F64 => format!("JsRuntime.callDouble(obj, \"{key}\", {args}*)"),
            pit_core::ArgTy::Resource { .. } => ret_from_js_scala(
                &format!("JsRuntime.callObject(obj, \"{key}\", {args}*)"),
                a,
                rid,
                generics,
            ),
            _ => format!("JsRuntime.callObject(obj, \"{key}\", {args}*)"),
        };
        return format!(
            r#"      def {name}({params}): {ret} =
        {call}"#
        );
    }

    format!(
        r#"      def {name}({params}): {ret} =
        JsRuntime.callArray(obj, "{key}", {args}*).asInstanceOf[{ret}]"#
    )
}
