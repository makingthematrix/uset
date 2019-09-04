#![macro_use]

use super::uset::USet;
use itertools::{Itertools, MinMaxResult};
use std::clone::Clone;
use std::cmp;
use std::fmt;
use std::ops::Add;

use std::iter::FromIterator;

/// A map of unsigned integers (usizes) to values of the type T implementing `PartialEq` and `Clone`.
/// The map is implemented as a vector of options of T, where `vec[n - offset] == Some(t)` means that
/// the set contains the value `t` under the index `n`. Intended for handling small to medium number
/// of elements.
/// Searching is O(1), addition and removal is O(1) for values within the map's capacity, O(n)
/// otherwise, as values have to be copied to a new vector. The map is sorted. Getting `min` and
/// `max` is O(1).
///
/// In all cases when values are moved to a new vector, the operation ensures that
/// the size of the new vector is `max - min`: in that case the minimum value is at vec[0]
/// (so `offset == min`) and `max - offset == capacity`. However, for performance
/// purposes, if the operation does not require new allocation, the capacity might be
/// left bigger than `max - min`.
///
/// `UMap` closely cooperates with `USet`. The idiomatic way to work with it is to put all the
/// elements in one map stored in an accesible place, query it for sets of identifiers which
/// fulfill certain conditions, carry them around, as they are much lightweight than the map,
/// perform operations on them, and only at the end use them to retrieve the elements or make
/// changes to the map.

/// Creates a `UMap` with the given pairs of identifiers and elements.
/// Equivalent to calling [`from_slice`].
///
/// [`from_slice`]: #method.from_slice
#[allow(unused_macros)]
macro_rules! umap {
    ($($x:expr),*) => (UMap::from_slice(&vec![$($x),*]))
}

#[derive(Default, Clone)]
pub struct UMap<T> {
    pub vec: Vec<Option<T>>,
    len: usize,
    offset: usize,
    min: usize,
    max: usize,
}

#[derive(Debug, Clone)]
pub struct UMapIter<'a, T: 'a> {
    handle: &'a UMap<T>,
    index: usize,
    rindex: usize,
}

impl<'a, T> Iterator for UMapIter<'a, T>
where
    T: Clone + PartialEq,
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let max = self.handle.vec.len() - self.rindex;
        while self.index < max {
            let index = self.index;
            self.index += 1;
            if let Some(ref value) = self.handle.vec[index] {
                return Some((index + self.handle.offset, value));
            }
        }
        None
    }
}

impl<'a, T> DoubleEndedIterator for UMapIter<'a, T>
where
    T: Clone + PartialEq,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let len = self.handle.vec.len();
        while self.rindex < len - self.index {
            let index = len - self.rindex - 1;
            self.rindex += 1;
            if let Some(ref value) = self.handle.vec[index] {
                return Some((index + self.handle.offset, &value));
            }
        }
        None
    }
}

pub const INITIAL_CAPACITY: usize = 8;

