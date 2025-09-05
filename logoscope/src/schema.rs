use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("not a JSON object line")]
    NotJson,
    #[error("json parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub type Fingerprint = BTreeMap<String, String>; // field_path -> type

pub fn fingerprint_line(line: &str) -> Result<Fingerprint, SchemaError> {
    let v: Value = serde_json::from_str(line)?;
    match v {
        Value::Object(_) => Ok(fingerprint_value(&v)),
        _ => Err(SchemaError::NotJson),
    }
}

pub fn fingerprint_value(v: &Value) -> Fingerprint {
    let mut out = BTreeMap::new();
    flatten_types("", v, &mut out);
    out
}

fn flatten_types(prefix: &str, v: &Value, out: &mut Fingerprint) {
    match v {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_types(&key, v, out);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                let key = if prefix.is_empty() {
                    idx.to_string()
                } else {
                    format!("{}.{}", prefix, idx)
                };
                flatten_types(&key, item, out);
            }
        }
        _ => {
            out.insert(prefix.to_string(), type_of(v).to_string());
        }
    }
}

fn type_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "float"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaChange {
    FieldAdded { field: String, new_type: String },
    FieldRemoved { field: String, old_type: String },
    TypeChanged { field: String, from_type: String, to_type: String },
}

pub fn diff_fingerprints(before: &Fingerprint, after: &Fingerprint) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    // Detect removed and type-changed
    for (field, b_type) in before.iter() {
        match after.get(field) {
            None => changes.push(SchemaChange::FieldRemoved { field: field.clone(), old_type: b_type.clone() }),
            Some(a_type) if a_type != b_type => changes.push(SchemaChange::TypeChanged {
                field: field.clone(),
                from_type: b_type.clone(),
                to_type: a_type.clone(),
            }),
            _ => {}
        }
    }

    // Detect added
    for (field, a_type) in after.iter() {
        if !before.contains_key(field) {
            changes.push(SchemaChange::FieldAdded { field: field.clone(), new_type: a_type.clone() });
        }
    }

    changes
}

