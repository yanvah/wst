use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool { !b }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub enforcers: Vec<Enforcer>,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertion_defs: Vec<AssertionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enforcer {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImportKind {
    Named { copy: bool, types: Vec<String> },
    Namespace { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Definition {
    Enum(EnumDef),
    Variant(VariantDef),
    Struct(StructDef),
    Protocol(ProtocolDef),
    Const(ConstDef),
}

// ── Constants ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstDef {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    Str { value: String },
    Number { value: f64 },
    Bool { value: bool },
    Null,
    Struct {
        ty: String,
        fields: Vec<ExprField>,
    },
    EnumCase {
        ty: String,
        case: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExprField {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    pub cases: Vec<EnumCase>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumCase {
    pub name: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    pub cases: Vec<VariantCase>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantCase {
    pub name: String,
    pub ty: TypeRef,
    pub tags: Vec<Tag>,
    #[serde(skip)]
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copies: Vec<StructSource>,
    pub fields: Vec<StructField>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asserts: Vec<AssertRef>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
}

// ── Assertions ────────────────────────────────────────────────────────────────

/// A top-level reusable assertion definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionDef {
    pub name: String,
    /// The metavariable name (without `$`) that refers to the struct under test.
    pub param: String,
    pub body: Vec<AssertionStmt>,
}

/// A statement inside an assertion body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum AssertionStmt {
    ForIn {
        /// Loop variable name (without `$`).
        var: String,
        /// Enum to iterate over.
        source: String,
        body: Vec<AssertionStmt>,
        #[serde(skip)]
        line: usize,
    },
    HasKey {
        /// The struct metavariable being tested (without `$`).
        subject: String,
        /// The metavariable holding the key to check (without `$`).
        key: String,
        #[serde(skip)]
        line: usize,
    },
}

/// An assertion attached to a struct — either inline or a reference to a named assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssertRef {
    Inline {
        /// Metavariable name bound to the struct (without `$`).
        param: String,
        body: Vec<AssertionStmt>,
        #[serde(skip)]
        line: usize,
    },
    Named {
        name: String,
        #[serde(skip)]
        line: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum StructSource {
    Named { name: String },
    Exclude { base: String, exclude: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: TypeRef,
    pub tags: Vec<Tag>,
    #[serde(skip)]
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    pub endpoints: Vec<Endpoint>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    pub tags: Vec<Tag>,
    pub request: TypeRef,
    pub response: TypeRef,
    pub error: Option<TypeRef>,
    #[serde(skip)]
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "of")]
pub enum TypeRef {
    Primitive(Primitive),
    Vec(Box<TypeRef>),
    Map { key: Primitive, value: Box<TypeRef> },
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Primitive {
    Int32,
    Int64,
    Uin64,
    Flt64,
    Boolean,
    #[serde(rename = "String")]
    Str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub namespace: Option<String>,
    pub name: String,
    pub value: TagValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TagValue {
    Bool(bool),
    Number(f64),
    Str(String),
}
