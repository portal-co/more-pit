#![cfg(feature = "unstable-sdk")]

//! SDK schema lowering to the minimal PIT interface model.
//!
//! Only SDK interfaces become PIT interfaces. Aliases and aggregate/generic
//! shapes are retained as source metadata in `Info`; this keeps recursive SDK
//! graphs from multiplying PIT identities or creating synthetic schemas.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hex::encode as hex_encode;
use pit_core::{
    Arg, ArgTy, Interface, ResTy, Sig,
    info::{Info, InfoEntry, MethEntry, ParamEntry},
};
#[cfg(test)]
use portal_solutions_sdk::SdkMethod;
use portal_solutions_sdk::{Arity, Sdk, SdkInterface, SdkItemContents, SdkParam, SdkTy};

mod shim;
pub use shim::*;

const INFO_VERSION: &str = "sdk.lowering.version";
const DOC_ATTRS: &[&str] = &[
    "name",
    "doc",
    "brief",
    "deprecated",
    "since",
    "author",
    "license",
    "see",
    "category",
    "tags",
    "example",
];

/// Output of deterministic schema lowering.
#[derive(Clone, Debug, Default)]
pub struct LoweredSdk {
    /// Interfaces keyed by their canonical PIT RID.
    pub interfaces: BTreeMap<[u8; 32], Interface>,
    /// Documentation, source markers, and erased SDK type descriptions keyed by PIT RID.
    pub info: Info,
    /// Stable SDK source identity to PIT RID mapping for shim generation.
    pub source_rids: BTreeMap<String, [u8; 32]>,
}

/// A lowering failure. Unsupported declarations are rejected instead of
/// becoming an unmarked `R` or a partially emitted graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoweringError {
    MissingDocument(String),
    MissingDeclaration { document: String, name: String },
    MethodUsedAsType { document: String, name: String },
    StandaloneMethod { document: String, name: String },
    AliasCycle(String),
    DuplicateGenericParameter { location: String, name: String },
    IncompatibleGenericParameter { location: String, name: String },
    InvalidGenericUse(String),
    MissingInterface(String),
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDocument(path) => write!(f, "SDK document not loaded: {path}"),
            Self::MissingDeclaration { document, name } => {
                write!(f, "SDK declaration {name} not found in {document}")
            }
            Self::MethodUsedAsType { document, name } => {
                write!(
                    f,
                    "SDK method {name} in {document} cannot be used as a type"
                )
            }
            Self::StandaloneMethod { document, name } => write!(
                f,
                "top-level SDK method {name} in {document} is not an SDK interface; nest it in SdkInterface"
            ),
            Self::AliasCycle(path) => write!(f, "SDK type-alias cycle at {path}"),
            Self::DuplicateGenericParameter { location, name } => {
                write!(f, "duplicate SDK generic parameter {name} at {location}")
            }
            Self::IncompatibleGenericParameter { location, name } => {
                write!(f, "incompatible SDK generic parameter {name} at {location}")
            }
            Self::InvalidGenericUse(path) => write!(f, "invalid SDK generic use at {path}"),
            Self::MissingInterface(path) => {
                write!(f, "SDK interface source identity not found: {path}")
            }
        }
    }
}

impl std::error::Error for LoweringError {}

#[derive(Clone)]
struct InterfaceDef {
    source_id: String,
    document: String,
    interface: SdkInterface,
    arity: Arity,
}

#[derive(Clone)]
struct AliasDef {
    document: String,
    name: String,
    param: SdkParam,
    arity: Arity,
}

#[derive(Clone)]
enum ResolvedType {
    Interface {
        source_id: String,
        generic: bool,
    },
    NonInterface {
        param: SdkParam,
        document: String,
        generic: bool,
    },
}

#[derive(Clone, Debug, Default)]
struct TypeMetadata {
    any_reason: Option<&'static str>,
    scalar: Option<&'static str>,
    target_source: Option<String>,
    target_rid: Option<[u8; 32]>,
    expanded_type: Option<String>,
}

/// Lower all SDK interfaces reachable from the supplied source catalog.
///
/// `documents` is keyed by the exact source path used by `SdkTy::Path`; include
/// the root document and every referenced document. Interfaces in those
/// documents are emitted once, deduplicated by PIT RID.
pub fn lower_sdk(
    root_path: &str,
    documents: &BTreeMap<String, Sdk>,
) -> Result<LoweredSdk, LoweringError> {
    if !documents.contains_key(root_path) {
        return Err(LoweringError::MissingDocument(root_path.to_owned()));
    }

    let mut lowerer = Lowerer::new(documents.clone());
    lowerer.collect_definitions()?;
    lowerer.analyze_cycles()?;

    let source_ids: Vec<String> = lowerer.definitions.keys().cloned().collect();
    lowerer.source_indices = source_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    for source_id in source_ids {
        lowerer.lower_interface(&source_id)?;
    }
    lowerer.add_alias_rids()?;
    Ok(lowerer.finish())
}

struct Lowerer {
    documents: BTreeMap<String, Sdk>,
    definitions: BTreeMap<String, InterfaceDef>,
    aliases: BTreeMap<String, AliasDef>,
    edges: BTreeMap<String, BTreeSet<String>>,
    recursive_edges: BTreeSet<(String, String)>,
    source_indices: BTreeMap<String, usize>,
    interfaces: BTreeMap<[u8; 32], Interface>,
    source_rids: BTreeMap<String, [u8; 32]>,
    info_parts: BTreeMap<[u8; 32], Vec<(usize, InfoEntry)>>,
}

