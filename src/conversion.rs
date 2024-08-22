//! This module provides [`flatten()`] and [`unflatten()`] to do the conversions
//! between nested and flattened YAML values.

use serde_yaml_ng::Mapping;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

const DOT: &str = ".";

/// Flattens the `input` YAML value.
///
/// # Examples
///
/// ```rust
/// # use serde_yaml_ng::from_str;
/// # use serde_yaml_ng::Value;
/// # use serde_yaml_nested::conversion::flatten;
/// # use std::collections::BTreeMap;
/// let nested: Value = from_str(
///     r#"
/// a:  
///   b:
///     c: null
/// "#,
/// )
/// .unwrap();
///
/// let flattened = flatten(nested);
/// assert_eq!(
///     flattened,
///     BTreeMap::from([(String::from("a.b.c"), Value::Null)])
/// );
/// ```
pub fn flatten(input: Value) -> BTreeMap<String, Value> {
    let mut output = BTreeMap::new();
    let mut path = Vec::new();
    _flatten(&mut output, &mut path, input);

    output
}

/// Inner helper function to do the recursive flatten job.
fn _flatten(output: &mut BTreeMap<String, Value>, path: &mut Vec<String>, input: Value) {
    match input {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            if !path.is_empty() {
                let full_path = path.join(DOT);
                output.insert(full_path, input);
            }
        }

        Value::Tagged(_) => unimplemented!(),

        Value::Sequence(_) => {
            // Let's do not flatten array for now.
            if !path.is_empty() {
                let full_path = path.join(DOT);
                output.insert(full_path, input);
            }
        }

        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let key = match key {
                    Value::Null => unreachable!("a mapping key cannot be NULL"),
                    Value::Bool(boolean) => boolean.to_string(),
                    Value::Number(number) => number.to_string(),
                    Value::String(string) => string,

                    non_literal => {
                        unreachable!("a mapping key should be listeral, found: {:?}", non_literal)
                    }
                };
                path.push(key);

                _flatten(output, path, value);

                path.pop();
            }
        }
    }
}

