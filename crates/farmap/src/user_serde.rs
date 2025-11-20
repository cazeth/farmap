use crate::user_value::AnyUserValue;
use crate::Fid;
use crate::User;
use chrono::NaiveDateTime;
use serde::Deserialize;
use serde::Serialize;

const LATEST_VERSION: u64 = 2;

#[derive(Deserialize, Serialize)]
pub struct UserSerde {
    #[serde(default)]
    version: u64,
    #[serde(default)]
    #[serde(skip_serializing)]
    user_values: Vec<(AnyUserValue, NaiveDateTime)>,
    fid: Fid,
}

impl From<UserSerde> for User {
    fn from(value: UserSerde) -> Self {
        User::from_user_values(value.fid, value.user_values)
    }
}

impl From<User> for UserSerde {
    fn from(value: User) -> Self {
        let user_values = if let Some(values) = value.all_user_values() {
            values.clone()
        } else {
            Vec::new()
        };
        Self {
            version: LATEST_VERSION,
            user_values,
            fid: value.fid(),
        }
    }
}
