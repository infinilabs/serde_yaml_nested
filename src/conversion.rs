//! This module provides [`flatten()`] and [`unflatten()`] to do the conversions
//! between nested and flattened YAML values.

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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_yaml_ng::from_str;
    use serde_yaml_ng::Number;
    use serde_yaml_ng::Value;

    #[test]
    fn one_layer() {
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
    fn two_layers() {
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
    fn three_layers() {
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
    fn partially_flattened() {
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
    fn totally_flattened() {
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
}
