//! recursively walk a `YamlValue` and find the strings to apply variable interpolation to
use serde_yaml::{Mapping, Sequence};

use super::{parser::Parser, VariableResolver};
use crate::common::YamlValue;

/// recursively interpolate all nested YAML strings containing variables.
/// will return the first failure to replace a variable.
pub(crate) fn interpolate_value(
    vars: &VariableResolver,
    value: &mut YamlValue,
) -> serde_yaml::Result<()> {
    match value {
        YamlValue::String(s) => {
            let new_s = Parser::start(vars, s)?;
            let _ = std::mem::replace(value, YamlValue::String(new_s));
        }
        YamlValue::Sequence(sequence) => interpolate_sequence(vars, sequence)?,
        YamlValue::Mapping(mapping) => interpolate_mapping(vars, mapping)?,
        _ => {}
    };
    Ok(())
}

/// try to interpolate variables in each of the elements of the given sequence.
fn interpolate_sequence(
    vars: &VariableResolver,
    sequence: &mut Sequence,
) -> serde_yaml::Result<()> {
    for element in sequence.iter_mut() {
        interpolate_value(vars, element)?;
    }
    Ok(())
}

/// try to interpolate variables in each of the values of the mapping. keys are never interpolated.
fn interpolate_mapping(vars: &VariableResolver, mapping: &mut Mapping) -> serde_yaml::Result<()> {
    for (_key, value) in mapping.iter_mut() {
        interpolate_value(vars, value)?;
    }
    Ok(())
}
