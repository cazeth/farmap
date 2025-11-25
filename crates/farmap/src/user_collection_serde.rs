use crate::Fid;
use crate::UserCollection;
use crate::UserStoreWithNativeUserValue;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const LATEST_VERSION: u64 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename(serialize = "UserCollection"))]
pub struct UserCollectionSerde {
    #[serde(default)]
    version: u64,
    map: HashMap<Fid, UserStoreWithNativeUserValue>,
}

impl UserCollectionSerde {
    fn is_latest_version(&self) -> bool {
        self.version == LATEST_VERSION
    }
}

impl From<UserCollectionSerde> for UserCollection {
    fn from(value: UserCollectionSerde) -> Self {
        if !value.is_latest_version() {
            warn!("Data is not latest version. Please overwrite your database.");
        }
        value.map.into()
    }
}

impl From<UserCollection> for UserCollectionSerde {
    fn from(value: UserCollection) -> Self {
        let data = value.data();
        Self {
            version: LATEST_VERSION,
            map: data.clone(),
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    fn test_deserialize_dummy_data(version: u64) {
        let db_path = PathBuf::from(format!("data/dummy-data_db_v{version}.json"));
        let raw = read_to_string(db_path).unwrap();
        let data: UserCollectionSerde = serde_json::from_str(raw.as_str()).unwrap();
        assert_eq!(data.version, version);
        assert_eq!(data.map.len(), 2);
    }

    mod v1 {
        use super::*;

        #[test]
        fn test_deserialize() {
            let raw = r#"{"map":{"1":{"fid":1,"user_values":[[{"DatedSpamUpdate":{"WithoutSourceCommit":"One","date":"2024-01-01"}},"2025-11-15T13:52:49.282499716"],[{"DatedSpamUpdate":{"WithoutSourceCommit":"Zero","date":"2025-01-23"}},"2025-11-15T13:52:49.282549865"]]},"2":{"fid":2,"user_values" :[[{"DatedSpamUpdate":{"WithoutSourceCommit":"Two","date":"2025-01-23"}},"2025-11-15T13:52: 49.282553015"]]}},"version":1}"#;
            let data: UserCollectionSerde = serde_json::from_str(raw).unwrap();
            assert_eq!(data.version, 1);
            assert_eq!(data.map.len(), 2);
        }

        #[test]
        fn deserialize_dummy_data() {
            test_deserialize_dummy_data(1);
        }
    }

    mod v0 {
        use super::*;
        #[test]
        fn deserialize_dummy_data() {
            test_deserialize_dummy_data(0);
        }
    }
}
