//! Form operation vocabulary (W3C WoT TD `Form.op`).

use serde::{Deserialize, Serialize};

/// Operation types of form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    ReadProperty,
    WriteProperty,
    ObserveProperty,
    UnobserveProperty,
    InvokeAction,
    QueryAction,
    /// Action cancellation (TD 1.1 `cancelaction`).
    CancelAction,
    SubscribeEvent,
    UnsubscribeEvent,
    ReadAllProperties,
    WriteAllProperties,
    ReadMultipleProperties,
    WriteMultipleProperties,
    ObserveAllProperties,
    UnobserveAllProperties,
    QueryAllActions,
    /// Subscribe to all events (TD 1.1 `subscribeallevents`).
    SubscribeAllEvents,
    /// Unsubscribe from all events (TD 1.1 `unsubscribeallevents`).
    UnsubscribeAllEvents,
}

impl Operation {
    /// Returns the canonical lowercase operation name matching the W3C WoT
    /// TD serialization (`#[serde(rename_all = "lowercase")]`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadProperty => "readproperty",
            Self::WriteProperty => "writeproperty",
            Self::ObserveProperty => "observeproperty",
            Self::UnobserveProperty => "unobserveproperty",
            Self::InvokeAction => "invokeaction",
            Self::QueryAction => "queryaction",
            Self::CancelAction => "cancelaction",
            Self::SubscribeEvent => "subscribeevent",
            Self::UnsubscribeEvent => "unsubscribeevent",
            Self::ReadAllProperties => "readallproperties",
            Self::WriteAllProperties => "writeallproperties",
            Self::ReadMultipleProperties => "readmultipleproperties",
            Self::WriteMultipleProperties => "writemultipleproperties",
            Self::ObserveAllProperties => "observeallproperties",
            Self::UnobserveAllProperties => "unobserveallproperties",
            Self::QueryAllActions => "queryallactions",
            Self::SubscribeAllEvents => "subscribeallevents",
            Self::UnsubscribeAllEvents => "unsubscribeallevents",
        }
    }
}
