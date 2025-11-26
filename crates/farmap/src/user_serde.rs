use crate::user_value::AnyNativeUserValue;
use crate::Fid;
use crate::UserStoreWithNativeUserValue;
use chrono::NaiveDateTime;
use serde::Deserialize;
use serde::Serialize;

const LATEST_VERSION: u64 = 2;

#[derive(Deserialize, Serialize)]
pub struct UserSerde {
    #[serde(default)]
    version: u64,
    #[serde(default)]
    #[serde(rename(serialize = "user_values"))]
    user_values_v2: Vec<AnyNativeUserValue>,
    #[serde(default)]
    #[serde(skip_serializing)]
    user_values: Vec<(AnyNativeUserValue, NaiveDateTime)>,
    fid: Fid,
}

impl From<UserSerde> for UserStoreWithNativeUserValue {
    fn from(value: UserSerde) -> Self {
        let user_values = value.user_values.into_iter().map(|x| x.0).collect();

        UserStoreWithNativeUserValue::from_user_values(value.fid, user_values)
    }
}

impl From<UserStoreWithNativeUserValue> for UserSerde {
    fn from(value: UserStoreWithNativeUserValue) -> Self {
        Self {
            version: LATEST_VERSION,
            user_values_v2: value.all_user_values().cloned().collect(),
            user_values: Vec::new(),
            fid: value.fid(),
        }
    }
}
