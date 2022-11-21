use std::collections::HashSet;

use anyhow::Result;
use bincode::{deserialize, serialize};
use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Db, Tree};
use teloxide::types::{MessageId, UserId};

pub const TREE_KARMA: &str = "karma";
pub const TREE_UP: &str = "up";
pub const TREE_DOWN: &str = "down";
pub const TREE_LAST: &str = "last";
pub const TREE_LAST_MESSAGE: &str = "last_message";
pub const TREE_MEMBERS: &str = "members";
pub const TREE_GRAPH: &str = "graph";

pub struct SpecialTree<T>(Tree, std::marker::PhantomData<T>);

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Measure {
    pub timestamp: i64,
    pub karma: i64,
}

impl Measure {
    pub fn new(karma: i64) -> Self {
        let timestamp = Utc::now().timestamp();
        Self { timestamp, karma }
    }
}

impl<T> SpecialTree<T> {
    pub fn get_or<K>(&self, key: K, default: T) -> Result<T>
    where
        T: DeserializeOwned,
        K: AsRef<[u8]>,
    {
        let value = self.get(key)?.unwrap_or(default);
        Ok(value)
    }

    pub fn remove<K>(&self, key: K) -> Result<Option<T>>
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

    pub fn clear(&self) -> Result<()> {
        self.0.clear()?;
        Ok(())
    }

    pub fn insert<K>(&self, key: K, value: T) -> Result<()>
    where
        T: Serialize,
        K: AsRef<[u8]>,
    {
        let bytes = serialize(&value)?;
        self.0.insert(key, bytes)?;
        Ok(())
    }

    pub fn get<K>(&self, key: K) -> Result<Option<T>>
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
    pub graph: SpecialTree<Vec<Measure>>,
}

impl Store {
    pub fn new(db: &Db) -> Result<Self> {
        let karma = db.open_tree(TREE_KARMA)?;
        let up = db.open_tree(TREE_UP)?;
        let down = db.open_tree(TREE_DOWN)?;
        let last = db.open_tree(TREE_LAST)?;
        let last_message = db.open_tree(TREE_LAST_MESSAGE)?;
        let members = db.open_tree(TREE_MEMBERS)?;
        let graph = db.open_tree(TREE_GRAPH)?;

        Ok(Self {
            karma: SpecialTree(karma, std::marker::PhantomData),
            up: SpecialTree(up, std::marker::PhantomData),
            down: SpecialTree(down, std::marker::PhantomData),
            last: SpecialTree(last, std::marker::PhantomData),
            last_message: SpecialTree(last_message, std::marker::PhantomData),
            members: SpecialTree(members, std::marker::PhantomData),
            graph: SpecialTree(graph, std::marker::PhantomData),
        })
    }
}
