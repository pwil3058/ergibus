// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
/// Implement an ordered map to use in place of BTreeMap to sidestep limitation of serde_json
/// with respect to only allowing String keys
#[macro_use]
extern crate serde_derive;

use std::borrow::Borrow;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct OrderedMap<K: Ord, V> {
    pub(crate) keys: Vec<K>,
    pub(crate) values: Vec<V>,
}

impl<K: Ord, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
        }
    }
}

impl<K: Ord, V> OrderedMap<K, V> {
    #[cfg(test)]
    pub(crate) fn is_valid(&self) -> bool {
        for i in 1..self.keys.len() {
            if self.keys[i - 1] >= self.keys[i] {
                return false;
            }
        }
        self.keys.len() == self.values.len()
    }

    /// Return the number of items in this set.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    /// Returns `true` if there is an entry for `key` in the `OrderedMap` and `false` otherwise.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.keys.binary_search_by_key(&key, |x| x.borrow()).is_ok()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    /// Returns an iterator visiting all values in the `OrderedMap` in ascending order of their keys.
    pub fn values(&self) -> Values<'_, V> {
        Values::new(&self.values)
    }

    /// Inserts a key-value (`key`, `value`) pair into the `OrderedMap` and returns the previous
    /// value associated with `key` if it exists and `None` otherwise.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.keys.binary_search(&key) {
            Ok(index) => {
                self.values.push(value);
                Some(self.values.swap_remove(index))
            }
            Err(index) => {
                self.keys.insert(index, key);
                self.values.insert(index, value);
                None
            }
        }
    }

    /// Returns an immutable reference to the value in the `OrderedMap` associated with `key` if
    /// it exists and `None` otherwise.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if let Ok(index) = self.keys.binary_search_by_key(&key, |x| x.borrow()) {
            Some(&self.values[index])
        } else {
            None
        }
    }

    /// Returns an mutable reference to the value in the `OrderedMap` associated with `key` if
    /// it exists and `None` otherwise.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if let Ok(index) = self.keys.binary_search_by_key(&key, |x| x.borrow()) {
            Some(&mut self.values[index])
        } else {
            None
        }
    }
}

// VALUE ITERATOR

/// An Iterator over the values in an ordered map in key order
pub struct Values<'a, V> {
    values: &'a [V],
    index: usize,
}

impl<'a, V> Values<'a, V> {
    pub(crate) fn new(values: &'a [V]) -> Self {
        Self { values, index: 0 }
    }
}

impl<'a, V> Iterator for Values<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.values.get(self.index) {
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod ordered_map_tests {
    use crate::OrderedMap;
    use std::collections::HashMap;

    static TEST_ITEMS_0: &[(&str, (&str, u32))] = &[
        ("hhh", ("HHH", 0)),
        ("aaa", ("AAA", 0)),
        ("ggg", ("GGG", 0)),
        ("sss", ("SSS", 0)),
        ("zzz", ("ZZZ", 0)),
        ("bbb", ("BBB", 0)),
        ("fff", ("FFF", 0)),
        ("iii", ("III", 0)),
        ("qqq", ("QQQ", 0)),
        ("jjj", ("JJJ", 0)),
        ("ddd", ("DDD", 0)),
        ("eee", ("EEE", 0)),
        ("ccc", ("CCC", 0)),
        ("mmm", ("MMM", 0)),
        ("lll", ("LLL", 0)),
        ("nnn", ("NNN", 0)),
        ("ppp", ("PPP", 0)),
        ("rrr", ("RRR", 0)),
    ];

    static TEST_ITEMS_1: &[(&str, (&str, u32))] = &[
        ("hhh", ("HHH", 1)),
        ("aaa", ("AAA", 1)),
        ("ggg", ("GGG", 1)),
        ("sss", ("SSS", 1)),
        ("zzz", ("ZZZ", 1)),
        ("bbb", ("BBB", 1)),
        ("fff", ("FFF", 1)),
        ("iii", ("III", 1)),
        ("qqq", ("QQQ", 1)),
        ("jjj", ("JJJ", 1)),
        ("ddd", ("DDD", 1)),
        ("eee", ("EEE", 1)),
        ("ccc", ("CCC", 1)),
        ("mmm", ("MMM", 1)),
        ("lll", ("LLL", 1)),
        ("nnn", ("NNN", 1)),
        ("ppp", ("PPP", 1)),
        ("rrr", ("RRR", 1)),
    ];

    #[test]
    fn map_default_works() {
        let map = OrderedMap::<u32, u32>::default();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn contains_key() {
        let map = OrderedMap::<String, u32>::default();
        let anything = "anything".to_string();
        assert!(!map.contains_key(&anything));
        assert!(!map.contains_key("whatever"));
    }

    #[test]
    fn map_basic_functionality() {
        let mut map = OrderedMap::<&str, (&str, u32)>::default();
        for (key, value) in TEST_ITEMS_0.iter() {
            //println!("{:?} => {:?}", key, value);
            assert!(map.insert(key, *value).is_none());
            assert!(map.is_valid());
            assert_eq!(map.get(key), Some(value));
            assert_eq!(map.insert(key, *value), Some(*value));
            assert!(map.is_valid());
        }
        let mut hash_map = HashMap::<&str, (&str, u32)>::new();
        for (key, value) in TEST_ITEMS_0.iter() {
            assert!(hash_map.insert(key, *value).is_none());
        }
        for (key, value) in TEST_ITEMS_1.iter() {
            if let Some(old_value) = hash_map.get(key) {
                assert_eq!(map.insert(key, *value), Some(*old_value));
                assert!(map.is_valid());
                assert_eq!(map.get(key), Some(value));
            } else {
                assert!(false);
            }
        }
    }
}