/// The errors that may happen during conversion.
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    DuplicateValue { key: String, token: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateValue { key, token } => {
                write!(
                    f,
                    "while handling key '{}', found a token '{}' that has at least 2 values",
                    key, token
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// Unflattens the given `input` YAML.
///
/// # Examples
///
/// ```rust
/// # use serde_yaml_ng::from_str;
/// # use serde_yaml_ng::Value;
/// # use serde_yaml_nested::conversion::unflatten;
/// let flattened: Value = from_str(r#"a.b.c: null"#).unwrap();
/// let mapping = match flattened {
///     Value::Mapping(mapping) => mapping,
///     _ => unreachable!(),
/// };
///
/// let nested = unflatten(
///     mapping
///         .into_iter()
///         .map(|(key, value)| (key.as_str().unwrap().to_string(), value)),
/// )
/// .unwrap();
///
/// let expected: Value = from_str(
///     r#"
/// a:
///   b:
///     c: null"#,
/// )
/// .unwrap();
/// assert_eq!(nested, expected);
/// ```
pub fn unflatten<I: IntoIterator<Item = (String, Value)>>(input: I) -> Result<Value, Error> {
    let mut mapping = Mapping::new();
    for (key, value) in input {
        let mut split_by_dot = key.split(DOT).peekable();

        let mut outermost_mapping = &mut mapping;
        'inner: loop {
            let token_str = split_by_dot
                .next()
                .expect("should be Some, guarded by last iteration");
            let token = Value::String(token_str.into());

            let key_is_last_key = split_by_dot.peek().is_none();

            // We use `.get(&self)` to acquire if this key exists or not
            // cannot use `.get_mut(&mut self)` as that will borrow
            // `outermost_mapping` for more than once.
            let exist = outermost_mapping.get(&token).is_some();

            if exist {
                let existing = outermost_mapping
                    .get_mut(&token)
                    .expect("should be Some as `exist` is true");
                if key_is_last_key {
                    return Err(Error::DuplicateValue {
                        key: key.clone(),
                        token: token_str.to_string(),
                    });
                } else if let Value::Mapping(new_mapping) = existing {
                    outermost_mapping = new_mapping;
                } else {
                    return Err(Error::DuplicateValue {
                        key: key.clone(),
                        token: token_str.to_string(),
                    });
                }
            } else if key_is_last_key {
                outermost_mapping.insert(token, value);
                break 'inner;
            } else {
                outermost_mapping.insert(token.clone(), Value::Mapping(Mapping::new()));
                let newly_inserted_mapping = outermost_mapping
                    .get_mut(&token)
                    .unwrap()
                    .as_mapping_mut()
                    .unwrap();
                outermost_mapping = newly_inserted_mapping;
            }
        }
    }

    Ok(Value::Mapping(mapping))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_yaml_ng::from_str;
    use serde_yaml_ng::Number;
    use serde_yaml_ng::Value;

    #[test]
    fn test_flatten_one_layer() {
        let bool_null = "true: null";
        let yaml = from_str::<Value>(&bool_null).unwrap();
        let flattened = flatten(yaml);
        assert_eq!(
            flattened,
            BTreeMap::from([(String::from("true"), Value::Null)])
        );

        let bool_bool = "true: true";
        let yaml = from_str::<Value>(&bool_bool).unwrap();
        let flattened = flatten(yaml);
        assert_eq!(
            flattened,
            BTreeMap::from([(String::from("true"), Value::Bool(true))])
        );

        let bool_number = "true: 1";
        let yaml = from_str::<Value>(&bool_number).unwrap();
        let flattened = flatten(yaml);
        assert_eq!(
            flattened,
            BTreeMap::from([(String::from("true"), Value::Number(Number::from(1)))])
        );

        let bool_str = "true: str";
        let yaml = from_str::<Value>(&bool_str).unwrap();
        let flattened = flatten(yaml);
        assert_eq!(
            flattened,
            BTreeMap::from([(String::from("true"), Value::String("str".into()))])
        );

        let yaml_str = r#"
1: null 
2: true
3: 1
4: hello

str1: null
str2: true
str3: 1
str4: hello
    "#;

        let yaml = from_str::<Value>(&yaml_str).unwrap();
        let flattened = flatten(yaml);

        let expected = BTreeMap::from([
            (String::from("1"), Value::Null),
            (String::from("2"), Value::Bool(true)),
            (String::from("3"), Value::Number(Number::from(1))),
            (String::from("4"), Value::String("hello".into())),
            (String::from("str1"), Value::Null),
            (String::from("str2"), Value::Bool(true)),
            (String::from("str3"), Value::Number(Number::from(1))),
            (String::from("str4"), Value::String("hello".into())),
        ]);
        assert_eq!(flattened, expected);
    }

    #[test]
    fn teset_flatten_two_layers() {
        let yaml_str = r#"
true:
  true: true
  false: false

  1: null
  2: true
  3: 1
  4: hello

  str1: null
  str2: true
  str3: 1
  str4: hello
1:
  true: true
  false: false

  1: null
  2: true
  3: 1
  4: hello

  str1: null
  str2: true
  str3: 1
  str4: hello

str:
  true: true
  false: false

  1: null
  2: true
  3: 1
  4: hello

  str1: null
  str2: true
  str3: 1
  str4: hello
"#;

        let yaml = from_str::<Value>(&yaml_str).unwrap();

        let flattened = flatten(yaml);

        let expected = BTreeMap::from([
            (String::from("true.true"), Value::Bool(true)),
            (String::from("true.false"), Value::Bool(false)),
            (String::from("true.1"), Value::Null),
            (String::from("true.2"), Value::Bool(true)),
            (String::from("true.3"), Value::Number(Number::from(1))),
            (String::from("true.4"), Value::String("hello".into())),
            (String::from("true.str1"), Value::Null),
            (String::from("true.str2"), Value::Bool(true)),
            (String::from("true.str3"), Value::Number(Number::from(1))),
            (String::from("true.str4"), Value::String("hello".into())),
            (String::from("1.true"), Value::Bool(true)),
            (String::from("1.false"), Value::Bool(false)),
            (String::from("1.1"), Value::Null),
            (String::from("1.2"), Value::Bool(true)),
            (String::from("1.3"), Value::Number(Number::from(1))),
            (String::from("1.4"), Value::String("hello".into())),
            (String::from("1.str1"), Value::Null),
            (String::from("1.str2"), Value::Bool(true)),
            (String::from("1.str3"), Value::Number(Number::from(1))),
            (String::from("1.str4"), Value::String("hello".into())),
            (String::from("str.true"), Value::Bool(true)),
            (String::from("str.false"), Value::Bool(false)),
            (String::from("str.1"), Value::Null),
            (String::from("str.2"), Value::Bool(true)),
            (String::from("str.3"), Value::Number(Number::from(1))),
            (String::from("str.4"), Value::String("hello".into())),
            (String::from("str.str1"), Value::Null),
            (String::from("str.str2"), Value::Bool(true)),
            (String::from("str.str3"), Value::Number(Number::from(1))),
            (String::from("str.str4"), Value::String("hello".into())),
        ]);

        assert_eq!(flattened, expected);
    }

    #[test]
    fn test_flatten_three_layers() {
        let yaml_str = r#"
true:
  true:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  1:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  
  str:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello

1:
  true:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  1:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  
  str:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
str:
  true:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  1:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
  
  str:
    true: true
    false: false
  
    1: null
    2: true
    3: 1
    4: hello
  
    str1: null
    str2: true
    str3: 1
    str4: hello
"#;

        let yaml = from_str::<Value>(&yaml_str).unwrap();
        let flattened = flatten(yaml);

        let expected = BTreeMap::from([
            (String::from("true.true.true"), Value::Bool(true)),
            (String::from("true.true.false"), Value::Bool(false)),
            (String::from("true.true.1"), Value::Null),
            (String::from("true.true.2"), Value::Bool(true)),
            (String::from("true.true.3"), Value::Number(Number::from(1))),
            (String::from("true.true.4"), Value::String("hello".into())),
            (String::from("true.true.str1"), Value::Null),
            (String::from("true.true.str2"), Value::Bool(true)),
            (
                String::from("true.true.str3"),
                Value::Number(Number::from(1)),
            ),
            (
                String::from("true.true.str4"),
                Value::String("hello".into()),
            ),
            (String::from("true.1.true"), Value::Bool(true)),
            (String::from("true.1.false"), Value::Bool(false)),
            (String::from("true.1.1"), Value::Null),
            (String::from("true.1.2"), Value::Bool(true)),
            (String::from("true.1.3"), Value::Number(Number::from(1))),
            (String::from("true.1.4"), Value::String("hello".into())),
            (String::from("true.1.str1"), Value::Null),
            (String::from("true.1.str2"), Value::Bool(true)),
            (String::from("true.1.str3"), Value::Number(Number::from(1))),
            (String::from("true.1.str4"), Value::String("hello".into())),
            (String::from("true.str.true"), Value::Bool(true)),
            (String::from("true.str.false"), Value::Bool(false)),
            (String::from("true.str.1"), Value::Null),
            (String::from("true.str.2"), Value::Bool(true)),
            (String::from("true.str.3"), Value::Number(Number::from(1))),
            (String::from("true.str.4"), Value::String("hello".into())),
            (String::from("true.str.str1"), Value::Null),
            (String::from("true.str.str2"), Value::Bool(true)),
            (
                String::from("true.str.str3"),
                Value::Number(Number::from(1)),
            ),
            (String::from("true.str.str4"), Value::String("hello".into())),
            (String::from("1.true.true"), Value::Bool(true)),
            (String::from("1.true.false"), Value::Bool(false)),
            (String::from("1.true.1"), Value::Null),
            (String::from("1.true.2"), Value::Bool(true)),
            (String::from("1.true.3"), Value::Number(Number::from(1))),
            (String::from("1.true.4"), Value::String("hello".into())),
            (String::from("1.true.str1"), Value::Null),
            (String::from("1.true.str2"), Value::Bool(true)),
            (String::from("1.true.str3"), Value::Number(Number::from(1))),
            (String::from("1.true.str4"), Value::String("hello".into())),
            (String::from("1.1.true"), Value::Bool(true)),
            (String::from("1.1.false"), Value::Bool(false)),
            (String::from("1.1.1"), Value::Null),
            (String::from("1.1.2"), Value::Bool(true)),
            (String::from("1.1.3"), Value::Number(Number::from(1))),
            (String::from("1.1.4"), Value::String("hello".into())),
            (String::from("1.1.str1"), Value::Null),
            (String::from("1.1.str2"), Value::Bool(true)),
            (String::from("1.1.str3"), Value::Number(Number::from(1))),
            (String::from("1.1.str4"), Value::String("hello".into())),
            (String::from("1.str.true"), Value::Bool(true)),
            (String::from("1.str.false"), Value::Bool(false)),
            (String::from("1.str.1"), Value::Null),
            (String::from("1.str.2"), Value::Bool(true)),
            (String::from("1.str.3"), Value::Number(Number::from(1))),
            (String::from("1.str.4"), Value::String("hello".into())),
            (String::from("1.str.str1"), Value::Null),
            (String::from("1.str.str2"), Value::Bool(true)),
            (String::from("1.str.str3"), Value::Number(Number::from(1))),
            (String::from("1.str.str4"), Value::String("hello".into())),
            (String::from("str.true.true"), Value::Bool(true)),
            (String::from("str.true.false"), Value::Bool(false)),
            (String::from("str.true.1"), Value::Null),
            (String::from("str.true.2"), Value::Bool(true)),
            (String::from("str.true.3"), Value::Number(Number::from(1))),
            (String::from("str.true.4"), Value::String("hello".into())),
            (String::from("str.true.str1"), Value::Null),
            (String::from("str.true.str2"), Value::Bool(true)),
            (
                String::from("str.true.str3"),
                Value::Number(Number::from(1)),
            ),
            (String::from("str.true.str4"), Value::String("hello".into())),
            (String::from("str.1.true"), Value::Bool(true)),
            (String::from("str.1.false"), Value::Bool(false)),
            (String::from("str.1.1"), Value::Null),
            (String::from("str.1.2"), Value::Bool(true)),
            (String::from("str.1.3"), Value::Number(Number::from(1))),
            (String::from("str.1.4"), Value::String("hello".into())),
            (String::from("str.1.str1"), Value::Null),
            (String::from("str.1.str2"), Value::Bool(true)),
            (String::from("str.1.str3"), Value::Number(Number::from(1))),
            (String::from("str.1.str4"), Value::String("hello".into())),
            (String::from("str.str.true"), Value::Bool(true)),
            (String::from("str.str.false"), Value::Bool(false)),
            (String::from("str.str.1"), Value::Null),
            (String::from("str.str.2"), Value::Bool(true)),
            (String::from("str.str.3"), Value::Number(Number::from(1))),
            (String::from("str.str.4"), Value::String("hello".into())),
            (String::from("str.str.str1"), Value::Null),
            (String::from("str.str.str2"), Value::Bool(true)),
            (String::from("str.str.str3"), Value::Number(Number::from(1))),
            (String::from("str.str.str4"), Value::String("hello".into())),
        ]);
        assert_eq!(flattened, expected);
    }

    #[test]
    fn test_flatten_partially_flattened() {
        let yaml_str = r#"
cluster.fault_detection:
  follower_check:
    interval: 1000
    retry: 3
  master_check:
    interval: 500
    retry: 9
routing.allocation.same_shard.host: false"#;
        let yaml: Value = from_str(&yaml_str).unwrap();
        let flattened = flatten(yaml);
        let expected = BTreeMap::from([
            (
                String::from("cluster.fault_detection.follower_check.interval"),
                Value::Number(Number::from(1000)),
            ),
            (
                String::from("cluster.fault_detection.follower_check.retry"),
                Value::Number(Number::from(3)),
            ),
            (
                String::from("cluster.fault_detection.master_check.interval"),
                Value::Number(Number::from(500)),
            ),
            (
                String::from("cluster.fault_detection.master_check.retry"),
                Value::Number(Number::from(9)),
            ),
            (
                String::from("routing.allocation.same_shard.host"),
                Value::Bool(false),
            ),
        ]);

        assert_eq!(flattened, expected);
    }

    #[test]
    fn test_flatten_totally_flattened() {
        let yaml_str = r#"
action.auto_create_index: true
action.destructive_requires_name: true
action.search.pre_filter_shard_size.default: 128
action.search.shard_count.limit: 9223372036854775807
async_search.index_cleanup_interval: 1h
bootstrap.ctrlhandler: true
bootstrap.memory_lock: false
cache.recycler.page.limit.heap: 10%
cache.recycler.page.type: CONCURRENT
cache.recycler.page.weight.bytes: 1.0"#;
        let yaml: Value = from_str(&yaml_str).unwrap();
        let flattened = flatten(yaml);

        let expected = BTreeMap::from([
            (String::from("action.auto_create_index"), Value::Bool(true)),
            (
                String::from("action.destructive_requires_name"),
                Value::Bool(true),
            ),
            (
                String::from("action.search.pre_filter_shard_size.default"),
                Value::Number(128.into()),
            ),
            (
                String::from("action.search.shard_count.limit"),
                Value::Number(Number::from(9223372036854775807_u64)),
            ),
            (
                String::from("async_search.index_cleanup_interval"),
                Value::String("1h".into()),
            ),
            (String::from("bootstrap.ctrlhandler"), Value::Bool(true)),
            (String::from("bootstrap.memory_lock"), Value::Bool(false)),
            (
                String::from("cache.recycler.page.limit.heap"),
                Value::String("10%".into()),
            ),
            (
                String::from("cache.recycler.page.type"),
                Value::String("CONCURRENT".into()),
            ),
            (
                String::from("cache.recycler.page.weight.bytes"),
                Value::Number(Number::from(1.0)),
            ),
        ]);

        assert_eq!(flattened, expected);
    }

    #[test]
    fn test_unflatten_one_layer() {
        let nested = unflatten([
            ("a".into(), Value::Null),
            ("b".into(), Value::Bool(false)),
            ("c".into(), Value::Number(Number::from(1))),
            ("d".into(), Value::String("hello".into())),
        ])
        .unwrap();
        let expected_mapping: Mapping = [
            (Value::String("a".into()), Value::Null),
            (Value::String("b".into()), Value::Bool(false)),
            (Value::String("c".into()), Value::Number(Number::from(1))),
            (Value::String("d".into()), Value::String("hello".into())),
        ]
        .into_iter()
        .collect();

        let expected = Value::Mapping(expected_mapping);
        assert_eq!(expected, nested);
    }

    #[test]
    fn test_unflatten_two_layers() {
        let nested = unflatten([
            ("a.a".into(), Value::Null),
            ("a.b".into(), Value::Bool(false)),
            ("a.c".into(), Value::Number(Number::from(1))),
            ("a.d".into(), Value::String("hello".into())),
        ])
        .unwrap();

        let inner_mapping: Mapping = [
            (Value::String("a".into()), Value::Null),
            (Value::String("b".into()), Value::Bool(false)),
            (Value::String("c".into()), Value::Number(Number::from(1))),
            (Value::String("d".into()), Value::String("hello".into())),
        ]
        .into_iter()
        .collect();

        let mut expected_mapping = Mapping::new();
        expected_mapping.insert(Value::String("a".into()), Value::Mapping(inner_mapping));

        let expected = Value::Mapping(expected_mapping);
        assert_eq!(expected, nested);
    }

    #[test]
    fn test_unflatten_three_layers() {
        let nested = unflatten([
            ("a.a.a".into(), Value::Null),
            ("a.a.b".into(), Value::Bool(false)),
            ("a.a.c".into(), Value::Number(Number::from(1))),
            ("a.a.d".into(), Value::String("hello".into())),
        ])
        .unwrap();

        let innermost_mapping: Mapping = [
            (Value::String("a".into()), Value::Null),
            (Value::String("b".into()), Value::Bool(false)),
            (Value::String("c".into()), Value::Number(Number::from(1))),
            (Value::String("d".into()), Value::String("hello".into())),
        ]
        .into_iter()
        .collect();

        let mut middle_mapping = Mapping::new();
        middle_mapping.insert(Value::String("a".into()), Value::Mapping(innermost_mapping));

        let mut expected_mapping = Mapping::new();
        expected_mapping.insert(Value::String("a".into()), Value::Mapping(middle_mapping));

        let expected = Value::Mapping(expected_mapping);
        assert_eq!(expected, nested);
    }

    #[test]
    fn test_unflatten_duplicate_value() {
        let error =
            unflatten([("a".into(), Value::Null), ("a".into(), Value::Bool(false))]).unwrap_err();
        assert_eq!(
            error,
            Error::DuplicateValue {
                key: "a".into(),
                token: "a".into()
            }
        );

        let error = unflatten([
            ("a.b".into(), Value::Null),
            ("a.b.c".into(), Value::Bool(false)),
        ])
        .unwrap_err();
        assert_eq!(
            error,
            Error::DuplicateValue {
                key: "a.b.c".into(),
                token: "b".into()
            }
        );

        let error = unflatten([
            ("a.b.c".into(), Value::Null),
            ("a.b".into(), Value::Bool(false)),
        ])
        .unwrap_err();
        assert_eq!(
            error,
            Error::DuplicateValue {
                key: "a.b".into(),
                token: "b".into()
            }
        );
    }
}