impl<T> UMap<T>
where
    T: Clone + PartialEq,
{
    /// Constructs a new, empty `UMap`.
    ///
    /// The map will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let map: UMap<&str> = UMap::<&str>::new();
    /// ```
    pub fn new() -> Self {
        UMap::with_capacity(0)
    }

    /// Constructs a new, empty `UMap` with the specified capacity.
    ///
    /// The map will be able to hold exactly `capacity` elements without
    /// reallocating. If `capacity` is 0, the internal vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// *capacity* specified, the vector will have a zero *length*.
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::with_capacity(10);
    ///
    /// // The map contains no items, even though it has capacity for more
    /// assert_eq!(map.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     map.push(i);
    /// }
    ///
    /// // ...but this may make the vector reallocate
    /// map.push(11);
    /// ```
    pub fn with_capacity(size: usize) -> Self {
        UMap {
            vec: vec![None; size],
            len: 0,
            offset: 0,
            min: 0,
            max: 0,
        }
    }

    /// Returns the number of elements in the map, also referred to as its 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// assert_eq!(map.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    /// assert!(map.is_empty());
    ///
    /// map.push("a".to_string());
    /// assert!(!map.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements the map can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let map = UMap::<&str>::with_capacity(10);
    /// assert_eq!(map.capacity(), 10);
    /// ```
    pub fn capacity(&self) -> usize {
        self.vec.len()
    }

    /// Shrinks the map to the minimal size able to hold its elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (50, "b")]);
    /// assert!(map.capacity() >= 50);
    /// map.remove(1);
    /// assert!(map.capacity() >= 50);
    /// map.shrink_to_fit();
    /// assert_eq!(1, map.capacity());
    /// ```
    pub fn shrink_to_fit(&mut self) {
        if !self.is_empty() && (self.vec[0].is_none() || self.vec[self.vec.len() - 1].is_none()) {
            let mut vec = vec![None; self.max - self.min + 1];
            for id in self.min..=self.max {
                vec[id - self.min] = self.get(id);
            }
            self.vec = vec;
            self.offset = self.min;
        } else if self.is_empty() && self.capacity() > 0 {
            self.vec = Vec::with_capacity(0);
        }
    }

    /// Shortens the map, keeping the first `len` elements and dropping the rest.
    /// If `len` is greater than the map's current length, this has no effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// This method does not shrink the map's capacity.
    /// If you want to shrink the set's capacity, call `shrink_to_fit` afterwards.
    ///
    /// # Examples
    ///
    /// Truncating a five element map to two elements:
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e")]);
    /// map.truncate(2);
    /// assert_eq!(map, UMap::from_slice(&[(1, "a"), (2, "b")]));
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current length:
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// map.truncate(8);
    /// assert_eq!(map, UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]));
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`] method.
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// map.truncate(0);
    /// assert!(map.is_empty());
    /// ```
    ///
    /// [`clear`]: #method.clear
    /// [`drain`]: #method.drain
    /// [`shrink_to_fit`]: #method.shrink_to_fit
    pub fn truncate(&mut self, len: usize) {
        if !self.is_empty() && len > 0 && len < self.len {
            let mut values_left = len;
            let mut new_max = 0usize;
            self.vec
                .iter_mut()
                .enumerate()
                .for_each(|(index, value_holder)| {
                    if value_holder.is_some() {
                        if values_left > 0 {
                            values_left -= 1;
                            new_max = index;
                        } else {
                            *value_holder = None;
                        }
                    }
                });
            self.max = new_max + self.offset;
            self.len = len;
        } else if !self.is_empty() && len == 0 {
            self.vec
                .iter_mut()
                .for_each(|value_holder| *value_holder = None);
            self.offset = 0;
            self.min = 0;
            self.max = 0;
            self.len = 0;
        }
    }

    /// Works like [`truncate`], but returns the removed elements in the form of a new map.
    /// This method does not shrink the map's capacity.
    /// If you want to shrink the map's capacity, call [`shrink_to_fit`] afterwards.
    ///
    /// # Examples
    ///
    /// Draining a five element set to two elements:
    ///
    /// ```
    /// use self::uset::core::umap::*;
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    /// let d = String::from("d");
    /// let e = String::from("e");
    /// let mut map = UMap::from_slice(&[(1, a.clone()), (2, b.clone()), (3, c.clone()), (4, d.clone()), (5, e.clone())]);
    /// let drained = map.drain(2);
    /// assert_eq!(map, UMap::from_slice(&[(1, a), (2, b)]));
    /// assert_eq!(drained, UMap::from_slice(&[(3, c), (4, d), (5, e)]));
    /// ```
    ///
    /// No draining occurs when `len` is greater than the map's current length:
    ///
    /// ```
    /// use self::uset::core::umap::*;
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    /// let mut map = UMap::from_slice(&[(1, a.clone()), (2, b.clone()), (3, c.clone())]);
    /// let drained = map.drain(8);
    /// assert_eq!(map, UMap::from_slice(&[(1, a), (2, b), (3, c)]));
    /// assert!(drained.is_empty());
    /// ```
    ///
    /// Draining when `len == 0` is equivalent to cloning the map and calling the [`clear`]
    /// method on the original one.
    ///
    /// ```
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, a.clone()), (2, b.clone()), (3, c.clone())]);
    /// let drained = map.drain(0);
    /// assert!(map.is_empty());
    /// assert_eq!(drained, UMap::from_slice(&[(1, a), (2, b), (3, c)]));
    /// ```
    ///
    /// [`clear`]: #method.clear
    /// [`truncate`]: #method.truncate
    /// [`shrink_to_fit`]: #method.shrink_to_fit
    pub fn drain(&mut self, len: usize) -> Self {
        if !self.is_empty() && len > 0 && len < self.len {
            let mut new_map = UMap::with_capacity(self.len - len);
            let mut values_left = len;
            let mut new_max = 0usize;
            let offset = self.offset;
            self.vec
                .iter_mut()
                .enumerate()
                .for_each(|(index, value_holder)| {
                    if let Some(ref value) = value_holder {
                        if values_left > 0 {
                            values_left -= 1;
                            new_max = index;
                        } else {
                            new_map.put(index + offset, value.clone());
                            *value_holder = None;
                        }
                    }
                });
            self.max = new_max + self.offset;
            self.len = len;
            new_map.shrink_to_fit(); // TODO integrate with populating the vector
            new_map
        } else if !self.is_empty() && len == 0 {
            let new_map = self.clone();
            self.vec.iter_mut().for_each(|value_holder| {
                if value_holder.is_some() {
                    *value_holder = None
                }
            });
            self.offset = 0;
            self.min = 0;
            self.max = 0;
            self.len = 0;
            new_map
        } else {
            UMap::with_capacity(0)
        }
    }

    /// Clears the map, removing all elements.
    ///
    /// Note that this method has no effect on the allocated capacity of the map.
    /// If you want to shrink the set's capacity, call [`shrink_to_fit`] afterwards.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    ///
    /// map.clear();
    ///
    /// assert!(map.is_empty());
    /// ```
    ///
    /// [`shrink_to_fit`]: #method.shrink_to_fit
    pub fn clear(&mut self) {
        self.truncate(0)
    }

    /// Changes the map's capacity, so that it can hold new elements up to the `new_capacity + offset - 1`
    /// value without reallocation. Note that `new_capacity + offset - 1` is now the largest **identifier**
    /// the map can hold without the reallocation, not the total number of values that can be held.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    /// let d = String::from("d");
    /// let mut map = UMap::from_slice(&[(1, a), (8, b)]);
    /// assert_eq!(8, map.capacity());
    /// map.enlarge_capacity_to(10);
    /// assert_eq!(10, map.capacity());
    /// map.put(9, c); // no reallocation needed
    /// assert_eq!(10, map.capacity());
    /// map.put(11, d); // this will trigger reallocation
    /// assert_eq!(11, map.capacity());
    /// ```
    pub fn enlarge_capacity_to(&mut self, new_capacity: usize) {
        if new_capacity > self.capacity() {
            self.vec.resize(new_capacity, None);
        }
    }

    /// Adds the element at the end of the map and returns its new identifier.
    /// This is equivalent to calling [`put`] with `id == self.max + 1` and remembering the `id`.
    ///
    /// If you plan to use it in a loop, better first estimate the size of the map after the whole
    /// operation, and call [`enlarge_capacity_to`] in order to avoid frequent reallocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    /// let id = map.push(String::from("a"));
    /// let value = map.get(id);
    /// assert_eq!(Some(String::from("a")), value);
    /// ```
    ///
    /// [`put`]: #method.put
    /// [`enlarge_capacity_to`]: #method.enlarge_capacity_to
    pub fn push(&mut self, value: T) -> usize {
        let id = self.max + 1;
        self.put(id, value);
        id
    }

    pub fn push_all(&mut self, slice: &[T]) -> Vec<usize> {
        self.enlarge_capacity_to(self.capacity() + slice.len());
        slice.iter().map(|v| self.push(v.clone())).collect()
    }

    /// Adds the element with the given id to the map, possibly overwriting the old element
    /// at that position, and reallocates if needed.
    /// Reallocation is not necessary if the id falls in-between the current min and max.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, String::from("a")), (3, String::from("b"))]);
    /// map.put(2, String::from("c"));
    /// assert_eq!(map, UMap::from_slice(&[(1, String::from("a")), (2, String::from("c")), (3, String::from("b"))]));
    /// ```
    pub fn put(&mut self, id: usize, value: T) {
        match id {
            _ if self.capacity() == 0 => {
                self.vec = vec![None; INITIAL_CAPACITY];
                self.vec[0] = Some(value);
                self.min = id;
                self.len += 1;
                self.max = id;
                self.offset = id;
            }
            _ if self.is_empty() => {
                self.vec[0] = Some(value);
                self.min = id;
                self.len = 1;
                self.max = id;
                self.offset = id;
            }
            _ if id < self.offset => {
                let mut vec = vec![None; self.max - id + 1];
                vec[0] = Some(value);
                for i in self.min..=self.max {
                    vec[i - id] = self.get(i);
                }
                self.vec = vec;
                self.len += 1;
                self.min = id;
                self.offset = id;
            }
            _ if id >= self.offset + self.capacity() => {
                self.vec.resize(id + 1 - self.offset, None);
                self.vec[id - self.offset] = Some(value);
                self.len += 1;
                self.max = id;
            }
            _ if self.vec[id - self.offset].is_none() => {
                self.vec[id - self.offset] = Some(value);
                self.len += 1;
                if id < self.min {
                    self.min = id
                } else if id > self.max {
                    self.max = id
                }
            }
            _ => {}
        }
    }

    /// Returns `true` if the map contains the given id.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    /// let id = map.push("a");
    /// assert!(map.contains(id));
    /// assert_eq!(1, map.len());
    /// ```
    pub fn contains(&self, id: usize) -> bool {
        id >= self.min && id <= self.max && self.vec[id - self.offset].is_some()
    }

    /// Returns `Some` with a copy of the element under the given id, or `None` otherwise.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, String::from("a")), (2, String::from("b"))]);
    /// let b = map.get(2);
    /// assert_eq!(Some(String::from("b")), b);
    /// let c = map.get(3);
    /// assert_eq!(None, c);
    /// ```
    pub fn get(&self, id: usize) -> Option<T> {
        if id >= self.min && id <= self.max {
            unsafe { self.vec.get_unchecked(id - self.offset).clone() }
        } else {
            None
        }
    }

    /// Returns `Some` with a reference to the element under the given id, or `None` otherwise.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, String::from("a")), (2, String::from("b"))]);
    /// let b = map.get_ref(2);
    /// assert_eq!(Some(&String::from("b")), b);
    /// let c = map.get_ref(3);
    /// assert_eq!(None, c);
    /// ```
    pub fn get_ref(&self, id: usize) -> Option<&T> {
        if id >= self.min && id <= self.max {
            unsafe {
                if let Some(ref v) = self.vec.get_unchecked(id - self.offset) {
                    Some(v)
                } else {
                    None
                }
            }
        } else {
            None
        }
    }

    /// Returns `Some` with a mutable reference to the element under the given id, or `None` otherwise.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// let mut map = UMap::from_slice(&[(1, String::from("a")), (2, String::from("b"))]);
    /// let mut b_ref = map.get_ref_mut(2);
    /// assert_eq!(Some(&mut String::from("b")), b_ref);
    /// if let Some(value) = map.get_ref_mut(2) {
    ///     *value = String::from("d");
    /// }
    /// assert_eq!(Some(String::from("d")), map.get(2));
    /// let c = map.get_ref_mut(3);
    /// assert_eq!(None, c);
    /// ```
    pub fn get_ref_mut(&mut self, id: usize) -> Option<&mut T> {
        if id >= self.min && id <= self.max {
            unsafe {
                if let Some(ref mut v) = self.vec.get_unchecked_mut(id - self.offset) {
                    Some(v)
                } else {
                    None
                }
            }
        } else {
            None
        }
    }

    /// Removes the element from the map and returns it.
    /// Does nothing if the element with the given id is not in the map (returns `None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// let b = map.remove(2);
    /// assert_eq!(map, UMap::from_slice(&[(1, "a"), (3, "c")]));
    /// assert_eq!(b, Some("b"))
    /// ```
    pub fn remove(&mut self, id: usize) -> Option<T> {
        match id {
            _ if id < self.min || id > self.max || !self.contains(id) => None,
            _ if self.len == 1 => {
                let t = self.vec[id - self.offset].clone();
                self.vec[id - self.offset] = None;
                self.max = 0;
                self.min = 0;
                self.len = 0;
                self.offset = 0;
                t
            }
            _ if id > self.min && id < self.max => {
                let t = self.vec[id - self.offset].clone();
                self.vec[id - self.offset] = None;
                self.len -= 1;
                t
            }
            _ if id == self.min => {
                let t = self.vec[id - self.offset].clone();
                self.vec[id - self.offset] = None;
                self.len -= 1;
                self.min = (self.min..self.max)
                    .find(|&i| self.vec[i - self.offset].is_some())
                    .unwrap_or(self.max);
                t
            }
            _ if id == self.max => {
                let t = self.vec[id - self.offset].clone();
                self.vec[id - self.offset] = None;
                self.len -= 1;
                self.max = (self.min..self.max)
                    .rev()
                    .find(|&i| self.vec[i - self.offset].is_some())
                    .unwrap_or(self.min);
                t
            }
            _ => None,
        }
    }

    // Returns the keys of the map as `USet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// assert_eq!(USet::from_slice(&[1, 2, 3]), map.keys());
    /// ```
    pub fn keys(&self) -> USet {
        let set: Vec<bool> = self.vec.iter().map(Option::is_some).collect();
        USet::from_fields(set, self.offset)
    }

    /// Removes and returns the element at position `index` within the map.
    /// Returns `None` if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::from_slice(&[(1, "a"), (2, "b"), (3, "c")]);
    /// assert_eq!(map.pop(1), Some((2, "b")));
    /// assert_eq!(map, UMap::from_slice(&[(1, "a"), (3, "c")]));
    /// ```
    pub fn pop(&mut self, index: usize) -> Option<(usize, T)> {
        let d = self.at_index(index);
        if let Some((id, value)) = d {
            self.remove(id);
            Some((id, value.clone()))
        } else {
            None
        }
    }

    /// The map allows to access its values by index.
    /// It's the same as if the user created an iterator and took the n-th element.
    /// `UMap` currently does not implement the `Index` trait.
    ///
    ///# Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let map = UMap::from_slice(&[(2, "a"), (3, "b"), (4, "c")]);
    /// assert_eq!(map.at_index(0), Some((2, "a")));
    /// assert_eq!(map.at_index(1), Some((3, "b")));
    /// assert_eq!(map.at_index(2), Some((4, "c")));
    /// assert_eq!(map.at_index(3), None);
    /// ```
    pub fn at_index(&self, index: usize) -> Option<(usize, T)> {
        if index >= self.len {
            None
        } else {
            let mut it = self.iter();
            for _i in 0..index {
                it.next();
            }
            it.next().map(|(id, value)| (id, value.clone()))
        }
    }

    /// Returns an iterator over the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    ///
    /// let map = UMap::from_slice(&[(1, a), (2, b), (4, c)]);
    /// let mut iterator = map.iter();
    ///
    /// assert_eq!(iterator.next(), Some((1, &String::from("a"))));
    /// assert_eq!(iterator.next(), Some((2, &String::from("b"))));
    /// assert_eq!(iterator.next(), Some((4, &String::from("c"))));
    /// assert_eq!(iterator.next(), None);
    /// ```
    pub fn iter(&self) -> UMapIter<T> {
        UMapIter {
            handle: self,
            index: 0,
            rindex: 0,
        }
    }

    /// Returns the smallest identifier in the map or None if the map is empty.
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    /// assert_eq!(map.min(), None);
    ///
    /// map.put(2, "a".to_string());
    /// assert_eq!(map.min(), Some(2));
    ///
    /// map.put(3, "b".to_string());
    /// assert_eq!(map.min(), Some(2));
    ///
    /// map.put(1, "c".to_string());
    /// assert_eq!(map.min(), Some(1));
    /// ```
    pub fn min(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }

    /// Returns the largest identifier in the map or None if the map is empty.
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    /// assert_eq!(map.max(), None);
    ///
    /// map.put(2, "a".to_string());
    /// assert_eq!(map.max(), Some(2));
    ///
    /// map.put(3, "b".to_string());
    /// assert_eq!(map.max(), Some(3));
    ///
    /// map.put(1, "c".to_string());
    /// assert_eq!(map.max(), Some(3));
    /// ```
    pub fn max(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }

    fn make_from_slice(slice: &[(usize, T)]) -> (usize, usize, usize, Vec<Option<T>>) {
        match slice.iter().minmax_by_key(|(ref id, _)| *id) {
            MinMaxResult::NoElements => (0, 0, 0, Vec::<Option<T>>::new()),
            MinMaxResult::OneElement((ref id, value)) => {
                (*id, *id, 1, vec![Some(value.clone()); 1])
            }
            MinMaxResult::MinMax(&(min, _), &(max, _)) => {
                let len = slice.len();
                let capacity = cmp::max(INITIAL_CAPACITY, max + 1 - min);
                let mut vec = vec![None; capacity];
                slice
                    .iter()
                    .for_each(|(id, value)| vec[*id - min] = Some(value.clone()));
                (min, max, len, vec)
            }
        }
    }

    /// Creates a map from a slice of tuples: identifiers and values.
    /// This is the same as the `from_iter` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let vec = vec![(2usize, "a"), (4, "b"), (5, "c")];
    /// let map = UMap::from_slice(&vec);
    /// assert_eq!(vec.len(), map.len());
    /// assert_eq!(Some("a"), map.get(2));
    /// assert_eq!(Some("b"), map.get(4));
    /// assert_eq!(Some("c"), map.get(5));
    /// ```
    pub fn from_slice(slice: &[(usize, T)]) -> Self {
        if slice.is_empty() {
            UMap::new()
        } else {
            let (min, max, len, new_vec) = UMap::make_from_slice(slice);
            UMap {
                vec: new_vec,
                len,
                offset: min,
                min,
                max,
            }
        }
    }

    fn debug_compare(self: &Self, other: &UMap<T>) {
        // don't perform operation on maps if they have different elements at the same places - clearly something's messed up
        debug_assert!(self
            .iter()
            .zip(other.iter())
            .find(|&((i1, ref v1), (i2, ref v2))| i1 == i2 && v1 != v2)
            .is_none());
    }

    /// Adds all tuples in the slice to the map.
    ///
    /// It's equivalent to calling `put` for every element or to the `extend` method over the iterator,
    /// but it will be faster if the slice contains many elements which would require reallocation.
    /// In that case, `put_all` will perform reallocation only once.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    ///
    /// let mut map = UMap::new();
    ///
    /// let v1 = vec![(2, "a"), (4, "b")];
    /// map.put_all(&v1);
    ///  assert_eq!(2, map.len());
    ///
    /// let v2 = vec![(3, "c"), (5, "d")];
    /// map.put_all(&v2);
    /// assert_eq!(4, map.len());
    ///
    /// assert_eq!(Some("a"), map.get(2));
    /// assert_eq!(Some("c"), map.get(3));
    /// assert_eq!(Some("b"), map.get(4));
    /// assert_eq!(Some("d"), map.get(5));
    /// ```
    pub fn put_all(&mut self, slice: &[(usize, T)]) {
        if !slice.is_empty() {
            if self.is_empty() {
                let (min, max, len, new_vec) = UMap::make_from_slice(slice);
                self.min = min;
                self.max = max;
                self.offset = min;
                self.len = len;
                self.vec = new_vec;
            } else {
                let (min, max) = match slice.iter().minmax_by_key(|&(id, _)| *id) {
                    MinMaxResult::NoElements => (0, 0), // should not happen1
                    MinMaxResult::OneElement(&(min, _)) => (min, min),
                    MinMaxResult::MinMax(&(min, _), &(max, _)) => (min, max),
                };

                if min >= self.min && max <= self.max {
                    slice.iter().for_each(|(ref id, value)| {
                        if self.vec[*id - self.offset].is_none() {
                            self.vec[*id - self.offset] = Some(value.clone());
                            self.len += 1;
                        }
                    })
                } else {
                    let new_min = cmp::min(self.min, min);
                    let new_max = cmp::max(self.max, max);
                    let mut new_vec = vec![None; new_max - new_min + 1];
                    self.iter()
                        .skip(self.min - self.offset)
                        .take(self.max - self.min + 1)
                        .for_each(|(id, value)| new_vec[id - new_min] = Some(value.clone()));
                    slice.iter().for_each(|(ref id, value)| {
                        if new_vec[*id - new_min].is_none() {
                            new_vec[*id - new_min] = Some(value.clone());
                            self.len += 1;
                        }
                    });
                    self.min = new_min;
                    self.offset = new_min;
                    self.max = new_max;
                    self.vec = new_vec;
                }
            }
        }
    }

    /// Joins two maps of the same type, creating a new one. Values are cloned.
    /// If one of the maps is empty, the other is cloned.
    ///
    /// # Panics
    ///
    /// Panics if both maps contain two different values under the same identifier.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// let map1 = UMap::from_slice(&[(1, "a".to_string()), (3, "c".to_string())]);
    /// let map2 = UMap::from_slice(&[(2, "b".to_string()), (4, "d".to_string())]);
    /// let map3 = map1.join(&map2);
    /// assert_eq!(4, map3.len());
    /// assert_eq!(map3, UMap::from_slice(&[(1, "a".to_string()), (2, "b".to_string()), (3, "c".to_string()), (4, "d".to_string())]));
    /// ```
    pub fn join(&self, other: &Self) -> Self {
        if self.is_empty() {
            if other.is_empty() {
                UMap::new()
            } else {
                other.clone()
            }
        } else if other.is_empty() {
            if self.is_empty() {
                UMap::new()
            } else {
                self.clone()
            }
        } else {
            self.debug_compare(other);
            let min: usize = cmp::min(self.min, other.min);
            let max: usize = cmp::max(self.max, other.max);

            let mut vec = vec![None; max + 1 - min];
            let mut len = 0usize;

            vec.iter_mut().enumerate().for_each(|(id, value)| {
                if self.contains(id + min) {
                    *value = self.get(id + min);
                    len += 1;
                } else if other.contains(id + min) {
                    *value = other.get(id + min);
                    len += 1;
                }
            });

            UMap {
                vec,
                len,
                offset: min,
                min,
                max,
            }
        }
    }

    /// Returns a submap of all elements with identifiers belonging to `set` which also belong to the map.
    /// Values are cloned.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(2, "a"), (4, "b"), (3, "c"), (5, "d")]);
    /// let set = USet::from_slice(&[2, 3]);
    /// let map2 = map.submap(&set);
    /// assert_eq!(map2, UMap::from_slice(&[(2, "a"), (3, "c")]));
    /// ```
    pub fn submap(&self, set: &USet) -> Self {
        if set.is_empty() {
            UMap::new()
        } else {
            let min = set.min().unwrap();
            let max = set.max().unwrap();
            let mut vec = vec![None; max - min + 1];
            set.iter().for_each(|id| vec[id - min] = self.get(id));
            UMap {
                vec,
                len: set.len(),
                offset: min,
                min,
                max,
            }
        }
    }

    /// Returns a vector of all values with identifiers belonging to `set` which also belong to the map.
    /// Values are cloned.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(2, "a"), (4, "b"), (3, "c"), (5, "d")]);
    /// let set = USet::from_slice(&[2, 3]);
    /// let vec = map.retrieve(&set);
    /// assert_eq!(vec, vec!["a", "c"]);
    /// ```
    pub fn retrieve(&self, set: &USet) -> Vec<T> {
        let mut vec = Vec::with_capacity(set.len());
        set.iter()
            .filter_map(|id| self.get(id))
            .for_each(|value| vec.push(value));
        vec
    }

    /// Returns a vector of references to all values with identifiers belonging to `set`
    /// which also belong to the map.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    /// let a = String::from("a");
    /// let b = String::from("b");
    /// let c = String::from("c");
    /// let d = String::from("d");
    /// let e = String::from("e");
    /// let map = UMap::from_slice(&[(2, a.clone()), (4, b.clone()), (3, c.clone()), (5, d.clone())]);
    /// let set = USet::from_slice(&[2, 3]);
    /// let vec = map.retrieve_ref(&set);
    /// assert_eq!(vec, vec![&a, &c]);
    /// ```
    pub fn retrieve_ref(&self, set: &USet) -> Vec<&T> {
        let mut vec = Vec::with_capacity(set.len());
        set.iter()
            .filter_map(|id| self.get_ref(id))
            .for_each(|value| vec.push(value));
        vec
    }

    /// Returns a set of identifiers for which elements in the map fulfill the `predicate`.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "cc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// let set = map.query(|v| { v.len() > 1 });
    /// assert_eq!(set, USet::from_slice(&[2, 3, 11]));
    /// ```
    pub fn query(&self, predicate: impl Fn(&T) -> bool) -> USet {
        if self.is_empty() {
            USet::new()
        } else {
            let mut vec = Vec::with_capacity(self.max - self.min + 1);
            for id in self.min..=self.max {
                if let Some(v) = self.get_ref(id) {
                    if predicate(v) {
                        vec.push(id);
                    }
                }
            }

            USet::from_slice(&vec)
        }
    }

    /// A utility function making it easier to call `all` on values in the map.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map1 = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "cc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// assert!(!map1.all(|v| { v.len() > 1 }));
    /// let set = map1.query(|v| { v.len() > 1 });
    /// let map2 = map1.submap(&set);
    /// assert!(map2.all(|v| { v.len() > 1 }));
    /// ```
    pub fn all(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.iter().all(|(_id, value)| predicate(value))
    }

    /// A utility function making it easier to call `any` on values in the map.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map1 = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "cc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// assert!(map1.any(|v| { v.len() > 1 }));
    /// let set = map1.query(|v| { v.len() > 1 });
    /// let map2 = map1.submap(&set);
    /// assert!(!map2.any(|v| { v.len() == 1 }));
    /// ```
    pub fn any(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.iter().any(|(_id, value)| predicate(value))
    }

    /// A utility method making it easier to call `all` on values in the map with identifiers
    /// belonging to the given `subset`. You could achieve the same by calling [`retrieve`] on
    /// the map with `subset` as the argument, and then `all` on the iterator over the resulting
    /// vector.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "ccc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// let set = map.query(|v| { v.len() > 1 });
    /// assert!(map.all_in_subset(&set, |v| { v.len() > 1 }));
    /// assert!(!map.all_in_subset(&set, |v| { v.len() == 2 }));
    /// ```
    ///
    /// [`retrieve`]: #method.retrieve
    pub fn all_in_subset(&self, subset: &USet, predicate: impl Fn(&T) -> bool) -> bool {
        !self
            .iter()
            .any(|(id, value)| subset.contains(id) && !predicate(value))
    }

    /// A utility method making it easier to call `any` on values in the map with identifiers
    /// belonging to the given `subset`. You could achieve the same by calling [`retrieve`] on
    /// the map with `subset` as the argument, and then `any` on the iterator over the resulting
    /// vector.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let map = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "ccc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// let set = map.query(|v| { v.len() > 1 });
    /// assert!(!map.any_in_subset(&set, |v| { v.len() == 1 }));
    /// assert!(map.any_in_subset(&set, |v| { v.len() == 3 }));
    /// ```
    ///
    /// [`retrieve`]: #method.retrieve
    pub fn any_in_subset(&self, subset: &USet, predicate: impl Fn(&T) -> bool) -> bool {
        self.iter()
            .any(|(id, value)| subset.contains(id) && predicate(value))
    }

    /// A utility method for removing all elements with identifiers in `subset` from the map.
    /// As [`remove`] does not perform reallocation, `remove_all` is equivalent to calling `remove`
    /// on all identifiers in `subset`. (Contrary to [`put`] and [`put_all`]).
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// use self::uset::core::uset::*;
    ///
    /// let mut map = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "ccc".to_string()), (5, "d".to_string()), (11, "ee".to_string())]);
    /// let set = map.query(|v| { v.len() > 1 });
    /// map.remove_all(&set);
    /// assert_eq!(map, UMap::from_slice(&[(4, "b".to_string()),(5, "d".to_string())]))
    /// ```
    ///
    /// [`remove`]: #method.remove
    /// [`put`]: #method.put
    /// [`put_all`]: #method.put_all
    pub fn remove_all(&mut self, subset: &USet) {
        subset.iter().for_each(|id| {
            self.remove(id);
        });
    }

    /// Replaces the value under the identifier `id`.
    /// If the map does not contain any element with the given identifier, the [`put`] method is called.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// let mut map = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "ccc".to_string())]);
    /// map.replace(3, "d".to_string());
    /// assert_eq!(map, UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "d".to_string())]));
    ///
    /// map.replace(5, "e".to_string());
    /// assert_eq!(map, UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "d".to_string()), (5, "e".to_string())]));
    /// ```
    ///
    /// [`put`]: #method.put
    pub fn replace(&mut self, id: usize, value: T) {
        if let Some(v) = self.get_ref_mut(id) {
            *v = value;
        } else {
            self.put(id, value);
        }
    }

    /// Replaces all the values with the common identifiers in the map with the ones from the `other`.
    /// If the given identifier does not exist in the map, the [`put`] method is called.
    /// Since we want to preserve the original `other` map, values are cloned.
    /// You can use this method instead of [`join`] if you are sure that it is not an error that some
    /// of the elements in both maps have different values under the same identifiers.
    ///
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::umap::*;
    /// let mut map1 = UMap::from_slice(&[(2, "aa".to_string()), (4, "b".to_string()), (3, "ccc".to_string())]);
    /// let map2 = UMap::from_slice(&[(2, "d".to_string()), (3, "e".to_string())]);
    /// map1.replace_all(&map2);
    /// assert_eq!(map1, UMap::from_slice(&[(2, "d".to_string()), (4, "b".to_string()), (3, "e".to_string())]));
    ///
    /// let map3 = UMap::from_slice(&[(4, "f".to_string()), (6, "g".to_string())]);
    /// map1.replace_all(&map3);
    /// assert_eq!(map1, UMap::from_slice(&[(2, "d".to_string()), (4, "f".to_string()), (3, "e".to_string()), (6, "g".to_string())]));
    /// ```
    ///
    /// [`put`]: #method.put
    /// [`join`]: #method.join
    pub fn replace_all(&mut self, other: &UMap<T>) {
        other.iter().for_each(|(id, v)| self.replace(id, v.clone()));
    }
}

