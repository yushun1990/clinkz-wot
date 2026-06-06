use alloc::{format, string::String};

use clinkz_wot_core::{CoreError, CoreResult};
use zenoh::qos::{CongestionControl, Priority};

pub(super) fn parse_express_qos(value: &str) -> CoreResult<bool> {
    match normalized_metadata_value(value).as_str() {
        "express" | "true" | "yes" | "1" => Ok(true),
        "normal" | "default" | "false" | "no" | "0" => Ok(false),
        _ => Err(unsupported_metadata("cz-zenoh:qos", value)),
    }
}

pub(super) fn parse_priority(value: &str) -> CoreResult<Priority> {
    match normalized_metadata_value(value).as_str() {
        "real-time" | "realtime" | "real_time" => Ok(Priority::RealTime),
        "interactive-high" | "interactivehigh" | "interactive_high" => {
            Ok(Priority::InteractiveHigh)
        }
        "interactive-low" | "interactivelow" | "interactive_low" => Ok(Priority::InteractiveLow),
        "data-high" | "datahigh" | "data_high" => Ok(Priority::DataHigh),
        "data" | "default" => Ok(Priority::Data),
        "data-low" | "datalow" | "data_low" => Ok(Priority::DataLow),
        "background" => Ok(Priority::Background),
        _ => Err(unsupported_metadata("cz-zenoh:priority", value)),
    }
}

pub(super) fn parse_congestion_control(value: &str) -> CoreResult<CongestionControl> {
    match normalized_metadata_value(value).as_str() {
        "drop" => Ok(CongestionControl::Drop),
        "block" => Ok(CongestionControl::Block),
        _ => Err(unsupported_metadata("cz-zenoh:congestionControl", value)),
    }
}

fn normalized_metadata_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
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
