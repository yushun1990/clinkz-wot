//! TD document encoding of Core operation values.
pub use clinkz_wot_core::Operation;
use clinkz_wot_core::Operation as RuntimeOperation;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(remote = "RuntimeOperation")]
enum OperationEncoding {
    ReadProperty,
    WriteProperty,
    ObserveProperty,
    UnobserveProperty,
    InvokeAction,
    QueryAction,
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
    SubscribeAllEvents,
    UnsubscribeAllEvents,
}
/// TD-local encoding for a standalone Core operation.
/// ```
/// use clinkz_wot_td::data_type::{DocumentOperation,Operation};
/// let encoded=DocumentOperation::from(Operation::ReadProperty);
/// assert_eq!(serde_json::to_string(&encoded).unwrap(),"\"readproperty\"");
/// let decoded:DocumentOperation=serde_json::from_str("\"readproperty\"").unwrap();
/// assert_eq!(Operation::from(decoded),Operation::ReadProperty);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentOperation(#[serde(with = "OperationEncoding")] pub(crate) Operation);
pub(crate) struct OperationListRef<'a>(pub &'a [Operation]);
impl Serialize for OperationListRef<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        if let [op] = self.0 {
            return OperationEncoding::serialize(op, s);
        }
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for op in self.0 {
            seq.serialize_element(&DocumentOperation(*op))?;
        }
        seq.end()
    }
}

impl From<Operation> for DocumentOperation {
    fn from(op: Operation) -> Self {
        Self(op)
    }
}
impl From<DocumentOperation> for Operation {
    fn from(op: DocumentOperation) -> Self {
        op.0
    }
}