impl<T> PartialEq for UMap<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self.min == other.min
            && self.max == other.max
            && self
                .vec
                .iter()
                .skip(self.min - self.offset)
                .take(self.max + 1 - self.min)
                .zip(
                    other
                        .vec
                        .iter()
                        .skip(other.min - other.offset)
                        .take(other.max + 1 - other.min),
                )
                .all(|(a, b)| *a == *b)
    }
}

impl<T> Eq for UMap<T> where T: Clone + PartialEq {}

impl<'a, T> Add for &'a UMap<T>
where
    T: Clone + PartialEq,
{
    type Output = UMap<T>;
    fn add(self, other: &UMap<T>) -> UMap<T> {
        self.join(other)
    }
}

impl<'a, T> From<&'a [(usize, T)]> for UMap<T>
where
    T: Clone + PartialEq,
{
    fn from(slice: &'a [(usize, T)]) -> Self {
        UMap::from_slice(slice)
    }
}

impl<T> From<Vec<(usize, T)>> for UMap<T>
where
    T: Clone + PartialEq,
{
    fn from(vec: Vec<(usize, T)>) -> Self {
        UMap::from_slice(&vec)
    }
}

impl<A> FromIterator<(usize, A)> for UMap<A>
where
    A: Clone + PartialEq,
{
    fn from_iter<T: IntoIterator<Item = (usize, A)>>(iter: T) -> Self {
        let vec: Vec<(usize, A)> = iter.into_iter().collect();
        UMap::from_slice(&vec)
    }
}

impl<T> Into<Vec<(usize, T)>> for UMap<T>
where
    T: Clone + PartialEq,
{
    fn into(self) -> Vec<(usize, T)> {
        self.iter().map(|(id, value)| (id, value.clone())).collect()
    }
}

impl<T> fmt::Debug for UMap<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UMap(").unwrap();
        for item in &self.vec {
            if let Some(entry) = item {
                write!(f, "{:?}", entry).unwrap();
            }
        }
        write!(f, ")").unwrap();
        Ok(())
    }
}

impl<A> Extend<(usize, A)> for UMap<A>
where
    A: Clone + PartialEq,
{
    fn extend<T: IntoIterator<Item = (usize, A)>>(&mut self, iter: T) {
        for (id, value) in iter {
            self.put(id, value);
        }
    }
}

impl<A> Extend<A> for UMap<A>
where
    A: Clone + PartialEq,
{
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        for value in iter {
            self.push(value);
        }
    }
}