impl Lowerer {
    fn new(documents: BTreeMap<String, Sdk>) -> Self {
        Self {
            documents,
            definitions: BTreeMap::new(),
            aliases: BTreeMap::new(),
            edges: BTreeMap::new(),
            recursive_edges: BTreeSet::new(),
            source_indices: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            source_rids: BTreeMap::new(),
            info_parts: BTreeMap::new(),
        }
    }

    fn collect_definitions(&mut self) -> Result<(), LoweringError> {
        let document_paths: Vec<String> = self.documents.keys().cloned().collect();
        for document in document_paths {
            let sdk = self.documents.get(&document).expect("catalog key exists");
            let items: Vec<(String, portal_solutions_sdk::SdkItem)> = sdk
                .interfaces
                .iter()
                .map(|(name, item)| (name.clone(), item.clone()))
                .collect();
            for (name, item) in items {
                let source_id = source_id(&document, &name);
                match item.contents {
                    SdkItemContents::Method(_) => {
                        return Err(LoweringError::StandaloneMethod { document, name });
                    }
                    SdkItemContents::Type(param) => {
                        self.aliases.insert(
                            source_id.clone(),
                            AliasDef {
                                document: document.clone(),
                                name: name.clone(),
                                param: param.clone(),
                                arity: item.generics.clone(),
                            },
                        );
                        self.collect_param(&param, &document, &source_id, item.generics)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_param(
        &mut self,
        param: &SdkParam,
        document: &str,
        location: &str,
        outer_arity: Arity,
    ) -> Result<(), LoweringError> {
        match &param.ty {
            SdkTy::Interface { implementation } => {
                self.definitions.insert(
                    location.to_owned(),
                    InterfaceDef {
                        source_id: location.to_owned(),
                        document: document.to_owned(),
                        interface: implementation.clone(),
                        arity: outer_arity,
                    },
                );
                self.collect_interface_children(implementation, document, location)?;
            }
            SdkTy::Generic { args, .. } | SdkTy::Path { args, .. } => {
                for (name, child) in args {
                    self.collect_param(
                        child,
                        document,
                        &format!("{location}/generic/{name}"),
                        Arity::default(),
                    )?;
                }
            }
            SdkTy::Array { item } => {
                self.collect_param(
                    item,
                    document,
                    &format!("{location}/item"),
                    Arity::default(),
                )?;
            }
            SdkTy::Struct { items } => {
                for (name, child) in items {
                    self.collect_param(
                        child,
                        document,
                        &format!("{location}/field/{name}"),
                        Arity::default(),
                    )?;
                }
            }
            SdkTy::Variant { choices } => {
                for (index, child) in choices.iter().enumerate() {
                    self.collect_param(
                        child,
                        document,
                        &format!("{location}/choice/{index}"),
                        Arity::default(),
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn collect_interface_children(
        &mut self,
        interface: &SdkInterface,
        document: &str,
        owner: &str,
    ) -> Result<(), LoweringError> {
        for (method_name, (_, method)) in &interface.methods {
            for (name, param) in &method.args {
                self.collect_param(
                    param,
                    document,
                    &format!("{owner}/method/{method_name}/param/{name}"),
                    Arity::default(),
                )?;
            }
            for (name, param) in &method.rets {
                self.collect_param(
                    param,
                    document,
                    &format!("{owner}/method/{method_name}/return/{name}"),
                    Arity::default(),
                )?;
            }
        }
        Ok(())
    }

    fn analyze_cycles(&mut self) -> Result<(), LoweringError> {
        let defs: Vec<InterfaceDef> = self.definitions.values().cloned().collect();
        for def in &defs {
            let mut edges = BTreeSet::new();
            for (method_name, (_, method)) in &def.interface.methods {
                for (name, param) in &method.args {
                    self.collect_direct_edges(
                        param,
                        &def.document,
                        &def.source_id,
                        &format!("{}/method/{method_name}/param/{name}", def.source_id),
                        &mut edges,
                    )?;
                }
                for (name, param) in &method.rets {
                    self.collect_direct_edges(
                        param,
                        &def.document,
                        &def.source_id,
                        &format!("{}/method/{method_name}/return/{name}", def.source_id),
                        &mut edges,
                    )?;
                }
            }
            self.edges.insert(def.source_id.clone(), edges);
        }

        for (from, targets) in &self.edges {
            for to in targets {
                if from != to && self.reaches(to, from) {
                    self.recursive_edges.insert((from.clone(), to.clone()));
                }
            }
        }
        Ok(())
    }

    fn collect_direct_edges(
        &self,
        param: &SdkParam,
        document: &str,
        owner: &str,
        location: &str,
        edges: &mut BTreeSet<String>,
    ) -> Result<(), LoweringError> {
        match &param.ty {
            SdkTy::Interface { .. } => {
                if self.definitions.contains_key(location)
                    && self.definitions[location].arity.to_fill.is_empty()
                {
                    edges.insert(location.to_owned());
                }
            }
            SdkTy::Path { sdk, ty, args } => {
                let target_document = sdk.as_deref().unwrap_or(document);
                let resolved = self.resolve_path(
                    target_document,
                    ty,
                    Some(args),
                    &mut BTreeSet::new(),
                    location,
                )?;
                if let ResolvedType::Interface { source_id, generic } = resolved
                    && !generic
                    && args.is_empty()
                {
                    edges.insert(source_id);
                }
            }
            // Aggregates and generic parameters lower as `any`, so nested
            // interface references live only in Info and do not affect RIDs.
            _ => {}
        }
        let _ = owner;
        Ok(())
    }

    fn reaches(&self, start: &str, goal: &str) -> bool {
        let mut pending = vec![start.to_owned()];
        let mut visited = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if current == goal {
                return true;
            }
            if visited.insert(current.clone()) {
                if let Some(next) = self.edges.get(&current) {
                    pending.extend(next.iter().cloned());
                }
            }
        }
        false
    }

    fn resolve_path(
        &self,
        document: &str,
        name: &str,
        args: Option<&BTreeMap<String, SdkParam>>,
        visiting: &mut BTreeSet<String>,
        use_site: &str,
    ) -> Result<ResolvedType, LoweringError> {
        let sdk = self
            .documents
            .get(document)
            .ok_or_else(|| LoweringError::MissingDocument(document.to_owned()))?;
        let item = sdk
            .interfaces
            .get(name)
            .ok_or_else(|| LoweringError::MissingDeclaration {
                document: document.to_owned(),
                name: name.to_owned(),
            })?;
        let alias_id = source_id(document, name);
        if !visiting.insert(alias_id.clone()) {
            return Err(LoweringError::AliasCycle(alias_id));
        }
        let alias = self
            .aliases
            .get(&alias_id)
            .ok_or_else(|| match &item.contents {
                SdkItemContents::Method(_) => LoweringError::MethodUsedAsType {
                    document: document.to_owned(),
                    name: name.to_owned(),
                },
                SdkItemContents::Type(_) => LoweringError::MissingDeclaration {
                    document: document.to_owned(),
                    name: name.to_owned(),
                },
            })?;
        let declaration_is_generic = !alias.arity.to_fill.is_empty();
        if let Some(args) = args {
            let declared: BTreeSet<&String> = alias.arity.to_fill.keys().collect();
            let supplied: BTreeSet<&String> = args.keys().collect();
            if declared != supplied {
                return Err(LoweringError::InvalidGenericUse(use_site.to_owned()));
            }
        }
        match &alias.param.ty {
            SdkTy::Interface { .. } => Ok(ResolvedType::Interface {
                source_id: alias_id,
                generic: declaration_is_generic,
            }),
            SdkTy::Path { sdk, ty, args } => {
                let next_document = sdk.as_deref().unwrap_or(&alias.document);
                let mut resolved =
                    self.resolve_path(next_document, ty, Some(args), visiting, use_site)?;
                match &mut resolved {
                    ResolvedType::Interface { generic, .. }
                    | ResolvedType::NonInterface { generic, .. } => {
                        *generic |= declaration_is_generic || !args.is_empty();
                    }
                }
                Ok(resolved)
            }
            _ => Ok(ResolvedType::NonInterface {
                param: alias.param.clone(),
                document: alias.document.clone(),
                generic: declaration_is_generic,
            }),
        }
    }

    fn lower_interface(&mut self, source: &str) -> Result<[u8; 32], LoweringError> {
        if let Some(rid) = self.source_rids.get(source) {
            return Ok(*rid);
        }
        let def = self
            .definitions
            .get(source)
            .cloned()
            .ok_or_else(|| LoweringError::MissingInterface(source.to_owned()))?;
        let mut methods = BTreeMap::new();
        let source_index = self.source_indices[source];
        let mut info_entry = InfoEntry::default();
        info_entry.attrs.push(attr(INFO_VERSION, "1"));
        info_entry.attrs.push(attr(
            &format!("sdk.source.{source_index}.identity.hex"),
            &hex_encode(source.as_bytes()),
        ));
        info_entry.attrs.push(attr(
            &format!("sdk.source.{source_index}.document.hex"),
            &hex_encode(def.document.as_bytes()),
        ));
        if !def.arity.to_fill.is_empty() {
            info_entry
                .attrs
                .push(attr("sdk.lowering.genericized", "any"));
            info_entry.attrs.push(attr(
                &format!("sdk.source.{source_index}.lowering.genericized"),
                "any",
            ));
            info_entry.attrs.push(attr(
                &format!("sdk.source.{source_index}.arity.hex"),
                &hex_encode(def.arity.to_string().as_bytes()),
            ));
        }
        self.record_source_attrs(
            &mut info_entry.attrs,
            source_index,
            &def.interface.attr,
            "root",
        );

        for (method_name, (outer_arity, method)) in &def.interface.methods {
            let method_location = format!("{source}/method/{method_name}");
            let merged_arity =
                merge_arities([&def.arity, outer_arity, &method.arity], &method_location)?;
            let mut params = Vec::new();
            let mut rets = Vec::new();
            let mut method_info = MethEntry::default();
            method_info.attrs.push(attr(
                &format!("sdk.source.{source_index}.method.hex"),
                &hex_encode(method_name.as_bytes()),
            ));
            if !merged_arity.to_fill.is_empty() {
                method_info
                    .attrs
                    .push(attr("sdk.lowering.genericized", "any"));
                method_info.attrs.push(attr(
                    &format!("sdk.source.{source_index}.lowering.genericized"),
                    "any",
                ));
                method_info.attrs.push(attr(
                    &format!("sdk.source.{source_index}.method_arity.hex"),
                    &hex_encode(merged_arity.to_string().as_bytes()),
                ));
            }
            self.record_source_attrs(&mut method_info.attrs, source_index, &method.attr, "method");

            for (name, param) in &method.args {
                let index = params.len();
                let location = format!("{source}/method/{method_name}/param/{name}");
                let (arg, metadata) =
                    self.lower_param(param, &def.document, source, &location, &merged_arity)?;
                let mut entry = ParamEntry::default();
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.name.hex"),
                    &hex_encode(name.as_bytes()),
                ));
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.position"),
                    &index.to_string(),
                ));
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.type.hex"),
                    &hex_encode(param.to_string().as_bytes()),
                ));
                self.add_type_markers(&mut entry.attrs, source_index, &metadata);
                self.record_source_attrs(&mut entry.attrs, source_index, &param.attr, "param");
                method_info.params.insert(index, entry);
                params.push(arg);
            }

            for (name, param) in &method.rets {
                let index = rets.len();
                let location = format!("{source}/method/{method_name}/return/{name}");
                let (arg, metadata) =
                    self.lower_param(param, &def.document, source, &location, &merged_arity)?;
                let mut entry = ParamEntry::default();
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.name.hex"),
                    &hex_encode(name.as_bytes()),
                ));
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.position"),
                    &index.to_string(),
                ));
                entry.attrs.push(attr(
                    &format!("sdk.source.{source_index}.type.hex"),
                    &hex_encode(param.to_string().as_bytes()),
                ));
                self.add_type_markers(&mut entry.attrs, source_index, &metadata);
                self.record_source_attrs(&mut entry.attrs, source_index, &param.attr, "return");
                method_info.returns.insert(index, entry);
                rets.push(arg);
            }

            methods.insert(
                method_name.clone(),
                Sig {
                    ann: Vec::new(),
                    params,
                    rets,
                },
            );
            info_entry.methods.insert(method_name.clone(), method_info);
        }

