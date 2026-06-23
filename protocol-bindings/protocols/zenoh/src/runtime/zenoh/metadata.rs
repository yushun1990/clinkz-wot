use alloc::format;

use clinkz_wot_core::{CoreError, CoreResult};
use zenoh::qos::{CongestionControl, Priority};

/// Returns true when `value` (after trimming whitespace) case-insensitively
/// equals any of `candidates`. Avoids the per-call `String` allocation that the
/// previous `to_ascii_lowercase` implementation paid on every QoS-bearing form.
fn matches_any_ci(value: &str, candidates: &[&str]) -> bool {
    let trimmed = value.trim();
    candidates
        .iter()
        .any(|candidate| trimmed.eq_ignore_ascii_case(candidate))
}

pub(super) fn parse_express_qos(value: &str) -> CoreResult<bool> {
    if matches_any_ci(value, &["express", "true", "yes", "1"]) {
        Ok(true)
    } else if matches_any_ci(value, &["normal", "default", "false", "no", "0"]) {
        Ok(false)
    } else {
        Err(unsupported_metadata("cz-zenoh:qos", value))
    }
}

pub(super) fn parse_priority(value: &str) -> CoreResult<Priority> {
    if matches_any_ci(value, &["real-time", "realtime", "real_time"]) {
        Ok(Priority::RealTime)
    } else if matches_any_ci(
        value,
        &["interactive-high", "interactivehigh", "interactive_high"],
    ) {
        Ok(Priority::InteractiveHigh)
    } else if matches_any_ci(
        value,
        &["interactive-low", "interactivelow", "interactive_low"],
    ) {
        Ok(Priority::InteractiveLow)
    } else if matches_any_ci(value, &["data-high", "datahigh", "data_high"]) {
        Ok(Priority::DataHigh)
    } else if matches_any_ci(value, &["data", "default"]) {
        Ok(Priority::Data)
    } else if matches_any_ci(value, &["data-low", "datalow", "data_low"]) {
        Ok(Priority::DataLow)
    } else if matches_any_ci(value, &["background"]) {
        Ok(Priority::Background)
    } else {
        Err(unsupported_metadata("cz-zenoh:priority", value))
    }
}

pub(super) fn parse_congestion_control(value: &str) -> CoreResult<CongestionControl> {
    if matches_any_ci(value, &["drop"]) {
        Ok(CongestionControl::Drop)
    } else if matches_any_ci(value, &["block"]) {
        Ok(CongestionControl::Block)
    } else {
        Err(unsupported_metadata("cz-zenoh:congestionControl", value))
    }
}

fn unsupported_metadata(term: &str, value: &str) -> CoreError {
    CoreError::Transport(format!(
        "Unsupported zenoh metadata {} value '{}'",
        term, value
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_express_qos_metadata() {
        assert!(parse_express_qos("express").unwrap());
        assert!(parse_express_qos("true").unwrap());
        assert!(!parse_express_qos("normal").unwrap());
        assert!(!parse_express_qos("default").unwrap());
    }

    #[test]
    fn rejects_unknown_qos_metadata() {
        let err = parse_express_qos("guaranteed").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:qos value 'guaranteed'".into()
            )
        );
    }

    #[test]
    fn parses_priority_metadata() {
        assert_eq!(parse_priority("real-time").unwrap(), Priority::RealTime);
        assert_eq!(
            parse_priority("interactive-high").unwrap(),
            Priority::InteractiveHigh
        );
        assert_eq!(parse_priority("data").unwrap(), Priority::Data);
        assert_eq!(parse_priority("background").unwrap(), Priority::Background);
    }

    #[test]
    fn parses_congestion_control_metadata() {
        assert_eq!(
            parse_congestion_control("drop").unwrap(),
            CongestionControl::Drop
        );
        assert_eq!(
            parse_congestion_control("block").unwrap(),
            CongestionControl::Block
        );
    }

    #[test]
    fn parses_metadata_case_and_whitespace_variants() {
        assert!(parse_express_qos(" YES ").unwrap());
        assert!(!parse_express_qos(" No ").unwrap());
        assert_eq!(parse_priority("DATA_HIGH").unwrap(), Priority::DataHigh);
        assert_eq!(parse_priority("data-low").unwrap(), Priority::DataLow);
        assert_eq!(
            parse_congestion_control(" BLOCK ").unwrap(),
            CongestionControl::Block
        );
    }

    #[test]
    fn rejects_unknown_priority_metadata() {
        let err = parse_priority("urgent").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:priority value 'urgent'".into()
            )
        );
    }

    #[test]
    fn rejects_unknown_congestion_control_metadata() {
        let err = parse_congestion_control("queue").unwrap_err();

        assert_eq!(
            err,
            CoreError::Transport(
                "Unsupported zenoh metadata cz-zenoh:congestionControl value 'queue'".into()
            )
        );
    }
}
