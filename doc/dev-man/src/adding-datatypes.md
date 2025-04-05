# Adding new datatypes

This document outlines the process for adding a new datatype to the Moor language. Adding a new datatype requires changes in multiple parts of the codebase to ensure proper integration with the language's type system, serialization mechanisms, and runtime behavior.

## Overview of Steps

1. Define the datatype in the `var` crate
2. Implement necessary traits for the datatype
3. Update the `Variant` enum to include the new type
4. Implement serialization/deserialization
5. Add JavaScript/TypeScript support for web clients
6. Update documentation and tests

## Detailed Steps

### 1. Define the Datatype in the `var` crate

Create a new file in `crates/var/src/` for your datatype (e.g., `newtype.rs`). Define the structure and basic operations for your type.

```rust
// Example for a hypothetical "Set" type
pub struct Set(Arc<HashSet<Var>>);

impl Set {
    pub fn new() -> Self {
        Set(Arc::new(HashSet::new()))
    }
    
    pub fn from_iter<I: IntoIterator<Item = Var>>(iter: I) -> Self {
        Set(Arc::new(iter.into_iter().collect()))
    }
    
    // Add other methods specific to your type
}
```

### 2. Implement Necessary Traits

Implement the required traits for your datatype:

- If it's a sequence-like type, implement the `Sequence` trait
- If it's an associative type, implement the `Associative` trait
- Implement `Clone`, `Debug`, `PartialEq`, etc.

```rust
impl Sequence for Set {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
    
    // Implement other required methods
}
```

### 3. Update the `Variant` Enum

Modify `crates/var/src/variant.rs` to include your new type in the `Variant` enum:

```rust
pub enum Variant {
    None,
    Bool(bool),
    Obj(Obj),
    Int(i64),
    Float(f64),
    List(List),
    Str(string::Str),
    Map(map::Map),
    Err(Error),
    Flyweight(Flyweight),
    Sym(Symbol),
    Set(Set), // Add your new type here
}
```

Then update all the implementations for `Variant` to handle your new type:

- `Hash`
- `Ord`/`PartialOrd`
- `Debug`
- `PartialEq`/`Eq`

Update the `type_class` method in `Var` to properly classify your new type:

```rust
pub fn type_class(&self) -> TypeClass {
    match self.variant() {
        Variant::List(s) => TypeClass::Sequence(s),
        Variant::Flyweight(f) => TypeClass::Sequence(f),
        Variant::Str(s) => TypeClass::Sequence(s),
        Variant::Set(s) => TypeClass::Sequence(s), // Add your type
        Variant::Map(m) => TypeClass::Associative(m),
        _ => TypeClass::Scalar,
    }
}
```

### 4. Implement Serialization/Deserialization

Ensure your type implements the necessary traits for serialization:

```rust
impl BincodeAsByteBufferExt for Set {}
```

Make sure your type implements `Encode` and `Decode` from bincode:

```rust
#[derive(Clone, Encode, Decode)]
pub struct Set(Arc<HashSet<Var>>);
```

For JSON serialization (in `crates/web-host/src/host/mod.rs`):
- Update `var_as_json` to handle your new type
- Update `json_as_var` to parse your type from JSON

Update `moo_value_to_json` in `crates/kernel/src/builtins/bf_strings.rs`, so your value works with the generate and parse json builtins. (until we make this a utility)

### 5. Add JavaScript/TypeScript Support

Update the TypeScript definitions in `crates/web-host/src/client/var.ts`:

```typescript
// Add a new type representation
export class Set {
    elements: Array<any>;
    
    constructor(elements = []) {
        this.elements = elements;
    }
    
    // Add methods as needed
}

// Update jsonToValue and valueToJson functions
export function jsonToValue(json: JSON) {
    // ...existing code...
    } else if (json["set"] != null) {
        return new Set(json["set"].map(jsonToValue));
    }
    // ...
}

export function valueToJson(v) {
    // ...existing code...
    } else if (v instanceof Set) {
        return { set: v.elements.map(valueToJson) };
    }
    // ...
}
```

### 6. Add Constructor Functions to `var.rs`

Add constructor functions to create instances of your new type:

```rust
pub fn v_set(elements: &[Var]) -> Var {
    Var::mk_set(elements)
}

pub fn v_empty_set() -> Var {
    v_set(&[])
}
```

### 7. Update Documentation and Tests

- Add tests for your new datatype in the appropriate test files
- Update documentation to reflect the new datatype
- Add examples showing how to use the new datatype

## Integration with MOO Language Features

If your datatype needs to be accessible from MOO code:

1. Add built-in functions for creating and manipulating your type
2. Update the parser to recognize literal syntax for your type (if applicable): `crates/compiler/src/{moo.pest[, ast.rs]} for ast nodes if a non-atomic type; {unparse.rs, decompile.rs} for vm opcodes, `parse.rs` if you need to add a new compiler option.
3. Update the `bf_toliteral` function in `crates/kernel/src/builtins/bf_values.rs` so `toliterall(value_of_your_type)` in MOO returns something meaningful. Its probably best if this is exactly as your literal representation looks if you can manage it.
4. Implement any special verbs or properties needed for your type
5. 

## Example: Adding a "Duration" Type

As a concrete example, to add a Duration type that represents time spans:

1. Create `crates/var/src/duration.rs` with the Duration struct and methods
2. Implement comparison, arithmetic, and conversion methods
3. Add Duration to the Variant enum
4. Implement serialization/deserialization
5. Add TypeScript support
6. Add built-in functions like `duration()`, `duration_seconds()`, etc.

## Troubleshooting

Common issues when adding new datatypes:

- Forgetting to update all serialization/deserialization code paths
- Missing trait implementations
- Not handling equality and comparison correctly
- Forgetting to update TypeScript definitions

When in doubt, look at how existing datatypes like List or Map are implemented and follow similar patterns.