        let pit_interface = Interface {
            methods,
            ann: Vec::new(),
        };
        let rid = pit_interface.rid();
        self.interfaces.entry(rid).or_insert(pit_interface);
        self.source_rids.insert(source.to_owned(), rid);
        self.info_parts
            .entry(rid)
            .or_default()
            .push((source_index, info_entry));
        Ok(rid)
    }

    fn lower_param(
        &mut self,
        param: &SdkParam,
        document: &str,
        owner: &str,
        location: &str,
        generic_arity: &Arity,
    ) -> Result<(Arg, TypeMetadata), LoweringError> {
        let mut metadata = TypeMetadata::default();
        let ty = match &param.ty {
            SdkTy::I32 => ArgTy::I32,
            SdkTy::I64 => ArgTy::I64,
            SdkTy::F32 => ArgTy::F32,
            SdkTy::F64 => ArgTy::F64,
            SdkTy::Byte => {
                metadata.scalar = Some("byte");
                ArgTy::I32
            }
            SdkTy::I16 => {
                metadata.scalar = Some("i16");
                ArgTy::I32
            }
            SdkTy::Generic { ty, args } => {
                let Some(declared) = generic_arity.to_fill.get(ty) else {
                    return Err(LoweringError::InvalidGenericUse(location.to_owned()));
                };
                let declared_args: BTreeSet<&String> = declared.to_fill.keys().collect();
                let supplied_args: BTreeSet<&String> = args.keys().collect();
                if declared_args != supplied_args {
                    return Err(LoweringError::InvalidGenericUse(location.to_owned()));
                }
                metadata.any_reason = Some("generic-parameter");
                resource(ResTy::None)
            }
            SdkTy::Array { .. } | SdkTy::Struct { .. } | SdkTy::Variant { .. } => {
                metadata.any_reason = Some("aggregate");
                resource(ResTy::None)
            }
            SdkTy::Interface { .. } => {
                let target = location.to_owned();
                let generic = self
                    .definitions
                    .get(&target)
                    .is_some_and(|d| !d.arity.to_fill.is_empty());
                self.interface_reference(owner, target, generic, &mut metadata)?
            }
            SdkTy::Path { sdk, ty, args } => {
                let target_document = sdk.as_deref().unwrap_or(document);
                let resolved = self.resolve_path(
                    target_document,
                    ty,
                    Some(args),
                    &mut BTreeSet::new(),
                    location,
                )?;
                match resolved {
                    ResolvedType::Interface { source_id, generic } => {
                        if !args.is_empty() && !generic {
                            return Err(LoweringError::InvalidGenericUse(location.to_owned()));
                        }
                        self.interface_reference(
                            owner,
                            source_id,
                            generic || !args.is_empty(),
                            &mut metadata,
                        )?
                    }
                    ResolvedType::NonInterface {
                        param: target,
                        document: target_doc,
                        generic,
                    } => {
                        if !args.is_empty() && !generic {
                            return Err(LoweringError::InvalidGenericUse(location.to_owned()));
                        }
                        metadata.expanded_type = Some(target.to_string());
                        if generic {
                            metadata.any_reason = Some(
                                if matches!(
                                    target.ty,
                                    portal_solutions_sdk::SdkTy::Array { .. }
                                        | portal_solutions_sdk::SdkTy::Struct { .. }
                                        | portal_solutions_sdk::SdkTy::Variant { .. }
                                ) {
                                    "aggregate-alias"
                                } else {
                                    "generic-parameter"
                                },
                            );
                            resource(ResTy::None)
                        } else {
                            let empty_arity = Arity::default();
                            let (expanded, expanded_meta) = self.lower_param(
                                &target,
                                &target_doc,
                                owner,
                                location,
                                &empty_arity,
                            )?;
                            metadata.scalar = expanded_meta.scalar;
                            metadata.any_reason = match expanded_meta.any_reason {
                                Some("aggregate") => Some("aggregate-alias"),
                                other => other,
                            };
                            metadata.target_source = expanded_meta.target_source;
                            metadata.target_rid = expanded_meta.target_rid;
                            expanded.ty
                        }
                    }
                }
            }
        };
        Ok((
            Arg {
                ty,
                ann: Vec::new(),
            },
            metadata,
        ))
    }

    fn interface_reference(
        &mut self,
        owner: &str,
        target: String,
        generic: bool,
        metadata: &mut TypeMetadata,
    ) -> Result<ArgTy, LoweringError> {
        metadata.target_source = Some(target.clone());
        if generic {
            metadata.any_reason = Some("generic-interface");
            return Ok(resource(ResTy::None));
        }
        if owner == target {
            return Ok(resource(ResTy::This));
        }
        if self
            .recursive_edges
            .contains(&(owner.to_owned(), target.clone()))
        {
            metadata.any_reason = Some("recursive-interface-reference");
            return Ok(resource(ResTy::None));
        }
        let rid = self.lower_interface(&target)?;
        metadata.target_rid = Some(rid);
        Ok(resource(ResTy::Of(rid)))
    }

    fn add_type_markers(
        &self,
        attrs: &mut Vec<pit_core::Attr>,
        source_index: usize,
        metadata: &TypeMetadata,
    ) {
        if let Some(reason) = metadata.any_reason {
            attrs.push(attr("sdk.lowering.any", "true"));
            attrs.push(attr("sdk.lowering.any_reason", reason));
            attrs.push(attr(
                &format!("sdk.source.{source_index}.lowering.any"),
                "true",
            ));
            attrs.push(attr(
                &format!("sdk.source.{source_index}.lowering.any_reason"),
                reason,
            ));
        }
        if let Some(scalar) = metadata.scalar {
            attrs.push(attr("sdk.lowering.scalar", scalar));
            attrs.push(attr(
                &format!("sdk.source.{source_index}.lowering.scalar"),
                scalar,
            ));
        }
        if let Some(source) = &metadata.target_source {
            attrs.push(attr(
                &format!("sdk.source.{source_index}.target_identity.hex"),
                &hex_encode(source.as_bytes()),
            ));
        }
        if let Some(rid) = metadata.target_rid {
            attrs.push(attr(
                &format!("sdk.source.{source_index}.target_rid.hex"),
                &hex_encode(rid),
            ));
        }
        if let Some(expanded) = &metadata.expanded_type {
            attrs.push(attr(
                &format!("sdk.source.{source_index}.expanded_type.hex"),
                &hex_encode(expanded.as_bytes()),
            ));
        }
    }

    fn record_source_attrs(
        &self,
        destination: &mut Vec<pit_core::Attr>,
        source_index: usize,
        attrs: &[pit_core::Attr],
        _scope: &str,
    ) {
        for (index, source_attr) in attrs.iter().enumerate() {
            destination.push(attr(
                &format!(
                    "sdk.source.{source_index}.attr.{index}.{}.hex",
                    hex_encode(source_attr.name.as_bytes())
                ),
                &hex_encode(source_attr.value.as_bytes()),
            ));
            if DOC_ATTRS.contains(&source_attr.name.as_str()) && source_index == 0 {
                destination.push(source_attr.clone());
            }
        }
    }

    fn add_alias_rids(&mut self) -> Result<(), LoweringError> {
        let aliases: Vec<(String, AliasDef)> = self
            .aliases
            .iter()
            .filter(|(_, alias)| matches!(alias.param.ty, SdkTy::Path { .. }))
            .map(|(id, alias)| (id.clone(), alias.clone()))
            .collect();
        for (index, (alias_id, alias)) in aliases.into_iter().enumerate() {
            let Ok(ResolvedType::Interface { source_id, .. }) = self.resolve_path(
                &alias.document,
                &alias.name,
                None,
                &mut BTreeSet::new(),
                &alias_id,
            ) else {
                continue;
            };
            let Some(rid) = self.source_rids.get(&source_id).copied() else {
                continue;
            };

            let prefix = format!("sdk.source.alias.{index}");
            let mut entry = InfoEntry::default();
            entry.attrs.push(attr(
                &format!("{prefix}.identity.hex"),
                &hex_encode(alias_id.as_bytes()),
            ));
            entry.attrs.push(attr(
                &format!("{prefix}.document.hex"),
                &hex_encode(alias.document.as_bytes()),
            ));
            entry.attrs.push(attr(
                &format!("{prefix}.name.hex"),
                &hex_encode(alias.name.as_bytes()),
            ));
            entry.attrs.push(attr(
                &format!("{prefix}.target_identity.hex"),
                &hex_encode(source_id.as_bytes()),
            ));
            entry
                .attrs
                .push(attr(&format!("{prefix}.target_rid.hex"), &hex_encode(rid)));
            entry.attrs.push(attr(
                &format!("{prefix}.type.hex"),
                &hex_encode(alias.param.to_string().as_bytes()),
            ));
            if !alias.arity.to_fill.is_empty() {
                entry.attrs.push(attr(
                    &format!("{prefix}.arity.hex"),
                    &hex_encode(alias.arity.to_string().as_bytes()),
                ));
            }
            for (attr_index, source_attr) in alias.param.attr.iter().enumerate() {
                entry.attrs.push(attr(
                    &format!(
                        "{prefix}.attr.{attr_index}.{}.hex",
                        hex_encode(source_attr.name.as_bytes())
                    ),
                    &hex_encode(source_attr.value.as_bytes()),
                ));
            }
            self.source_rids.insert(alias_id, rid);
            self.info_parts
                .entry(rid)
                .or_default()
                .push((self.source_indices.len() + index, entry));
        }
        Ok(())
    }

    fn finish(mut self) -> LoweredSdk {
        for entries in self.info_parts.values_mut() {
            for (_, entry) in entries {
                complete_target_rids(entry, &self.source_rids);
            }
        }

        let mut info = Info::default();
        for (rid, mut entries) in std::mem::take(&mut self.info_parts) {
            entries.sort_by_key(|(index, _)| std::cmp::Reverse(*index));
            let mut merged = InfoEntry::default();
            for (_, entry) in entries {
                merged = merged.merge(entry);
            }
            info.interfaces.insert(rid, merged);
        }
        LoweredSdk {
            interfaces: self.interfaces,
            info,
            source_rids: self.source_rids,
        }
    }
}

