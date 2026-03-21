# Well Structured Type

WST is a language for defining types that compile to TypeScript and Rust.

## Primitives

| WST | TypeScript | Rust |
|-----|-----------|------|
| `int32` | `number` | `i32` |
| `int64` | `number` | `i64` |
| `uin64` | `number` | `u64` |
| `flt64` | `number` | `f64` |
| `boolean` | `boolean` | `bool` |
| `string` | `string` | `String` |

## Composites

| WST | TypeScript | Rust |
|-----|-----------|------|
| `vec<T>` | `T[]` | `Vec<T>` |
| `map<P, T>` | `Record<P, T>` | `HashMap<P, T>` |

Map keys (`P`) must be a primitive type — primitives have well-defined equality and hashing semantics.

## Enums

Enums define a fixed set of named values with no associated data.

```wst
enum Direction {
    North,
    South,
    East,
    West,
}
```

TypeScript output:
```ts
export enum Direction {
  North = "North",
  South = "South",
  East = "East",
  West = "West",
}
```

Rust output:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}
```

## Variants

Variants are tagged unions — each case carries an associated type. Use a variant when different cases need to hold different data.

```wst
variant Result {
    Ok = string,
    Err = int32,
}
```

TypeScript output:
```ts
export type Result =
  | { Ok: string }
  | { Err: number }
;
```

Rust output:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Result {
    Ok(String),
    Err(i32),
}
```

## Structs

```wst
struct Point {
    x = flt64 #required,
    y = flt64 #required,
    label = string,
}
```

TypeScript output:
```ts
export interface Point {
  x: number;
  y: number;
  label?: string | null;
}
```

