use crate::is_user::IsUser;
use crate::user::User;
use crate::user_collection::UserCollection;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsersSubset<'a> {
    map: HashMap<usize, &'a User>,
}

impl<'a> UsersSubset<'a> {
    pub fn from_filter<F>(users: &'a UserCollection, filter: F) -> Self
    where
        F: Fn(&User) -> bool,
    {
        let filtered_map: HashMap<usize, &'a User> = users
            .iter()
            .filter(|user| filter(user))
            .map(|user| (user.fid(), user))
            .collect();

        Self { map: filtered_map }
    }

    /// apply filter to existing subset and mutate subset.
    pub fn filter<F>(&mut self, filter: F)
    where
        F: Fn(&User) -> bool,
    {
        self.map = self
            .map
            .values()
            .filter(|user| filter(user))
            .map(|user| (user.fid(), *user))
            .collect::<HashMap<usize, &User>>();
    }

    /// return a new struct with filter applied
    pub fn filtered<F>(&self, filter: F) -> Self
    where
        F: Fn(&User) -> bool,
    {
        let mut new = self.clone();
        new.filter(filter);
        new
    }

    pub fn into_map(self) -> HashMap<usize, &'a User> {
        self.map
    }

    pub fn drop_fid(&mut self, fid: usize) -> Option<&User> {
        self.map.get(&fid).map(|v| &**v)
    }

    pub fn add_user(&mut self, user: impl IsUser<'a>) {
        self.map.insert(user.fid(), user.user());
    }

    pub fn user_count(&self) -> usize {
        self.map.len()
    }

    pub fn user(&self, fid: usize) -> Option<&User> {
        self.map.get(&fid).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.map.values().copied()
    }
}

impl<'a> IntoIterator for UsersSubset<'a> {
    type Item = &'a User;
    type IntoIter = std::collections::hash_map::IntoValues<usize, &'a User>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_values()
    }
}

impl<'a> From<HashMap<usize, &'a User>> for UsersSubset<'a> {
    fn from(value: HashMap<usize, &'a User>) -> Self {
        Self { map: value }
    }
}

impl<'a> From<&'a UserCollection> for UsersSubset<'a> {
    fn from(users: &'a UserCollection) -> Self {
        let map: HashMap<usize, &User> = users
            .data()
            .iter()
            .map(|(key, value)| (*key, value))
            .collect();

        Self { map }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::user_collection::tests::dummy_data;

    #[test]
    fn empty_set() {
        let users = UserCollection::default();
        let set = UsersSubset::from(&users);
        assert_eq!(set.user_count(), 0);
    }

    pub fn create_set(collection: &UserCollection) -> UsersSubset {
        UsersSubset::from(collection)
    }

    mod test_user_count {
        use super::*;
        use crate::user::tests::test_fid::is_fid;

        #[track_caller]
        pub fn check_user_count(set: &UsersSubset, count: usize) {
            assert_eq!(set.user_count(), count);
        }

        #[test]
        fn test_user_count_with_new() {
            let users = dummy_data();
            let set = create_set(&users);
            check_user_count(&set, 2);
        }

        #[test]
        fn test_user_count_on_empty_set() {
            let users = crate::user_collection::tests::empty_collection();
            let set = create_set(&users);
            check_user_count(&set, 0);
        }

        #[test]
        fn test_user_count_before_and_after_filter() {
            let users = dummy_data();
            let mut set = create_set(&users);
            let fid_filter = |user: &User| is_fid(user, 1 as u64);
            check_user_count(&set, 2);
            test_filter::check_filter(&mut set, fid_filter);
            check_user_count(&set, 1);
        }

        #[test]
        fn test_user_count_before_and_after_filter_two() {
            let users = dummy_data();
            let mut set = create_set(&users);
            let fid_filter = |user: &User| !is_fid(user, 3 as u64);
            check_user_count(&set, 2);
            test_filter::check_filter(&mut set, fid_filter);
            check_user_count(&set, 2);
        }
    }

    mod test_filter {
        use super::*;

        #[track_caller]
        pub fn check_filter(set: &mut UsersSubset, filter: impl Fn(&User) -> bool) {
            set.filter(filter);
        }
    }
}
