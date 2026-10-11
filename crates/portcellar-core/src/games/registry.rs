use crate::{PortCellarError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WineRegistryEntry {
    pub key: String,
    pub name: String,
    pub value: WineRegistryValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WineRegistryValue {
    String { data: String },
    Dword { data: u32 },
}

impl WineRegistryEntry {
    pub(crate) fn validate(&self) -> Result<()> {
        let rooted = ["HKCU\\", "HKLM\\", "HKCR\\", "HKU\\"]
            .iter()
            .any(|root| self.key.starts_with(root));
        if !rooted
            || self.key.split('\\').any(|part| part.is_empty())
            || self.name.is_empty()
            || self.key.contains(['\0', '\r', '\n'])
            || self.name.contains('\0')
            || matches!(&self.value, WineRegistryValue::String { data } if data.contains('\0'))
        {
            return Err(PortCellarError::Message(
                "invalid Wine registry entry".into(),
            ));
        }
        Ok(())
    }
}