fn complete_target_rids(entry: &mut InfoEntry, source_rids: &BTreeMap<String, [u8; 32]>) {
    fn update_attrs(attrs: &mut Vec<pit_core::Attr>, source_rids: &BTreeMap<String, [u8; 32]>) {
        let targets: Vec<(String, [u8; 32])> = attrs
            .iter()
            .filter_map(|source_attr| {
                let marker = source_attr.name.strip_suffix(".target_identity.hex")?;
                let identity = hex::decode(&source_attr.value).ok()?;
                let identity = String::from_utf8(identity).ok()?;
                let rid = source_rids.get(&identity)?;
                Some((format!("{marker}.target_rid.hex"), *rid))
            })
            .collect();
        for (name, rid) in targets {
            if !attrs.iter().any(|existing| existing.name == name) {
                attrs.push(attr(&name, &hex_encode(rid)));
            }
        }
    }

    for method in entry.methods.values_mut() {
        for param in method
            .params
            .values_mut()
            .chain(method.returns.values_mut())
        {
            update_attrs(&mut param.attrs, source_rids);
        }
    }
}

fn merge_arities<'a>(
    arities: impl IntoIterator<Item = &'a Arity>,
    location: &str,
) -> Result<Arity, LoweringError> {
    fn merge_into(
        destination: &mut BTreeMap<String, Arity>,
        source: &BTreeMap<String, Arity>,
        location: &str,
    ) -> Result<(), LoweringError> {
        for (name, arity) in source {
            if let Some(previous) = destination.get(name) {
                return Err(if previous == arity {
                    LoweringError::DuplicateGenericParameter {
                        location: location.to_owned(),
                        name: name.clone(),
                    }
                } else {
                    LoweringError::IncompatibleGenericParameter {
                        location: location.to_owned(),
                        name: name.clone(),
                    }
                });
            }
            destination.insert(name.clone(), arity.clone());
        }
        Ok(())
    }

    let mut merged = Arity::default();
    for arity in arities {
        merge_into(&mut merged.to_fill, &arity.to_fill, location)?;
    }
    Ok(merged)
}

