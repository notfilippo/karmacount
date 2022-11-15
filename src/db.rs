use std::collections::HashSet;

use bincode::{deserialize, serialize};
use serde::{de::DeserializeOwned, Serialize};
use sled::{Db, Tree};
use teloxide::types::{MessageId, UserId};

use crate::error::Error;

pub const TREE_KARMA: &str = "karma";
pub const TREE_UP: &str = "up";
pub const TREE_DOWN: &str = "down";
pub const TREE_LAST: &str = "last";
pub const TREE_LAST_MESSAGE: &str = "last_message";
pub const TREE_MEMBERS: &str = "members";

pub struct SpecialTree<T>(Tree, std::marker::PhantomData<T>);

impl<T> SpecialTree<T> {
    pub fn get_or<K>(&self, key: K, default: T) -> Result<T, Error>
    where
        T: DeserializeOwned,
        K: AsRef<[u8]>,
    {
        let value = self.get(key)?.unwrap_or(default);
        Ok(value)
    }

    pub fn remove<K>(&self, key: K) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
        K: AsRef<[u8]>,
    {
        let value = self
            .0
            .remove(key)?
            .map(|bytes| deserialize(&bytes))
            .transpose()?;
        Ok(value)
    }

    pub fn clear(&self) -> Result<(), Error> {
        self.0.clear()?;
        Ok(())
    }

    pub fn insert<K>(&self, key: K, value: T) -> Result<(), Error>
    where
        T: Serialize,
        K: AsRef<[u8]>,
    {
        let bytes = serialize(&value)?;
        self.0.insert(key, bytes)?;
        Ok(())
    }

    pub fn get<K>(&self, key: K) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
        K: AsRef<[u8]>,
    {
        let value = self
            .0
            .get(key)?
            .map(|bytes| deserialize(&bytes))
            .transpose()?;
        Ok(value)
    }
}

pub struct Store {
    pub karma: SpecialTree<i64>,
    pub up: SpecialTree<i64>,
    pub down: SpecialTree<i64>,
    pub last: SpecialTree<i64>,
    pub last_message: SpecialTree<MessageId>,
    pub members: SpecialTree<HashSet<UserId>>,
}

impl Store {
    pub fn new(db: &Db) -> Result<Self, Error> {
        let karma = db.open_tree(TREE_KARMA)?;
        let up = db.open_tree(TREE_UP)?;
        let down = db.open_tree(TREE_DOWN)?;
        let last = db.open_tree(TREE_LAST)?;
        let last_message = db.open_tree(TREE_LAST_MESSAGE)?;
        let members = db.open_tree(TREE_MEMBERS)?;

        Ok(Self {
            karma: SpecialTree(karma, std::marker::PhantomData),
            up: SpecialTree(up, std::marker::PhantomData),
            down: SpecialTree(down, std::marker::PhantomData),
            last: SpecialTree(last, std::marker::PhantomData),
            last_message: SpecialTree(last_message, std::marker::PhantomData),
            members: SpecialTree(members, std::marker::PhantomData),
        })
    }
}
