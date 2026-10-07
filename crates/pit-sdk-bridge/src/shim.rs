use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;

use hex::encode as hex_encode;
use pit_core::{Arg, ArgTy, ResTy, info::ParamEntry};

use crate::LoweredSdk;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfacePlan {
    pub source_identity: String,
    pub rid: [u8; 32],
    pub methods: BTreeMap<String, MethodPlan>,
    pub aliases: Vec<AliasPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasPlan {
    pub source_identity: String,
    pub target_identity: String,
    pub target_rid: [u8; 32],
    pub source_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodPlan {
    pub args: Vec<ValuePlan>,
    pub returns: Vec<ValuePlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValuePlan {
    pub name: String,
    pub source_type: String,
    pub kind: ValueKind,
    pub any_reason: Option<String>,
    pub target_identity: Option<String>,
    pub target_rid: Option<[u8; 32]>,
    pub self_ref: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    I32,
    I64,
    F32,
    F64,
    Byte,
    I16,
    Any,
    Interface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShimPlanError {
    MissingInterfaceInfo([u8; 32]),
    UnknownInfoVersion {
        rid: [u8; 32],
        version: Option<String>,
    },
    MissingMarker(String),
    InvalidMarker(String),
    InconsistentWireType(String),
}

impl fmt::Display for ShimPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInterfaceInfo(rid) => {
                write!(f, "missing PIT Info entry for RID {}", hex_encode(rid))
            }
            Self::UnknownInfoVersion { rid, version } => write!(
                f,
                "unsupported SDK lowering Info version {:?} for RID {}",
                version,
                hex_encode(rid)
            ),
            Self::MissingMarker(marker) => write!(f, "missing SDK lowering marker {marker}"),
            Self::InvalidMarker(marker) => write!(f, "invalid SDK lowering marker {marker}"),
            Self::InconsistentWireType(where_) => {
                write!(f, "PIT wire type conflicts with SDK metadata at {where_}")
            }
        }
    }
}

impl std::error::Error for ShimPlanError {}

impl LoweredSdk {
    /// Reconstructs converter plans from PIT signatures and source-indexed `Info`.
    /// The plan parser rejects unknown Info versions and inconsistent metadata rather
    /// than guessing how an untyped resource should be materialized.
    pub fn shim_plans(&self) -> Result<Vec<InterfacePlan>, ShimPlanError> {
        let mut plans = Vec::new();
        for (rid, interface) in &self.interfaces {
            let info = self
                .info
                .interfaces
                .get(rid)
                .ok_or(ShimPlanError::MissingInterfaceInfo(*rid))?;
            let version = marker(&info.attrs, "sdk.lowering.version").map(str::to_owned);
            if version.as_deref() != Some("1") {
                return Err(ShimPlanError::UnknownInfoVersion { rid: *rid, version });
            }

            let mut source_indexes = Vec::new();
            for attr in &info.attrs {
                if let Some(index) = numbered_marker(&attr.name, "sdk.source", "identity.hex") {
                    source_indexes.push((index, decode_marker(&attr.name, &attr.value)?));
                }
            }
            source_indexes.sort_by_key(|(index, _)| *index);
            source_indexes.dedup_by_key(|(index, _)| *index);
            if source_indexes.is_empty() {
                return Err(ShimPlanError::MissingMarker(format!(
                    "sdk.source.<n>.identity.hex for RID {}",
                    hex_encode(rid)
                )));
            }

            let aliases = parse_aliases(&info.attrs)?;
            for (source_index, source_identity) in &source_indexes {
                let mut methods = BTreeMap::new();
                for (method_name, signature) in &interface.methods {
                    let method_info = info.methods.get(method_name).ok_or_else(|| {
                        ShimPlanError::MissingMarker(format!("method {method_name}"))
                    })?;
                    let method_marker = indexed_name(*source_index, "method.hex");
                    let recorded_method = required_marker(&method_info.attrs, &method_marker)?;
                    if decode_marker(&method_marker, recorded_method)? != *method_name {
                        return Err(ShimPlanError::InvalidMarker(method_marker));
                    }

                    let args = signature
                        .params
                        .iter()
                        .enumerate()
                        .map(|(position, arg)| {
                            let entry = method_info.params.get(&position).ok_or_else(|| {
                                ShimPlanError::MissingMarker(format!(
                                    "method {method_name} parameter {position}"
                                ))
                            })?;
                            value_plan(
                                *rid,
                                method_name,
                                position,
                                "param",
                                *source_index,
                                arg,
                                entry,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let returns = signature
                        .rets
                        .iter()
                        .enumerate()
                        .map(|(position, arg)| {
                            let entry = method_info.returns.get(&position).ok_or_else(|| {
                                ShimPlanError::MissingMarker(format!(
                                    "method {method_name} return {position}"
                                ))
                            })?;
                            value_plan(
                                *rid,
                                method_name,
                                position,
                                "return",
                                *source_index,
                                arg,
                                entry,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    methods.insert(method_name.clone(), MethodPlan { args, returns });
                }
                plans.push(InterfacePlan {
                    source_identity: source_identity.clone(),
                    rid: *rid,
                    methods,
                    aliases: aliases.clone(),
                });
            }
        }
        plans.sort_by(|a, b| a.source_identity.cmp(&b.source_identity));
        Ok(plans)
    }
}

fn value_plan(
    rid: [u8; 32],
    method: &str,
    position: usize,
    section: &str,
    source_index: usize,
    arg: &Arg,
    entry: &ParamEntry,
) -> Result<ValuePlan, ShimPlanError> {
    let where_ = format!(
        "RID {} method {method} {section} {position}",
        hex_encode(rid)
    );
    let name_key = indexed_name(source_index, "name.hex");
    let type_key = indexed_name(source_index, "type.hex");
    let position_key = indexed_name(source_index, "position");
    let name = decode_marker(&name_key, required_marker(&entry.attrs, &name_key)?)?;
    let source_type = decode_marker(&type_key, required_marker(&entry.attrs, &type_key)?)?;
    let recorded_position = required_marker(&entry.attrs, &position_key)?;
    if recorded_position.parse::<usize>().ok() != Some(position) {
        return Err(ShimPlanError::InvalidMarker(position_key));
    }

    let scalar_key = indexed_name(source_index, "lowering.scalar");
    let reason_key = indexed_name(source_index, "lowering.any_reason");
    let any_key = indexed_name(source_index, "lowering.any");
    let target_key = indexed_name(source_index, "target_identity.hex");
    let target_rid_key = indexed_name(source_index, "target_rid.hex");
    let scalar = marker(&entry.attrs, &scalar_key);
    let any_reason = marker(&entry.attrs, &reason_key).map(str::to_owned);
    let any_marker = marker(&entry.attrs, &any_key);
    let any = any_marker == Some("true");
    if any_marker.is_some_and(|value| value != "true") {
        return Err(ShimPlanError::InvalidMarker(any_key));
    }
    if any_reason.is_some() && !any {
        return Err(ShimPlanError::InvalidMarker(reason_key));
    }
    if any
        && !matches!(
            any_reason.as_deref(),
            Some(
                "generic-parameter"
                    | "generic-interface"
                    | "aggregate"
                    | "recursive-interface-reference"
                    | "aggregate-alias"
            )
        )
    {
        return Err(ShimPlanError::InvalidMarker(reason_key));
    }
    if any && scalar.is_some() {
        return Err(ShimPlanError::InconsistentWireType(where_));
    }
    let target_identity = marker(&entry.attrs, &target_key)
        .map(|value| decode_marker(&target_key, value))
        .transpose()?;
    let target_rid = marker(&entry.attrs, &target_rid_key)
        .map(|value| decode_rid(&target_rid_key, value))
        .transpose()?;

    let kind = match (&arg.ty, scalar, any, any_reason.as_deref()) {
        (ArgTy::I32, Some("byte"), false, None) => ValueKind::Byte,
        (ArgTy::I32, Some("i16"), false, None) => ValueKind::I16,
        (ArgTy::I32, None, false, None) => ValueKind::I32,
        (ArgTy::I64, None, false, None) => ValueKind::I64,
        (ArgTy::F32, None, false, None) => ValueKind::F32,
        (ArgTy::F64, None, false, None) => ValueKind::F64,
        (
            ArgTy::Resource {
                ty: ResTy::None,
                nullable: false,
                take: false,
            },
            _,
            true,
            Some(_),
        ) => ValueKind::Any,
        (
            ArgTy::Resource {
                ty: ResTy::Of(target),
                nullable: false,
                take: false,
            },
            None,
            false,
            None,
        ) => {
            if target_rid != Some(*target) {
                return Err(ShimPlanError::InconsistentWireType(where_));
            }
            if target_identity.is_none() {
                return Err(ShimPlanError::MissingMarker(target_key));
            }
            ValueKind::Interface
        }
        (
            ArgTy::Resource {
                ty: ResTy::This,
                nullable: false,
                take: false,
            },
            None,
            false,
            None,
        ) => {
            if target_identity.is_none() {
                return Err(ShimPlanError::MissingMarker(target_key));
            }
            ValueKind::Interface
        }
        _ => return Err(ShimPlanError::InconsistentWireType(where_)),
    };

    Ok(ValuePlan {
        name,
        source_type,
        kind,
        any_reason,
        target_identity,
        target_rid,
        self_ref: matches!(
            arg.ty,
            ArgTy::Resource {
                ty: ResTy::This,
                ..
            }
        ),
    })
}

fn parse_aliases(attrs: &[pit_core::Attr]) -> Result<Vec<AliasPlan>, ShimPlanError> {
    let mut indexes = BTreeMap::<usize, BTreeMap<&str, &str>>::new();
    for attr in attrs {
        if let Some(rest) = attr.name.strip_prefix("sdk.source.alias.") {
            if let Some((index, field)) = rest.split_once('.') {
                if let Ok(index) = index.parse::<usize>() {
                    indexes.entry(index).or_default().insert(field, &attr.value);
                }
            }
        }
    }
    indexes
        .into_iter()
        .map(|(index, fields)| {
            let get = |field: &str| {
                fields.get(field).copied().ok_or_else(|| {
                    ShimPlanError::MissingMarker(format!("sdk.source.alias.{index}.{field}"))
                })
            };
            let decode = |field: &str| {
                let key = format!("sdk.source.alias.{index}.{field}");
                decode_marker(&key, get(field)?)
            };
            Ok(AliasPlan {
                source_identity: decode("identity.hex")?,
                target_identity: decode("target_identity.hex")?,
                target_rid: decode_rid(
                    &format!("sdk.source.alias.{index}.target_rid.hex"),
                    get("target_rid.hex")?,
                )?,
                source_type: decode("type.hex")?,
            })
        })
        .collect()
}

fn indexed_name(index: usize, suffix: &str) -> String {
    format!("sdk.source.{index}.{suffix}")
}

fn numbered_marker(name: &str, prefix: &str, suffix: &str) -> Option<usize> {
    let index = name
        .strip_prefix(prefix)?
        .strip_prefix('.')?
        .strip_suffix(&format!(".{suffix}"))?;
    index.parse().ok()
}

fn marker<'a>(attrs: &'a [pit_core::Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| attr.name == name)
        .map(|attr| attr.value.as_str())
}

fn required_marker<'a>(attrs: &'a [pit_core::Attr], name: &str) -> Result<&'a str, ShimPlanError> {
    marker(attrs, name).ok_or_else(|| ShimPlanError::MissingMarker(name.to_owned()))
}

fn decode_marker(name: &str, value: &str) -> Result<String, ShimPlanError> {
    let bytes = hex::decode(value).map_err(|_| ShimPlanError::InvalidMarker(name.to_owned()))?;
    String::from_utf8(bytes).map_err(|_| ShimPlanError::InvalidMarker(name.to_owned()))
}

fn decode_rid(name: &str, value: &str) -> Result<[u8; 32], ShimPlanError> {
    let bytes = hex::decode(value).map_err(|_| ShimPlanError::InvalidMarker(name.to_owned()))?;
    bytes
        .try_into()
        .map_err(|_| ShimPlanError::InvalidMarker(name.to_owned()))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SdkValue<T> {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Any(T),
    Interface(T),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PitValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Resource(u64),
}

/// Host hooks for converting opaque SDK values and interface objects to/from
/// runtime resource handles. The shim itself does not choose a transport.
pub trait ResourceResolver<T> {
    type Error;

    fn store_any(
        &mut self,
        owner_identity: &str,
        source_type: &str,
        target_identity: Option<&str>,
        value: T,
    ) -> Result<u64, Self::Error>;
    fn load_any(
        &mut self,
        owner_identity: &str,
        source_type: &str,
        target_identity: Option<&str>,
        handle: u64,
    ) -> Result<T, Self::Error>;
    fn store_interface(
        &mut self,
        target_identity: &str,
        target_rid: Option<&[u8; 32]>,
        value: T,
    ) -> Result<u64, Self::Error>;
    fn load_interface(
        &mut self,
        target_identity: &str,
        target_rid: Option<&[u8; 32]>,
        handle: u64,
    ) -> Result<T, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub enum ConversionError<E> {
    UnknownInterface(String),
    UnknownMethod(String),
    Arity {
        expected: usize,
        actual: usize,
    },
    WrongValue {
        position: usize,
        expected: &'static str,
    },
    OutOfRange {
        position: usize,
        scalar: &'static str,
    },
    MissingTargetIdentity {
        position: usize,
    },
    Resource(E),
}

impl<E: fmt::Display> fmt::Display for ConversionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownInterface(source) => write!(f, "unknown SDK interface {source}"),
            Self::UnknownMethod(method) => write!(f, "unknown SDK method {method}"),
            Self::Arity { expected, actual } => {
                write!(f, "expected {expected} values, got {actual}")
            }
            Self::WrongValue { position, expected } => {
                write!(f, "value {position} must be {expected}")
            }
            Self::OutOfRange { position, scalar } => {
                write!(f, "value {position} is outside {scalar} range")
            }
            Self::MissingTargetIdentity { position } => {
                write!(f, "value {position} has no target SDK identity")
            }
            Self::Resource(error) => write!(f, "resource conversion failed: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ConversionError<E> {}

pub fn sdk_values_to_pit<T, R: ResourceResolver<T>>(
    owner_identity: &str,
    slots: &[ValuePlan],
    values: Vec<SdkValue<T>>,
    resolver: &mut R,
) -> Result<Vec<PitValue>, ConversionError<R::Error>> {
    if slots.len() != values.len() {
        return Err(ConversionError::Arity {
            expected: slots.len(),
            actual: values.len(),
        });
    }
    slots
        .iter()
        .zip(values)
        .enumerate()
        .map(|(position, (slot, value))| {
            let wrong = |expected| ConversionError::WrongValue { position, expected };
            match (slot.kind, value) {
                (ValueKind::I32, SdkValue::I32(value)) => Ok(PitValue::I32(value)),
                (ValueKind::Byte, SdkValue::I32(value))
                    if (0..=u8::MAX as i32).contains(&value) =>
                {
                    Ok(PitValue::I32(value))
                }
                (ValueKind::Byte, SdkValue::I32(_)) => Err(ConversionError::OutOfRange {
                    position,
                    scalar: "byte",
                }),
                (ValueKind::I16, SdkValue::I32(value))
                    if (i16::MIN as i32..=i16::MAX as i32).contains(&value) =>
                {
                    Ok(PitValue::I32(value))
                }
                (ValueKind::I16, SdkValue::I32(_)) => Err(ConversionError::OutOfRange {
                    position,
                    scalar: "i16",
                }),
                (ValueKind::I64, SdkValue::I64(value)) => Ok(PitValue::I64(value)),
                (ValueKind::F32, SdkValue::F32(value)) => Ok(PitValue::F32(value)),
                (ValueKind::F64, SdkValue::F64(value)) => Ok(PitValue::F64(value)),
                (ValueKind::Any, SdkValue::Any(value)) => resolver
                    .store_any(
                        owner_identity,
                        &slot.source_type,
                        slot.target_identity.as_deref(),
                        value,
                    )
                    .map(PitValue::Resource)
                    .map_err(ConversionError::Resource),
                (ValueKind::Interface, SdkValue::Interface(value)) => {
                    let target = slot
                        .target_identity
                        .as_deref()
                        .ok_or(ConversionError::MissingTargetIdentity { position })?;
                    resolver
                        .store_interface(target, slot.target_rid.as_ref(), value)
                        .map(PitValue::Resource)
                        .map_err(ConversionError::Resource)
                }
                (ValueKind::I32, _) => Err(wrong("i32")),
                (ValueKind::Byte, _) => Err(wrong("byte/i32")),
                (ValueKind::I16, _) => Err(wrong("i16/i32")),
                (ValueKind::I64, _) => Err(wrong("i64")),
                (ValueKind::F32, _) => Err(wrong("f32")),
                (ValueKind::F64, _) => Err(wrong("f64")),
                (ValueKind::Any, _) => Err(wrong("opaque SDK value")),
                (ValueKind::Interface, _) => Err(wrong("SDK interface value")),
            }
        })
        .collect()
}

pub fn pit_values_to_sdk<T, R: ResourceResolver<T>>(
    owner_identity: &str,
    slots: &[ValuePlan],
    values: Vec<PitValue>,
    resolver: &mut R,
) -> Result<Vec<SdkValue<T>>, ConversionError<R::Error>> {
    if slots.len() != values.len() {
        return Err(ConversionError::Arity {
            expected: slots.len(),
            actual: values.len(),
        });
    }
    slots
        .iter()
        .zip(values)
        .enumerate()
        .map(|(position, (slot, value))| {
            let wrong = |expected| ConversionError::WrongValue { position, expected };
            match (slot.kind, value) {
                (ValueKind::I32, PitValue::I32(value)) => Ok(SdkValue::I32(value)),
                (ValueKind::Byte, PitValue::I32(value))
                    if (0..=u8::MAX as i32).contains(&value) =>
                {
                    Ok(SdkValue::I32(value))
                }
                (ValueKind::Byte, PitValue::I32(_)) => Err(ConversionError::OutOfRange {
                    position,
                    scalar: "byte",
                }),
                (ValueKind::I16, PitValue::I32(value))
                    if (i16::MIN as i32..=i16::MAX as i32).contains(&value) =>
                {
                    Ok(SdkValue::I32(value))
                }
                (ValueKind::I16, PitValue::I32(_)) => Err(ConversionError::OutOfRange {
                    position,
                    scalar: "i16",
                }),
                (ValueKind::I64, PitValue::I64(value)) => Ok(SdkValue::I64(value)),
                (ValueKind::F32, PitValue::F32(value)) => Ok(SdkValue::F32(value)),
                (ValueKind::F64, PitValue::F64(value)) => Ok(SdkValue::F64(value)),
                (ValueKind::Any, PitValue::Resource(handle)) => resolver
                    .load_any(
                        owner_identity,
                        &slot.source_type,
                        slot.target_identity.as_deref(),
                        handle,
                    )
                    .map(SdkValue::Any)
                    .map_err(ConversionError::Resource),
                (ValueKind::Interface, PitValue::Resource(handle)) => {
                    let target = slot
                        .target_identity
                        .as_deref()
                        .ok_or(ConversionError::MissingTargetIdentity { position })?;
                    resolver
                        .load_interface(target, slot.target_rid.as_ref(), handle)
                        .map(SdkValue::Interface)
                        .map_err(ConversionError::Resource)
                }
                (ValueKind::I32, _) => Err(wrong("PIT I32")),
                (ValueKind::Byte, _) => Err(wrong("PIT I32(byte)")),
                (ValueKind::I16, _) => Err(wrong("PIT I32(i16)")),
                (ValueKind::I64, _) => Err(wrong("PIT I64")),
                (ValueKind::F32, _) => Err(wrong("PIT F32")),
                (ValueKind::F64, _) => Err(wrong("PIT F64")),
                (ValueKind::Any, _) => Err(wrong("PIT resource")),
                (ValueKind::Interface, _) => Err(wrong("PIT resource")),
            }
        })
        .collect()
}

pub fn emit_rust_shim(lowered: &LoweredSdk) -> Result<String, ShimPlanError> {
    let plans = lowered.shim_plans()?;
    let mut output = String::from(
        "// Generated by pit-sdk-bridge. Add pit-sdk-bridge with feature unstable-sdk.\n\
         use pit_sdk_bridge::{AliasPlan, ConversionError, InterfacePlan, MethodPlan, PitValue,\n\
         ResourceResolver, SdkValue, ValueKind, ValuePlan, pit_values_to_sdk, sdk_values_to_pit};\n\n",
    );
    output.push_str("pub fn plans() -> Vec<InterfacePlan> {\n    vec![\n");
    for interface in plans {
        writeln!(
            output,
            "        InterfacePlan {{ source_identity: {:?}.into(), rid: {:?}, methods: [",
            interface.source_identity, interface.rid
        )
        .unwrap();
        for (method_name, method) in interface.methods {
            writeln!(
                output,
                "            ({method_name:?}.into(), MethodPlan {{ args: {}, returns: {} }}),",
                rust_values(&method.args),
                rust_values(&method.returns)
            )
            .unwrap();
        }
        output.push_str("        ].into_iter().collect(), aliases: vec![");
        for alias in interface.aliases {
            write!(output, "AliasPlan {{ source_identity: {:?}.into(), target_identity: {:?}.into(), target_rid: {:?}, source_type: {:?}.into() }},", alias.source_identity, alias.target_identity, alias.target_rid, alias.source_type).unwrap();
        }
        output.push_str("] },\n");
    }
    output.push_str("    ]\n}\n\
fn method_plan(source: &str, method: &str) -> Result<MethodPlan, ConversionError<std::convert::Infallible>> {\n\
    let interface = plans().into_iter().find(|item| item.source_identity == source).ok_or_else(|| ConversionError::UnknownInterface(source.into()))?;\n\
    interface.methods.get(method).cloned().ok_or_else(|| ConversionError::UnknownMethod(method.into()))\n}\n\
fn convert_error<E>(error: ConversionError<std::convert::Infallible>) -> ConversionError<E> {\n\
    match error { ConversionError::UnknownInterface(s) => ConversionError::UnknownInterface(s), ConversionError::UnknownMethod(s) => ConversionError::UnknownMethod(s), _ => unreachable!() }\n}\n\
pub fn sdk_args_to_pit<T, R: ResourceResolver<T>>(source: &str, method: &str, values: Vec<SdkValue<T>>, resolver: &mut R) -> Result<Vec<PitValue>, ConversionError<R::Error>> {\n\
    let plan = method_plan(source, method).map_err(convert_error)?; sdk_values_to_pit(source, &plan.args, values, resolver)\n}\n\
pub fn pit_args_to_sdk<T, R: ResourceResolver<T>>(source: &str, method: &str, values: Vec<PitValue>, resolver: &mut R) -> Result<Vec<SdkValue<T>>, ConversionError<R::Error>> {\n\
    let plan = method_plan(source, method).map_err(convert_error)?; pit_values_to_sdk(source, &plan.args, values, resolver)\n}\n\
pub fn sdk_returns_to_pit<T, R: ResourceResolver<T>>(source: &str, method: &str, values: Vec<SdkValue<T>>, resolver: &mut R) -> Result<Vec<PitValue>, ConversionError<R::Error>> {\n\
    let plan = method_plan(source, method).map_err(convert_error)?; sdk_values_to_pit(source, &plan.returns, values, resolver)\n}\n\
pub fn pit_returns_to_sdk<T, R: ResourceResolver<T>>(source: &str, method: &str, values: Vec<PitValue>, resolver: &mut R) -> Result<Vec<SdkValue<T>>, ConversionError<R::Error>> {\n\
    let plan = method_plan(source, method).map_err(convert_error)?; pit_values_to_sdk(source, &plan.returns, values, resolver)\n}\n");
    Ok(output)
}

fn rust_values(values: &[ValuePlan]) -> String {
    let mut output = String::from("vec![");
    for value in values {
        let kind = match value.kind {
            ValueKind::I32 => "I32",
            ValueKind::I64 => "I64",
            ValueKind::F32 => "F32",
            ValueKind::F64 => "F64",
            ValueKind::Byte => "Byte",
            ValueKind::I16 => "I16",
            ValueKind::Any => "Any",
            ValueKind::Interface => "Interface",
        };
        write!(output, "ValuePlan {{ name: {:?}.into(), source_type: {:?}.into(), kind: ValueKind::{kind}, any_reason: {}, target_identity: {}, target_rid: {}, self_ref: {} }},", value.name, value.source_type, rust_option_str(value.any_reason.as_deref()), rust_option_str(value.target_identity.as_deref()), rust_option_rid(value.target_rid.as_ref()), value.self_ref).unwrap();
    }
    output.push(']');
    output
}

fn rust_option_str(value: Option<&str>) -> String {
    value.map_or_else(
        || "None".to_owned(),
        |value| format!("Some({value:?}.into())"),
    )
}

fn rust_option_rid(value: Option<&[u8; 32]>) -> String {
    value.map_or_else(|| "None".to_owned(), |rid| format!("Some({rid:?})"))
}

pub fn emit_typescript_shim(lowered: &LoweredSdk) -> Result<String, ShimPlanError> {
    let plans = lowered.shim_plans()?;
    let json = ts_plans_json(&plans);
    let output = format!(
        r#"// Generated by pit-sdk-bridge. Runtime hooks intentionally leave transport to the host.
export type ValueKind = "i32" | "i64" | "f32" | "f64" | "byte" | "i16" | "any" | "interface";
export interface ValuePlan {{ name: string; sourceType: string; kind: ValueKind; anyReason: string | null; targetIdentity: string | null; targetRid: string | null; selfRef: boolean }}
export interface MethodPlan {{ args: readonly ValuePlan[]; returns: readonly ValuePlan[] }}
export interface AliasPlan {{ sourceIdentity: string; targetIdentity: string; targetRid: string; sourceType: string }}
export interface InterfacePlan {{ sourceIdentity: string; rid: string; methods: Readonly<Record<string, MethodPlan>>; aliases: readonly AliasPlan[] }}
export const plans: readonly InterfacePlan[] = {json};
export type SdkValue<T> = {{ kind: "i32" | "byte" | "i16"; value: number }} | {{ kind: "i64"; value: bigint }} | {{ kind: "f32" | "f64"; value: number }} | {{ kind: "any" | "interface"; value: T }};
export type PitValue = {{ kind: "i32"; value: number }} | {{ kind: "i64"; value: bigint }} | {{ kind: "f32" | "f64"; value: number }} | {{ kind: "resource"; handle: number }};
export interface ResourceResolver<T> {{
  storeAny(owner: string, sourceType: string, targetIdentity: string | null, value: T): number;
  loadAny(owner: string, sourceType: string, targetIdentity: string | null, handle: number): T;
  storeInterface(targetIdentity: string, targetRid: string | null, value: T): number;
  loadInterface(targetIdentity: string, targetRid: string | null, handle: number): T;
}}
export class ShimError extends Error {{}}
function checkedPlan(source: string, method: string): MethodPlan {{
  const iface = plans.find((entry) => entry.sourceIdentity === source);
  if (!iface) throw new ShimError(`unknown SDK interface ${{source}}`);
  const value = iface.methods[method];
  if (!value) throw new ShimError(`unknown SDK method ${{method}}`);
  return value;
}}
function toPit<T>(owner: string, slot: ValuePlan, value: SdkValue<T>, position: number, resolver: ResourceResolver<T>): PitValue {{
  const wrong = (): never => {{ throw new ShimError(`wrong SDK value at ${{position}} (${{slot.kind}})`); }};
  switch (slot.kind) {{
    case "i32": if (value.kind !== "i32" || !Number.isInteger(value.value) || value.value < -2147483648 || value.value > 2147483647) return wrong(); return {{ kind: "i32", value: value.value }};
    case "byte": if ((value.kind !== "byte" && value.kind !== "i32") || !Number.isInteger(value.value)) return wrong(); if (value.value < 0 || value.value > 255) throw new ShimError(`byte out of range at ${{position}}`); return {{ kind: "i32", value: value.value }};
    case "i16": if ((value.kind !== "i16" && value.kind !== "i32") || !Number.isInteger(value.value)) return wrong(); if (value.value < -32768 || value.value > 32767) throw new ShimError(`i16 out of range at ${{position}}`); return {{ kind: "i32", value: value.value }};
    case "i64": if (value.kind !== "i64") return wrong(); return {{ kind: "i64", value: value.value }};
    case "f32": if (value.kind !== "f32") return wrong(); return {{ kind: "f32", value: value.value }};
    case "f64": if (value.kind !== "f64") return wrong(); return {{ kind: "f64", value: value.value }};
    case "any": if (value.kind !== "any") return wrong(); return {{ kind: "resource", handle: resolver.storeAny(owner, slot.sourceType, slot.targetIdentity, value.value) }};
    case "interface": if (value.kind !== "interface" || slot.targetIdentity === null) return wrong(); return {{ kind: "resource", handle: resolver.storeInterface(slot.targetIdentity, slot.targetRid, value.value) }};
  }}
}}
function fromPit<T>(owner: string, slot: ValuePlan, value: PitValue, position: number, resolver: ResourceResolver<T>): SdkValue<T> {{
  const wrong = (): never => {{ throw new ShimError(`wrong PIT value at ${{position}} (${{slot.kind}})`); }};
  switch (slot.kind) {{
    case "i32": if (value.kind !== "i32") return wrong(); return {{ kind: "i32", value: value.value }};
    case "byte": if (value.kind !== "i32") return wrong(); if (!Number.isInteger(value.value) || value.value < 0 || value.value > 255) throw new ShimError(`byte out of range at ${{position}}`); return {{ kind: "byte", value: value.value }};
    case "i16": if (value.kind !== "i32") return wrong(); if (!Number.isInteger(value.value) || value.value < -32768 || value.value > 32767) throw new ShimError(`i16 out of range at ${{position}}`); return {{ kind: "i16", value: value.value }};
    case "i64": if (value.kind !== "i64") return wrong(); return {{ kind: "i64", value: value.value }};
    case "f32": if (value.kind !== "f32") return wrong(); return {{ kind: "f32", value: value.value }};
    case "f64": if (value.kind !== "f64") return wrong(); return {{ kind: "f64", value: value.value }};
    case "any": if (value.kind !== "resource") return wrong(); return {{ kind: "any", value: resolver.loadAny(owner, slot.sourceType, slot.targetIdentity, value.handle) }};
    case "interface": if (value.kind !== "resource" || slot.targetIdentity === null) return wrong(); return {{ kind: "interface", value: resolver.loadInterface(slot.targetIdentity, slot.targetRid, value.handle) }};
  }}
}}
function convertToPit<T>(owner: string, slots: readonly ValuePlan[], values: readonly SdkValue<T>[], resolver: ResourceResolver<T>): PitValue[] {{
  if (slots.length !== values.length) throw new ShimError(`arity mismatch: expected ${{slots.length}}, got ${{values.length}}`);
  return slots.map((slot, index) => toPit(owner, slot, values[index]!, index, resolver));
}}
function convertFromPit<T>(owner: string, slots: readonly ValuePlan[], values: readonly PitValue[], resolver: ResourceResolver<T>): SdkValue<T>[] {{
  if (slots.length !== values.length) throw new ShimError(`arity mismatch: expected ${{slots.length}}, got ${{values.length}}`);
  return slots.map((slot, index) => fromPit(owner, slot, values[index]!, index, resolver));
}}
export function sdkArgsToPit<T>(source: string, method: string, values: readonly SdkValue<NoInfer<T>>[], resolver: ResourceResolver<T>): PitValue[] {{ return convertToPit(source, checkedPlan(source, method).args, values, resolver); }}
export function pitArgsToSdk<T>(source: string, method: string, values: readonly PitValue[], resolver: ResourceResolver<T>): SdkValue<T>[] {{ return convertFromPit(source, checkedPlan(source, method).args, values, resolver); }}
export function sdkReturnsToPit<T>(source: string, method: string, values: readonly SdkValue<NoInfer<T>>[], resolver: ResourceResolver<T>): PitValue[] {{ return convertToPit(source, checkedPlan(source, method).returns, values, resolver); }}
export function pitReturnsToSdk<T>(source: string, method: string, values: readonly PitValue[], resolver: ResourceResolver<T>): SdkValue<T>[] {{ return convertFromPit(source, checkedPlan(source, method).returns, values, resolver); }}
"#
    );
    Ok(output)
}

fn ts_plans_json(plans: &[InterfacePlan]) -> String {
    let mut out = String::from("[");
    for (interface_index, interface) in plans.iter().enumerate() {
        if interface_index > 0 {
            out.push(',');
        }
        write!(
            out,
            "{{\"sourceIdentity\":{},\"rid\":{},\"methods\":{{",
            json_string(&interface.source_identity),
            json_string(&hex_encode(interface.rid))
        )
        .unwrap();
        for (method_index, (name, method)) in interface.methods.iter().enumerate() {
            if method_index > 0 {
                out.push(',');
            }
            write!(
                out,
                "{}:{{\"args\":{},\"returns\":{}}}",
                json_string(name),
                ts_values_json(&method.args),
                ts_values_json(&method.returns)
            )
            .unwrap();
        }
        out.push_str("},\"aliases\":[");
        for (alias_index, alias) in interface.aliases.iter().enumerate() {
            if alias_index > 0 {
                out.push(',');
            }
            write!(out, "{{\"sourceIdentity\":{},\"targetIdentity\":{},\"targetRid\":{},\"sourceType\":{}}}", json_string(&alias.source_identity), json_string(&alias.target_identity), json_string(&hex_encode(alias.target_rid)), json_string(&alias.source_type)).unwrap();
        }
        out.push_str("]}");
    }
    out.push(']');
    out
}

fn ts_values_json(values: &[ValuePlan]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let kind = match value.kind {
            ValueKind::I32 => "i32",
            ValueKind::I64 => "i64",
            ValueKind::F32 => "f32",
            ValueKind::F64 => "f64",
            ValueKind::Byte => "byte",
            ValueKind::I16 => "i16",
            ValueKind::Any => "any",
            ValueKind::Interface => "interface",
        };
        write!(out, "{{\"name\":{},\"sourceType\":{},\"kind\":{},\"anyReason\":{},\"targetIdentity\":{},\"targetRid\":{},\"selfRef\":{}}}", json_string(&value.name), json_string(&value.source_type), json_string(kind), json_optional(value.any_reason.as_deref()), json_optional(value.target_identity.as_deref()), json_optional(value.target_rid.as_ref().map(hex_encode).as_deref()), value.self_ref).unwrap();
    }
    out.push(']');
    out
}

fn json_optional(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1f}' => write!(out, "\\u{:04x}", ch as u32).unwrap(),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use portal_solutions_sdk::{
        Arity, Sdk, SdkInterface, SdkItem, SdkItemContents, SdkMethod, SdkParam, SdkTy,
    };

    fn param(ty: SdkTy) -> SdkParam {
        SdkParam {
            ty,
            attr: Vec::new(),
        }
    }

    fn lowered_fixture() -> LoweredSdk {
        let method = SdkMethod {
            arity: Arity::default(),
            args: BTreeMap::from([
                (
                    "blob".to_owned(),
                    param(SdkTy::Array {
                        item: Box::new(param(SdkTy::I32)),
                    }),
                ),
                ("octet".to_owned(), param(SdkTy::Byte)),
            ]),
            rets: BTreeMap::from([("small".to_owned(), param(SdkTy::I16))]),
            attr: Vec::new(),
        };
        let api = SdkInterface {
            methods: BTreeMap::from([("echo".to_owned(), (Arity::default(), method))]),
            attr: Vec::new(),
        };
        let docs = BTreeMap::from([(
            "fixture.sdk".to_owned(),
            Sdk {
                interfaces: BTreeMap::from([(
                    "Api".to_owned(),
                    SdkItem {
                        generics: Arity::default(),
                        contents: SdkItemContents::Type(param(SdkTy::Interface {
                            implementation: api,
                        })),
                    },
                )]),
            },
        )]);
        crate::lower_sdk("fixture.sdk", &docs).unwrap()
    }

    #[derive(Default)]
    struct TestResolver {
        any_values: BTreeMap<u64, String>,
        interface_values: BTreeMap<u64, String>,
        interface_target: Option<(String, Option<[u8; 32]>)>,
    }

    impl ResourceResolver<String> for TestResolver {
        type Error = &'static str;

        fn store_any(
            &mut self,
            _: &str,
            _: &str,
            _: Option<&str>,
            value: String,
        ) -> Result<u64, Self::Error> {
            let handle = self.any_values.len() as u64 + 1;
            self.any_values.insert(handle, value);
            Ok(handle)
        }

        fn load_any(
            &mut self,
            _: &str,
            _: &str,
            _: Option<&str>,
            handle: u64,
        ) -> Result<String, Self::Error> {
            self.any_values
                .get(&handle)
                .cloned()
                .ok_or("unknown any handle")
        }

        fn store_interface(
            &mut self,
            target: &str,
            target_rid: Option<&[u8; 32]>,
            value: String,
        ) -> Result<u64, Self::Error> {
            let handle = self.interface_values.len() as u64 + 1;
            self.interface_target = Some((target.to_owned(), target_rid.copied()));
            self.interface_values.insert(handle, value);
            Ok(handle)
        }

        fn load_interface(
            &mut self,
            target: &str,
            target_rid: Option<&[u8; 32]>,
            handle: u64,
        ) -> Result<String, Self::Error> {
            self.interface_target = Some((target.to_owned(), target_rid.copied()));
            self.interface_values
                .get(&handle)
                .cloned()
                .ok_or("unknown interface handle")
        }
    }

    #[test]
    fn plans_are_reconstructed_from_source_indexed_info() {
        let lowered = lowered_fixture();
        let plans = lowered.shim_plans().unwrap();
        assert_eq!(plans.len(), 1);
        let method = &plans[0].methods["echo"];
        assert_eq!(
            method.args.iter().map(|arg| arg.kind).collect::<Vec<_>>(),
            [ValueKind::Any, ValueKind::Byte]
        );
        assert_eq!(method.returns[0].kind, ValueKind::I16);
        assert_eq!(method.args[0].any_reason.as_deref(), Some("aggregate"));
    }

    #[test]
    fn paired_runtime_converters_round_trip_values_and_check_narrow_ranges() {
        let plans = lowered_fixture().shim_plans().unwrap();
        let plan = &plans[0];
        let method = &plan.methods["echo"];
        let mut resolver = TestResolver::default();
        let pit_args = sdk_values_to_pit(
            &plan.source_identity,
            &method.args,
            vec![SdkValue::Any("opaque array".to_owned()), SdkValue::I32(255)],
            &mut resolver,
        )
        .unwrap();
        assert_eq!(pit_args, [PitValue::Resource(1), PitValue::I32(255)]);
        let sdk_args =
            pit_values_to_sdk(&plan.source_identity, &method.args, pit_args, &mut resolver)
                .unwrap();
        assert_eq!(
            sdk_args,
            [SdkValue::Any("opaque array".to_owned()), SdkValue::I32(255)]
        );

        let error = sdk_values_to_pit(
            &plan.source_identity,
            &method.args,
            vec![SdkValue::Any("payload".to_owned()), SdkValue::I32(256)],
            &mut resolver,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ConversionError::OutOfRange {
                position: 1,
                scalar: "byte"
            }
        );
        let error = pit_values_to_sdk(
            &plan.source_identity,
            &method.returns,
            vec![PitValue::I32(32768)],
            &mut resolver,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ConversionError::OutOfRange {
                position: 0,
                scalar: "i16"
            }
        );
    }

    #[test]
    fn interface_resources_preserve_target_identity_and_rid() {
        let rid = [7; 32];
        let slot = ValuePlan {
            name: "child".to_owned(),
            source_type: "Child".to_owned(),
            kind: ValueKind::Interface,
            any_reason: None,
            target_identity: Some("api::Child".to_owned()),
            target_rid: Some(rid),
            self_ref: false,
        };
        let mut resolver = TestResolver::default();
        let pit = sdk_values_to_pit(
            "api::Parent",
            std::slice::from_ref(&slot),
            vec![SdkValue::Interface("child-value".to_owned())],
            &mut resolver,
        )
        .unwrap();
        assert_eq!(pit, [PitValue::Resource(1)]);
        assert_eq!(
            resolver.interface_target,
            Some(("api::Child".to_owned(), Some(rid)))
        );
        let sdk = pit_values_to_sdk("api::Parent", &[slot], pit, &mut resolver).unwrap();
        assert_eq!(sdk, [SdkValue::Interface("child-value".to_owned())]);
    }

    #[test]
    fn unknown_erasure_reasons_are_rejected_by_shim_generation() {
        let mut lowered = lowered_fixture();
        let method = lowered
            .info
            .interfaces
            .values_mut()
            .next()
            .unwrap()
            .methods
            .get_mut("echo")
            .unwrap();
        method
            .params
            .values_mut()
            .flat_map(|entry| entry.attrs.iter_mut())
            .find(|attr| {
                attr.name.starts_with("sdk.source.") && attr.name.ends_with("lowering.any_reason")
            })
            .unwrap()
            .value = "unknown-erasure".to_owned();
        let result = lowered.shim_plans();
        assert!(
            matches!(result, Err(ShimPlanError::InvalidMarker(_))),
            "unexpected extraction result: {result:?}"
        );
    }

    #[test]
    fn unknown_info_versions_are_rejected_by_shim_generation() {
        let mut lowered = lowered_fixture();
        lowered
            .info
            .interfaces
            .values_mut()
            .next()
            .unwrap()
            .attrs
            .iter_mut()
            .find(|attr| attr.name == "sdk.lowering.version")
            .unwrap()
            .value = "2".to_owned();
        assert!(matches!(
            lowered.shim_plans(),
            Err(ShimPlanError::UnknownInfoVersion { .. })
        ));
    }

    #[test]
    fn both_language_emitters_include_two_way_dispatch() {
        let lowered = lowered_fixture();
        let rust = emit_rust_shim(&lowered).unwrap();
        assert!(rust.contains("sdk_args_to_pit"));
        assert!(rust.contains("pit_returns_to_sdk"));
        let ts = emit_typescript_shim(&lowered).unwrap();
        assert!(ts.contains("sdkArgsToPit"));
        assert!(ts.contains("pitReturnsToSdk"));
        assert!(ts.contains("byte out of range"));
    }

    #[test]
    fn json_string_escapes_metadata() {
        assert_eq!(
            json_string("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