Rust output:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
```

## Tags

Tags annotate fields and cases with metadata. They appear after the field name or type.

### Built-in tags

| Tag | Meaning |
|-----|---------|
| `#required` | Field must be present (struct fields only) |
| `#optional` | Explicitly marks a field optional — the default, so rarely needed |
| `#nullable` | Field value may be `null` (see [Nullability](#nullability)) |
| `#deprecated` | Field or case is deprecated |
| `#banned` | Field is banned; emits `never` in TypeScript, `#[deprecated]` in Rust |

### Custom tags

Custom tags require a namespace prefix to prevent collision with built-in tags.

```wst
struct Foo {
    id = int64 #required #myorg:indexed,
    score = flt64 #myorg:precision=2,
    note = string #myorg:description="user-visible note",
}
```

Tags default to `true` when no value is given. Supported value types: string (`""`), number, boolean.

### Multi-line tags

Use `[...]` to expand tags across multiple lines.

```wst
struct Foo {
    score = flt64 [
        #required
        #nullable
        #myorg:precision=2
    ],
}

protocol Api {
    "/create" [
        #myorg:auth=true
        #myorg:version=2
    ] <Request, Response !Error>,
}
```

### Definition-level tags

Tags can appear on any definition — `enum`, `variant`, `struct`, or `protocol` — between the name and the opening `{`.

```wst
struct MyStruct #myorg:since="v2" {
    id = int64 #required,
}

enum Status [
    #myorg:codegen:exhaustive
] {
    Active,
    Inactive,
}
```

Definition-level tags follow the same inline/multi-line rules. Built-in struct-only tags (`#required`, `#optional`) are not valid at definition level.

## Protocols

Protocols define typed endpoints or RPCs.

```wst
protocol UserApi {
    "/users/get"    <GetRequest, User !ApiError>,
    "/users/create" <CreateRequest, User !ApiError>,
    "/users/list"   <ListRequest, vec<User>>,
}
```

The `!ErrorType` slot specifies the typed error for an endpoint. Omit it for endpoints that don't return typed errors.

## Private types

Prefix any definition with `private` to exclude it from exports and prevent it from being imported.

```wst
private struct InternalToken {
    value = string #required,
}

private enum InternalStatus { Ok, Fail }
```

Private types still participate in copy resolution and assertions within the same file.

## Enforcers

Enforcers are file-level directives declared at the top of a file. They impose constraints on all definitions within it.

```wst
!optional_mode=implicit;
```

### `!optional_mode`

Controls how optional (non-`#required`) struct fields are typed in TypeScript output.

| Value | TypeScript output for optional fields |
|-------|--------------------------------------|
| `implicit` (default) | `field?: T \| null` |
| `explicit` | `field: T \| null` |

```wst
!optional_mode=explicit;

struct Foo {
    bar = string,       // explicit: bar: string | null
    baz = string #required, // required: baz: string
}
```

This enforcer only affects TypeScript. Rust output is always `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`.

## Nullability

### `#nullable`

Use `#nullable` to allow a field's value to be `null`.

Combined with `#required`, the key is always present but the value may be null:

```wst
struct User {
    name     = string #required,
    nickname = string #nullable #required,  // always present, value can be null
    bio      = string,                      // optional field
}
```

TypeScript:
```ts
export interface User {
  name: string;
  nickname: string | null;
  bio?: string | null;
}
```

Rust:
```rust
pub struct User {
    pub name: String,
    pub nickname: Option<String>,   // no skip_serializing_if — always serialized
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
}
```

## Imports

Import specific types from another file:

```wst
@import ./models.wst { User, Role };
```

By default, imports are linked — generated code references the type as coming from its source module.

Use `^copy` to inline the imported types directly into this file's output:

```wst
@import ./models.wst ^copy { User, Role };
```

Import an entire file under a namespace alias:

```wst
@import ./models.wst *Models;

struct Session {
    user = Models.User #required,
}
```

## Struct copy

Use `copy` inside a struct to inline all fields from another struct. Copy directives must appear before any field definitions.

```wst
struct Base {
    id        = int64  #required,
    name      = string #required,
    created_at = string,
}

struct Extended {
    copy Base,
    email = string #required,
}
```

`Extended` compiles as if it had all of `Base`'s fields followed by its own. Multiple copies are allowed. A field name may not appear more than once across all copies and direct fields.

## @-Ops

`@-ops` are compile-time type transformations. They can be used anywhere a struct type reference is accepted (e.g. in `copy` directives).

### `@exclude`

```wst
@exclude(StructType, ["field1", "field2"])
```

Returns the struct with the listed fields removed.

```wst
struct Record {
    id       = int64  #required,
    name     = string #required,
    internal = string,
}

struct PublicRecord {
    copy @exclude(Record, ["internal"]),
}
```

TypeScript:
```ts
export interface PublicRecord {
  id: number;
  name: string;
}
```

`@exclude` can be combined with additional fields:

```wst
struct Summary {
    copy @exclude(Record, ["internal"]),
    summary = string #required,
}
```

## Enum field keys

Struct field names can be qualified with an enum type using dot notation. This is useful when field names are driven by an enum's cases.

```wst
enum Env { production, staging }

struct DeploymentConfig {
    Env.production = InstanceConfig,
    Env.staging    = InstanceConfig,
}
```

The qualifier is stripped in generated output — the emitted field name is the part after the dot. In Rust, `#[serde(rename = "Env.production")]` is added so the wire format preserves the qualified name.

This pairs naturally with assertions:

```wst
struct DeploymentConfig {
    Env.production = InstanceConfig,
    Env.staging    = InstanceConfig,
    assert ($s) {
        for $k in Env { $s haskey $k }
    },
}
```

## Assertions

Assertions are compile-time checks that validate structural invariants. They run during validation and produce no output in generated code.

### Inline assertions

Attach an `assert` block directly to a struct. `$s` is a metavariable bound to the struct under test.

```wst
enum Env { production, staging }

struct DeploymentConfig {
    Env.production = InstanceConfig,
    Env.staging    = InstanceConfig,
    assert ($s) {
        for $k in Env {
            $s haskey $k
        }
    },
}
```

This asserts that `DeploymentConfig` has a field for every case in `Env`.

### Named assertions

Define a reusable assertion with `assertion`, then reference it by name:

```wst
assertion CoversEnv (struct $s) {
    for $k in Env {
        $s haskey $k
    }
}

struct DeploymentConfig {
    Env.production = InstanceConfig,
    Env.staging    = InstanceConfig,
    assert CoversEnv,
}
```

Assertion failures are reported at the `assert` call site, not inside the assertion definition body.

### Assertion syntax

| Construct | Description |
|-----------|-------------|
| `assert ($s) { ... }` | Inline assertion; `$s` bound to the struct |
| `assert Name` | Reference a named assertion |
| `assertion Name (struct $s) { ... }` | Top-level named assertion definition |
| `for $k in EnumName { ... }` | Iterates over every case of `EnumName`, binding each name to `$k` |
| `$s haskey $k` | Asserts that struct `$s` has a field whose name matches the current value of `$k` |

Metavariables are always prefixed with `$`. Field name matching in `haskey` is exact (case-sensitive).

## Constants

Constants define named values. Names must be **SCREAMING_CASE**.

```wst
const DEFAULT_ERROR = ApiError.NotFound

const DEFAULT_CONFIG = ServiceConfig {
    timeout_ms = 5000,
    retries    = 3,
    label      = "default",
}
```

### Expression syntax

| Value | Example |
|-------|---------|
| String | `"hello"` |
| Number | `42`, `3.14` |
| Boolean | `true`, `false` |
| Null | `null` |
| Enum case | `Status.Active` |
| Struct literal | `Type { field = value, ... }` |

Struct literals support nesting and dotted field keys. Unspecified optional fields are automatically filled with `null` / `None` in output.

### TypeScript output

```ts
export const DEFAULT_ERROR: ApiError = ApiError.NotFound;

export const DEFAULT_CONFIG: ServiceConfig = {
  timeout_ms: 5000,
  retries: 3,
  label: "default",
};
```

### Rust output

**Enum cases and non-string structs** compile to `pub const` or `pub static`:

```rust
pub const DEFAULT_ERROR: ApiError = ApiError::NotFound;
```

**Structs containing string fields** use `std::sync::LazyLock` (stable since Rust 1.80), because `String` is heap-allocated and cannot be constructed in a `const`/`static` initializer. `LazyLock` initialises once on first access:

```rust
pub static DEFAULT_CONFIG: std::sync::LazyLock<ServiceConfig> = std::sync::LazyLock::new(|| ServiceConfig {
    timeout_ms: 5000,
    retries: 3,
    label: "default".to_string(),
    other_optional_field: None,
});
```

String values are emitted as `"...".to_string()`. Optional fields not present in the constant are explicitly set to `None` — no `Default` trait required.
