use alloc::{collections::BTreeMap, format, string::String};

use clinkz_wot_core::{CoreError, CoreResult};

pub(super) fn selector_with_parameters(
    key_expr: &str,
    parameters: &BTreeMap<String, String>,
) -> CoreResult<String> {
    if parameters.is_empty() {
        return Ok(key_expr.into());
    }

    let mut selector = String::with_capacity(key_expr.len() + encoded_parameters_len(parameters));
    selector.push_str(key_expr);
    if let Some(separator) = selector_parameter_separator(key_expr)? {
        selector.push(separator);
    }

    let mut first = true;
    for (key, value) in parameters {
        validate_selector_parameter("key", key)?;
        validate_selector_parameter("value", value)?;

        if !first {
            selector.push(';');
        }
        selector.push_str(key);
        if !value.is_empty() {
            selector.push('=');
            selector.push_str(value);
        }
        first = false;
    }

    Ok(selector)
}

fn selector_parameter_separator(key_expr: &str) -> CoreResult<Option<char>> {
    let mut parameter_separator_count = 0;
    for char_ in key_expr.chars() {
        if char_ == '?' {
            parameter_separator_count += 1;
        }
    }

    match parameter_separator_count {
        0 => Ok(Some('?')),
        1 if key_expr.ends_with(['?', ';']) => Ok(None),
        1 => Ok(Some(';')),
        _ => Err(CoreError::Transport(format!(
            "Zenoh selector '{}' contains multiple parameter separators",
            key_expr
        ))),
    }
}

fn encoded_parameters_len(parameters: &BTreeMap<String, String>) -> usize {
    let mut len = 1;
    for (key, value) in parameters {
        len += key.len() + 1;
        if !value.is_empty() {
            len += value.len() + 1;
        }
    }
    len
}

fn validate_selector_parameter(kind: &str, value: &str) -> CoreResult<()> {
    if kind == "key" && value.trim().is_empty() {
        return Err(CoreError::Transport(
            "Zenoh selector parameter key must not be empty".into(),
        ));
    }

    if value.contains(['?', ';', '=', '|']) {
        return Err(CoreError::Transport(format!(
            "Zenoh selector parameter {} '{}' contains a reserved separator",
            kind, value
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_selector_with_request_parameters() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply".into(), "full".into());
        parameters.insert("trace".into(), String::new());

        let selector =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap();

        assert_eq!(
            selector,
            "clinkz/things/lamp/actions/reboot?reply=full;trace"
        );
    }

    #[test]
    fn appends_request_parameters_to_existing_selector_parameters() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let selector = selector_with_parameters(
            "clinkz/things/lamp/actions/reboot?reply=summary",
            &parameters,
        )
        .unwrap();

        assert_eq!(
            selector,
            "clinkz/things/lamp/actions/reboot?reply=summary;trace=true"
        );
    }

    #[test]
    fn appends_request_parameters_to_open_selector_parameter_list() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let selector =
            selector_with_parameters("clinkz/things/lamp/actions/reboot?", &parameters).unwrap();

        assert_eq!(selector, "clinkz/things/lamp/actions/reboot?trace=true");
    }

    #[test]
    fn appends_request_parameters_to_selector_with_trailing_parameter_separator() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let selector = selector_with_parameters(
            "clinkz/things/lamp/actions/reboot?reply=summary;",
            &parameters,
        )
        .unwrap();

        assert_eq!(
            selector,
            "clinkz/things/lamp/actions/reboot?reply=summary;trace=true"
        );
    }

    #[test]
    fn rejects_selectors_with_multiple_parameter_separators() {
        let mut parameters = BTreeMap::new();
        parameters.insert("trace".into(), "true".into());

        let err = selector_with_parameters(
            "clinkz/things/lamp/actions/reboot?reply=summary?trace=false",
            &parameters,
        )
        .unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector 'clinkz/things/lamp/actions/reboot?reply=summary?trace=false' contains multiple parameter separators".into()
            )
        );
    }

    #[test]
    fn returns_plain_selector_without_request_parameters() {
        let selector =
            selector_with_parameters("clinkz/things/lamp/properties/status", &BTreeMap::new())
                .unwrap();

        assert_eq!(selector, "clinkz/things/lamp/properties/status");
    }

    #[test]
    fn rejects_ambiguous_selector_parameter_keys() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply;mode".into(), "full".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector parameter key 'reply;mode' contains a reserved separator".into()
            )
        );
    }

    #[test]
    fn rejects_empty_selector_parameter_keys() {
        let mut parameters = BTreeMap::new();
        parameters.insert(String::new(), "full".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport("Zenoh selector parameter key must not be empty".into())
        );
    }

    #[test]
    fn rejects_blank_selector_parameter_keys() {
        let mut parameters = BTreeMap::new();
        parameters.insert("  ".into(), "full".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport("Zenoh selector parameter key must not be empty".into())
        );
    }

    #[test]
    fn rejects_ambiguous_selector_parameter_values() {
        let mut parameters = BTreeMap::new();
        parameters.insert("reply".into(), "full;trace=true".into());

        let err =
            selector_with_parameters("clinkz/things/lamp/actions/reboot", &parameters).unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Zenoh selector parameter value 'full;trace=true' contains a reserved separator"
                    .into()
            )
        );
    }
}
