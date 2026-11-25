//! A YAML reader that refuses a key repeated in any mapping.
//!
//! `serde_yaml_ng` keeps the last of two equal keys without a word, so a
//! template could carry a value its author never sees. This reader builds the
//! JSON value itself and fails on the second occurrence, with the path and the
//! line of the key.

use std::fmt;

use plinth_application::TemplateError;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

/// Reads `yaml` into a JSON value; a key written twice in the same mapping is
/// an error, wherever the mapping is.
pub fn from_str(yaml: &str) -> Result<Value, TemplateError> {
    serde_yaml_ng::from_str::<Strict>(yaml)
        .map(|Strict(value)| value)
        .map_err(|e| TemplateError(format!("invalid YAML: {e}")))
}

/// A JSON value read under the no-duplicate-keys rule.
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Strict;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a YAML value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Strict, E> {
        Ok(Strict(Value::Bool(v)))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Strict, E> {
        Ok(Strict(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Strict, E> {
        Ok(Strict(v.into()))
    }

    /// `.inf` and `.nan` have no JSON number: they become `null`, which the
    /// schema then refuses where a number is expected.
    fn visit_f64<E>(self, v: f64) -> Result<Strict, E> {
        Ok(Strict(
            Number::from_f64(v).map_or(Value::Null, Value::Number),
        ))
    }

    fn visit_str<E>(self, v: &str) -> Result<Strict, E> {
        Ok(Strict(Value::String(v.to_owned())))
    }

    fn visit_string<E>(self, v: String) -> Result<Strict, E> {
        Ok(Strict(Value::String(v)))
    }

    fn visit_unit<E>(self) -> Result<Strict, E> {
        Ok(Strict(Value::Null))
    }

    fn visit_none<E>(self) -> Result<Strict, E> {
        Ok(Strict(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Strict(Value::Array(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
        let mut entries = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let Strict(value) = map.next_value()?;
            if entries.insert(key.clone(), value).is_some() {
                return Err(de::Error::custom(format!("duplicate key `{key}`")));
            }
        }
        Ok(Strict(Value::Object(entries)))
    }
}
