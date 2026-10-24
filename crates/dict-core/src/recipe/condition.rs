use std::collections::HashMap;

use serde::Deserialize;

use super::contains_spec::ContainsSpec;
use super::value_ref::ValueRef;

/// 策略生效条件。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    NonEmpty { non_empty: String },
    Empty { empty: String },
    Contains { contains: ContainsSpec },
}

impl Condition {
    pub fn matches<'a>(&self, vars: &HashMap<String, Vec<ValueRef<'a>>>) -> bool {
        match self {
            Self::NonEmpty { non_empty } => vars
                .get(non_empty)
                .is_some_and(|values| values.iter().any(|value| !value.as_string().is_empty())),
            Self::Empty { empty } => vars
                .get(empty)
                .is_none_or(|values| values.iter().all(|value| value.as_string().is_empty())),
            Self::Contains { contains } => vars.get(&contains.var).is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.as_string().contains(&contains.value))
            }),
        }
    }
}
