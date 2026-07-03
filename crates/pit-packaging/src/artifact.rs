use std::string::String;

/// Target language for routing and manifest src-list filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Rust,
    Java,
    Scala,
    C,
    Ts,
    Other,
}

impl Language {
    pub fn from_path(path: &str) -> Self {
        if path.ends_with(".rs") {
            Language::Rust
        } else if path.ends_with(".java") {
            Language::Java
        } else if path.ends_with(".scala") {
            Language::Scala
        } else if path.ends_with(".h") || path.ends_with(".c") {
            Language::C
        } else if path.ends_with(".ts") {
            Language::Ts
        } else {
            Language::Other
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitScope {
    PerInterface,
    /// Emitted once per bundle run (Handler.java, shared JS runtime, …).
    SharedOnce,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedArtifact {
    pub path: String,
    pub content: String,
    pub language: Language,
    pub scope: EmitScope,
    /// Java/Scala package for directory routing.
    pub package: Option<String>,
}

impl EmittedArtifact {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        let path = path.into();
        let language = Language::from_path(&path);
        Self {
            path,
            content: content.into(),
            language,
            scope: EmitScope::PerInterface,
            package: None,
        }
    }

    pub fn shared(mut self) -> Self {
        self.scope = EmitScope::SharedOnce;
        self
    }

    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
}