fn source_id(document: &str, name: &str) -> String {
    format!("{}:{document}#{name}", document.len())
}

fn attr(name: &str, value: &str) -> pit_core::Attr {
    pit_core::Attr {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

fn resource(ty: ResTy) -> ArgTy {
    ArgTy::Resource {
        ty,
        nullable: false,
        take: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(ty: SdkTy) -> SdkParam {
        SdkParam {
            ty,
            attr: Vec::new(),
        }
    }

    fn interface(methods: impl IntoIterator<Item = (String, SdkMethod)>) -> SdkInterface {
        SdkInterface {
            methods: methods
                .into_iter()
                .map(|(name, method)| (name, (Arity::default(), method)))
                .collect(),
            attr: Vec::new(),
        }
    }

    fn method(
        args: impl IntoIterator<Item = (String, SdkParam)>,
        rets: impl IntoIterator<Item = (String, SdkParam)>,
    ) -> SdkMethod {
        SdkMethod {
            arity: Arity::default(),
            args: args.into_iter().collect(),
            rets: rets.into_iter().collect(),
            attr: Vec::new(),
        }
    }

    fn catalog(types: impl IntoIterator<Item = (String, SdkParam)>) -> BTreeMap<String, Sdk> {
        BTreeMap::from([(
            "root.sdk".to_owned(),
            Sdk {
                interfaces: types
                    .into_iter()
                    .map(|(name, value)| {
                        (
                            name,
                            portal_solutions_sdk::SdkItem {
                                generics: Arity::default(),
                                contents: SdkItemContents::Type(value),
                            },
                        )
                    })
                    .collect(),
            },
        )])
    }

    #[test]
    fn only_sdk_interfaces_emit_pit_interfaces_and_aggregates_are_info_any() {
        let api = interface([(
            "submit".to_owned(),
            method(
                [(
                    "payload".to_owned(),
                    param(SdkTy::Struct {
                        items: BTreeMap::from([("count".to_owned(), param(SdkTy::I16))]),
                    }),
                )],
                [("ok".to_owned(), param(SdkTy::Byte))],
            ),
        )]);
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([(
                "Api".to_owned(),
                param(SdkTy::Interface {
                    implementation: api,
                }),
            )]),
        )
        .unwrap();

        assert_eq!(lowered.interfaces.len(), 1);
        let (rid, iface) = lowered.interfaces.iter().next().unwrap();
        let sig = &iface.methods["submit"];
        assert!(matches!(
            sig.params[0].ty,
            ArgTy::Resource {
                ty: ResTy::None,
                ..
            }
        ));
        assert!(matches!(sig.rets[0].ty, ArgTy::I32));
        let entry = &lowered.info.interfaces[rid];
        assert_eq!(
            entry
                .attrs
                .iter()
                .find(|a| a.name == INFO_VERSION)
                .unwrap()
                .value,
            "1"
        );
        let payload = &entry.methods["submit"].params[&0].attrs;
        assert!(
            payload
                .iter()
                .any(|a| a.name == "sdk.lowering.any_reason" && a.value == "aggregate")
        );
        let result = &entry.methods["submit"].returns[&0].attrs;
        assert!(
            result
                .iter()
                .any(|a| a.name == "sdk.lowering.scalar" && a.value == "byte")
        );
    }

    #[test]
    fn identical_pit_signatures_keep_distinct_source_type_markers() {
        let byte_api = interface([(
            "value".to_owned(),
            method([("input".to_owned(), param(SdkTy::Byte))], []),
        )]);
        let int_api = interface([(
            "value".to_owned(),
            method([("input".to_owned(), param(SdkTy::I32))], []),
        )]);
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([
                (
                    "ByteApi".to_owned(),
                    param(SdkTy::Interface {
                        implementation: byte_api,
                    }),
                ),
                (
                    "IntApi".to_owned(),
                    param(SdkTy::Interface {
                        implementation: int_api,
                    }),
                ),
            ]),
        )
        .unwrap();
        assert_eq!(lowered.interfaces.len(), 1);
        let info = lowered.info.interfaces.values().next().unwrap();
        let attrs = &info.methods["value"].params[&0].attrs;
        assert!(
            attrs.iter().any(|attr| {
                attr.name == "sdk.source.0.lowering.scalar" && attr.value == "byte"
            })
        );
        assert!(
            !attrs
                .iter()
                .any(|attr| attr.name == "sdk.source.1.lowering.scalar")
        );
        assert!(attrs.iter().any(|attr| {
            attr.name == "sdk.source.1.type.hex"
                && attr.value == hex_encode(param(SdkTy::I32).to_string().as_bytes())
        }));
    }

    #[test]
    fn mutual_interface_references_are_marked_any_without_extra_interfaces() {
        let a = interface([(
            "next".to_owned(),
            method(
                [(
                    "other".to_owned(),
                    param(SdkTy::Path {
                        sdk: None,
                        ty: "B".to_owned(),
                        args: BTreeMap::new(),
                    }),
                )],
                [],
            ),
        )]);
        let b = interface([(
            "next".to_owned(),
            method(
                [(
                    "other".to_owned(),
                    param(SdkTy::Path {
                        sdk: None,
                        ty: "A".to_owned(),
                        args: BTreeMap::new(),
                    }),
                )],
                [],
            ),
        )]);
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([
                (
                    "A".to_owned(),
                    param(SdkTy::Interface { implementation: a }),
                ),
                (
                    "B".to_owned(),
                    param(SdkTy::Interface { implementation: b }),
                ),
            ]),
        )
        .unwrap();

        assert_eq!(
            lowered.interfaces.len(),
            1,
            "identical PIT signatures share their content-addressed RID"
        );
        assert_eq!(lowered.source_rids.len(), 2);
        let iface = lowered.interfaces.values().next().unwrap();
        assert!(matches!(
            iface.methods["next"].params[0].ty,
            ArgTy::Resource {
                ty: ResTy::None,
                ..
            }
        ));
        let info = &lowered.info.interfaces[&iface.rid()];
        let param_info = &info.methods["next"].params[&0].attrs;
        assert!(
            param_info
                .iter()
                .any(|a| a.name == "sdk.lowering.any_reason"
                    && a.value == "recursive-interface-reference")
        );
        assert_eq!(
            info.attrs
                .iter()
                .filter(|a| a.name.contains("identity.hex") && !a.name.contains("target_identity"))
                .count(),
            2
        );
        assert!(
            param_info
                .iter()
                .any(|a| a.name.contains("target_identity.hex"))
        );
    }

    #[test]
    fn direct_self_reference_uses_this() {
        let api = interface([(
            "child".to_owned(),
            method(
                [],
                [(
                    "value".to_owned(),
                    param(SdkTy::Path {
                        sdk: None,
                        ty: "Api".to_owned(),
                        args: BTreeMap::new(),
                    }),
                )],
            ),
        )]);
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([(
                "Api".to_owned(),
                param(SdkTy::Interface {
                    implementation: api,
                }),
            )]),
        )
        .unwrap();
        let iface = lowered.interfaces.values().next().unwrap();
        assert!(matches!(
            iface.methods["child"].rets[0].ty,
            ArgTy::Resource {
                ty: ResTy::This,
                ..
            }
        ));
    }

    #[test]
    fn interface_aliases_share_rids_and_are_preserved_in_info() {
        let api = param(SdkTy::Interface {
            implementation: interface([("ping".to_owned(), method([], []))]),
        });
        let alias = param(SdkTy::Path {
            sdk: None,
            ty: "Api".to_owned(),
            args: BTreeMap::new(),
        });
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([("Api".to_owned(), api), ("Alias".to_owned(), alias)]),
        )
        .unwrap();

        assert_eq!(lowered.interfaces.len(), 1);
        assert_eq!(lowered.source_rids.len(), 2);
        let info = lowered.info.interfaces.values().next().unwrap();
        assert!(
            info.attrs
                .iter()
                .any(|attr| attr.name.starts_with("sdk.source.alias.0.identity.hex"))
        );
        let (_, parsed) = Info::parse(&lowered.info.to_string()).unwrap();
        assert_eq!(parsed, lowered.info);
    }

    #[test]
    fn generic_path_arguments_must_match_declared_arity() {
        let mut generic_arity = Arity::default();
        generic_arity
            .to_fill
            .insert("T".to_owned(), Arity::default());
        let target = portal_solutions_sdk::SdkItem {
            generics: generic_arity,
            contents: SdkItemContents::Type(param(SdkTy::Interface {
                implementation: interface([("ping".to_owned(), method([], []))]),
            })),
        };
        let caller = portal_solutions_sdk::SdkItem {
            generics: Arity::default(),
            contents: SdkItemContents::Type(param(SdkTy::Interface {
                implementation: interface([(
                    "call".to_owned(),
                    method(
                        [(
                            "value".to_owned(),
                            param(SdkTy::Path {
                                sdk: None,
                                ty: "Target".to_owned(),
                                args: BTreeMap::new(),
                            }),
                        )],
                        [],
                    ),
                )]),
            })),
        };
        let docs = BTreeMap::from([(
            "root.sdk".to_owned(),
            Sdk {
                interfaces: BTreeMap::from([
                    ("Caller".to_owned(), caller),
                    ("Target".to_owned(), target),
                ]),
            },
        )]);
        assert!(matches!(
            lower_sdk("root.sdk", &docs),
            Err(LoweringError::InvalidGenericUse(_))
        ));
    }

    #[test]
    fn generic_parameters_require_matching_declarations_and_arguments() {
        let mut method_with_generic = method(
            [(
                "value".to_owned(),
                param(SdkTy::Generic {
                    ty: "T".to_owned(),
                    args: BTreeMap::new(),
                }),
            )],
            [],
        );
        method_with_generic
            .arity
            .to_fill
            .insert("T".to_owned(), Arity::default());
        let api = interface([("echo".to_owned(), method_with_generic)]);
        let lowered = lower_sdk(
            "root.sdk",
            &catalog([(
                "Api".to_owned(),
                param(SdkTy::Interface {
                    implementation: api,
                }),
            )]),
        )
        .unwrap();
        let info = lowered.info.interfaces.values().next().unwrap();
        let attrs = &info.methods["echo"].params[&0].attrs;
        assert!(attrs.iter().any(|attr| {
            attr.name == "sdk.lowering.any_reason" && attr.value == "generic-parameter"
        }));

        let invalid = interface([(
            "echo".to_owned(),
            method(
                [(
                    "value".to_owned(),
                    param(SdkTy::Generic {
                        ty: "Missing".to_owned(),
                        args: BTreeMap::new(),
                    }),
                )],
                [],
            ),
        )]);
        assert!(matches!(
            lower_sdk(
                "root.sdk",
                &catalog([(
                    "Api".to_owned(),
                    param(SdkTy::Interface {
                        implementation: invalid,
                    }),
                )])
            ),
            Err(LoweringError::InvalidGenericUse(_))
        ));
    }

    #[test]
    fn generic_interface_erasure_retains_target_identity_and_rid() {
        let mut target_arity = Arity::default();
        target_arity
            .to_fill
            .insert("T".to_owned(), Arity::default());
        let target = portal_solutions_sdk::SdkItem {
            generics: target_arity,
            contents: SdkItemContents::Type(param(SdkTy::Interface {
                implementation: interface([("ping".to_owned(), method([], []))]),
            })),
        };
        let caller = portal_solutions_sdk::SdkItem {
            generics: Arity::default(),
            contents: SdkItemContents::Type(param(SdkTy::Interface {
                implementation: interface([(
                    "call".to_owned(),
                    method(
                        [(
                            "target".to_owned(),
                            param(SdkTy::Path {
                                sdk: None,
                                ty: "Target".to_owned(),
                                args: BTreeMap::from([("T".to_owned(), param(SdkTy::I32))]),
                            }),
                        )],
                        [],
                    ),
                )]),
            })),
        };
        let docs = BTreeMap::from([(
            "root.sdk".to_owned(),
            Sdk {
                interfaces: BTreeMap::from([
                    ("Caller".to_owned(), caller),
                    ("Target".to_owned(), target),
                ]),
            },
        )]);
        let lowered = lower_sdk("root.sdk", &docs).unwrap();
        let caller_rid = lowered.source_rids[&source_id("root.sdk", "Caller")];
        let target_rid = lowered.source_rids[&source_id("root.sdk", "Target")];
        let attrs = &lowered.info.interfaces[&caller_rid].methods["call"].params[&0].attrs;
        assert!(attrs.iter().any(|attr| {
            attr.name == "sdk.lowering.any_reason" && attr.value == "generic-interface"
        }));
        assert!(attrs.iter().any(|attr| {
            attr.name == "sdk.source.0.target_rid.hex" && attr.value == hex_encode(target_rid)
        }));
    }

    #[test]
    fn interface_and_method_generic_names_cannot_conflict() {
        let mut arity = Arity::default();
        arity.to_fill.insert("T".to_owned(), Arity::default());
        let mut api = interface([("ping".to_owned(), method([], []))]);
        api.methods.get_mut("ping").unwrap().1.arity = arity.clone();
        let docs = BTreeMap::from([(
            "root.sdk".to_owned(),
            Sdk {
                interfaces: BTreeMap::from([(
                    "Api".to_owned(),
                    portal_solutions_sdk::SdkItem {
                        generics: arity,
                        contents: SdkItemContents::Type(param(SdkTy::Interface {
                            implementation: api,
                        })),
                    },
                )]),
            },
        )]);
        assert!(matches!(
            lower_sdk("root.sdk", &docs),
            Err(LoweringError::DuplicateGenericParameter { .. })
        ));
    }

    #[test]
    fn standalone_method_is_rejected() {
        let docs = BTreeMap::from([(
            "root.sdk".to_owned(),
            Sdk {
                interfaces: BTreeMap::from([(
                    "orphan".to_owned(),
                    portal_solutions_sdk::SdkItem {
                        generics: Arity::default(),
                        contents: SdkItemContents::Method(method([], [])),
                    },
                )]),
            },
        )]);
        assert!(matches!(
            lower_sdk("root.sdk", &docs),
            Err(LoweringError::StandaloneMethod { .. })
        ));
    }
}
