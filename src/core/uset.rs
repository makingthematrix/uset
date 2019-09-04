#![macro_use]
use lazy_static::lazy_static;

use std::cmp;
use std::iter::FromIterator;
use std::ops::Range;
use std::ops::{Add, BitXor, Mul, Sub};

use super::umap::UMap;
use itertools::{Itertools, MinMaxResult};

/// A set of unsigned integers (usizes) implemented as a vector of booleans
/// where `vec[n - offset] == true` means that the set contains `n`. Intended for
/// handling small to medium number of identifiers.
/// Searching is O(1), addition and removal is O(1) for values within the set's
/// capacity, O(n) otherwise, as values have to be copied to a new vector.
/// The set is sorted. Getting `min` and `max` is O(1).
///
/// In all cases when values are moved to a new vector, the operation ensures that
/// the size of the new vector is `max - min`: in that case the minimum value is at vec[0]
/// (so `offset == min`) and `max - offset == capacity`. However, for performance
/// purposes, if the operation does not require new allocation, the capacity might be
/// left bigger than `max - min`.

/// Creates a `USet` with the given values.
/// Equivalent to calling [`from_slice`].
///
/// [`from_slice`]: #method.from_slice
#[allow(unused_macros)]
macro_rules! uset {
    ($($x:expr),*) => (USet::from_slice(&vec![$($x),*]))
}

#[derive(Debug, Default, Clone)]
pub struct USet {
    vec: Vec<bool>,
    len: usize,
    offset: usize,
    min: usize,
    max: usize,
}

pub struct USetIter<'a> {
    handle: &'a USet,
    index: usize,
    rindex: usize,
}

impl<'a> Iterator for USetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.handle.vec.len() - self.rindex {
            let index = self.index;
            self.index += 1;
            if self.handle.vec[index] {
                return Some(index + self.handle.offset);
            }
        }
        None
    }
}

impl<'a> DoubleEndedIterator for USetIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let len = self.handle.vec.len();
        while self.rindex < len - self.index {
            let index = len - self.rindex - 1;
            self.rindex += 1;
            if self.handle.vec[index] {
                return Some(index + self.handle.offset);
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a USet {
    type Item = usize;
    type IntoIter = USetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub const INITIAL_WORKING_CAPACITY: usize = 8;

lazy_static! {
    pub static ref EMPTY_SET: USet = USet::with_capacity(0);
}

impl USet {
    /// Constructs a new, empty `USet`.
    ///
    /// The set will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set: USet = USet::new();
    /// ```
    pub fn new() -> Self {
        EMPTY_SET.clone()
    }

    /// Constructs a new, empty `USet` with the specified capacity.
    ///
    /// The set will be able to hold exactly `capacity` elements without
    /// reallocating. If `capacity` is 0, the internal vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// *capacity* specified, the vector will have a zero *length*.
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::with_capacity(10);
    ///
    /// // The set contains no items, even though it has capacity for more
    /// assert_eq!(set.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     set.push(i);
    /// }
    ///
    /// // ...but this may make the vector reallocate
    /// set.push(11);
    /// ```
    pub fn with_capacity(size: usize) -> Self {
        USet {
            vec: vec![false; size],
            len: 0,
            offset: 0,
            min: 0,
            max: 0,
        }
    }

    /// Returns the number of elements in the set, also referred to as its 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set = USet::from_slice(&[1, 2, 3]);
    /// assert_eq!(set.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::new();
    /// assert!(set.is_empty());
    ///
    /// set.push(1);
    /// assert!(!set.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements the set can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set: USet = USet::with_capacity(10);
    /// assert_eq!(set.capacity(), 10);
    /// ```
    pub fn capacity(&self) -> usize {
        self.vec.len()
    }

    /// Shrinks the set to the minimal size able to hold given values.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 50]);
    /// assert!(set.capacity() >= 50);
    /// set.remove(1);
    /// assert!(set.capacity() >= 50);
    /// set.shrink_to_fit();
    /// assert!(set.capacity() == 1);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        // TODO: Possible performance optimization with Vec::shrink_to_fit and other in-place operations when possible
        if !self.is_empty() && (!self.vec[0] || !self.vec[self.vec.len() - 1]) {
            let mut vec = vec![false; self.max - self.min + 1];
            for id in self.min..=self.max {
                vec[id - self.min] = self.contains(id);
            }
            self.vec = vec;
            self.offset = self.min;
        } else if self.is_empty() && self.capacity() > 0 {
            self.vec = Vec::with_capacity(0);
        }
    }

    /// Shortens the set, keeping the first `len` elements and dropping the rest.
    /// If `len` is greater than the set's current length, this has no effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// This method does not shrink the set's capacity.
    /// If you want to shrink the set's capacity, call `shrink_to_fit` afterwards.
    ///
    /// # Examples
    ///
    /// Truncating a five element set to two elements:
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3, 4, 5]);
    /// set.truncate(2);
    /// assert_eq!(set, USet::from_slice(&[1, 2]));
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// set.truncate(8);
    /// assert_eq!(set, USet::from_slice(&[1, 2, 3]));
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// set.truncate(0);
    /// assert!(set.is_empty());
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
                    if *value_holder {
                        if values_left > 0 {
                            values_left -= 1;
                            new_max = index;
                        } else {
                            *value_holder = false;
                        }
                    }
                });
            self.max = new_max + self.offset;
            self.len = len;
        } else if !self.is_empty() && len == 0 {
            self.vec
                .iter_mut()
                .for_each(|value_holder| *value_holder = false);
            self.offset = 0;
            self.min = 0;
            self.max = 0;
            self.len = 0;
        }
    }

    /// Works like [`truncate`], but returns the removed elements in the form of a new set.
    /// This method does not shrink the set's capacity.
    /// If you want to shrink the set's capacity, call [`shrink_to_fit`] afterwards.
    ///
    /// # Examples
    ///
    /// Draining a five element set to two elements:
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3, 4, 5]);
    /// let drained = set.drain(2);
    /// assert_eq!(set, USet::from_slice(&[1, 2]));
    /// assert_eq!(drained, USet::from_slice(&[3, 4, 5]));
    /// ```
    ///
    /// No draining occurs when `len` is greater than the set's current length:
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// let drained = set.drain(8);
    /// assert_eq!(set, USet::from_slice(&[1, 2, 3]));
    /// assert!(drained.is_empty());
    /// ```
    ///
    /// Draining when `len == 0` is equivalent to cloning the set and calling the [`clear`]
    /// method on the original one. (but why would you want to do that?...)
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// let drained = set.drain(0);
    /// assert!(set.is_empty());
    /// assert_eq!(drained, USet::from_slice(&[1, 2, 3]));
    /// ```
    ///
    /// [`clear`]: #method.clear
    /// [`truncate`]: #method.truncate
    /// [`shrink_to_fit`]: #method.shrink_to_fit
    pub fn drain(&mut self, len: usize) -> Self {
        if !self.is_empty() && len > 0 && len < self.len {
            let mut new_set = USet::with_capacity(self.len - len);
            let mut values_left = len;
            let mut new_max = 0usize;
            let offset = self.offset;
            self.vec
                .iter_mut()
                .enumerate()
                .for_each(|(index, value_holder)| {
                    if *value_holder {
                        if values_left > 0 {
                            values_left -= 1;
                            new_max = index;
                        } else {
                            *value_holder = false;
                            new_set.push(index + offset);
                        }
                    }
                });
            self.max = new_max + self.offset;
            self.len = len;
            new_set.shrink_to_fit(); // TODO integrate with populating the vector
            new_set
        } else if !self.is_empty() && len == 0 {
            let new_set = self.clone();
            self.vec.iter_mut().for_each(|value_holder| {
                if *value_holder {
                    *value_holder = false
                }
            });
            self.offset = 0;
            self.min = 0;
            self.max = 0;
            self.len = 0;
            new_set
        } else {
            EMPTY_SET.clone()
        }
    }

    /// Clears the set, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the set.
    /// If you want to shrink the set's capacity, call [`shrink_to_fit`] afterwards.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    ///
    /// set.clear();
    ///
    /// assert!(set.is_empty());
    /// ```
    ///
    /// [`shrink_to_fit`]: #method.shrink_to_fit
    pub fn clear(&mut self) {
        self.truncate(0)
    }

    /// Changes the set's capacity, so that it can hold new elements up to the `new_capacity + offset - 1`
    /// value without reallocation. Note that `new_capacity + offset - 1` is now the largest **value**
    /// the set can hold without the reallocation, not the total number of values that can be held.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 8]);
    /// assert_eq!(8, set.capacity());
    /// set.enlarge_capacity_to(10);
    /// assert_eq!(10, set.capacity());
    /// set.push(9); // no reallocation needed
    /// assert_eq!(10, set.capacity());
    /// set.push(11); // this will trigger reallocation
    /// assert_eq!(11, set.capacity());
    /// ```
    pub fn enlarge_capacity_to(&mut self, new_capacity: usize) {
        if new_capacity > self.capacity() {
            self.vec.resize(new_capacity, false);
        }
    }

    /// Adds the id to the set, and reallocates if needed.
    /// Reallocation is not necessary if the id falls in-between the current min and max.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 3]);
    /// set.push(2);
    /// assert_eq!(set, USet::from_slice(&[1, 2, 3]));
    /// ```
    pub fn push(&mut self, id: usize) {
        match id {
            _ if self.capacity() == 0 => {
                self.vec = vec![false; INITIAL_WORKING_CAPACITY];
                self.vec[0] = true;
                self.min = id;
                self.len += 1;
                self.max = id;
                self.offset = id;
            }
            _ if self.is_empty() => {
                self.vec[0] = true;
                self.min = id;
                self.len = 1;
                self.max = id;
                self.offset = id;
            }
            _ if id < self.offset => {
                let mut vec = vec![false; self.max - id + 1];
                vec[0] = true;
                for i in self.min..=self.max {
                    vec[i - id] = self.contains(i);
                }
                self.vec = vec;
                self.len += 1;
                self.min = id;
                self.offset = id;
            }
            _ if id >= self.offset + self.capacity() => {
                self.vec.resize(id + 1 - self.offset, false);
                self.vec[id - self.offset] = true;
                self.len += 1;
                self.max = id;
            }
            _ if !self.vec[id - self.offset] => {
                self.vec[id - self.offset] = true;
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

    /// Removes the id from the set. Does nothing if the id is not in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// set.remove(2);
    /// assert_eq!(set, USet::from_slice(&[1, 3]));
    /// ```
    pub fn remove(&mut self, id: usize) {
        match id {
            _ if id < self.min || id > self.max || !self.contains(id) => {}
            _ if self.len == 1 => {
                self.vec[id - self.offset] = false;
                self.max = 0;
                self.min = 0;
                self.len = 0;
                self.offset = 0;
            }
            _ if id > self.min && id < self.max => {
                self.vec[id - self.offset] = false;
                self.len -= 1;
            }
            _ if id == self.min => {
                self.vec[id - self.offset] = false;
                self.len -= 1;
                self.min = (self.min..self.max)
                    .find(|&i| self.vec[i - self.offset])
                    .unwrap_or(self.max);
            }
            _ if id == self.max => {
                self.vec[id - self.offset] = false;
                self.len -= 1;
                self.max = (self.min..self.max)
                    .rev()
                    .find(|&i| self.vec[i - self.offset])
                    .unwrap_or(self.min);
            }
            _ => {}
        }
    }

    /// Removes all the identifiers belonging to the `other` set from `self`. Ignores identifiers
    /// from `other` which do not belong in `self`.
    /// Equivalent to calling [`remove`] multiple times. Does not reallocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set1 = USet::from_slice(&[1, 2, 3, 4]);
    /// let set2 = USet::from_slice(&[2, 3, 5]);
    /// set1.remove_all(&set2);
    /// assert_eq!(set1, USet::from_slice(&[1, 4]));
    /// ```
    ///
    /// [`remove`]: #method.remove
    pub fn remove_all(&mut self, other: &Self) {
        other.iter().for_each(|id| self.remove(id));
    }

    /// Returns true if `self` is a subset of `other`.
    /// Note that every set is a subset of itself, even if empty, and an empty set is a subset
    /// of every other set.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set1 = USet::from_slice(&[1, 2, 3]);
    /// let set2 = USet::from_slice(&[2, 3]);
    /// assert!(set2.is_subset_of(&set1));
    /// assert!(!set1.is_subset_of(&set2));
    /// assert!(set2.is_subset_of(&set2));
    ///
    /// let set3 = USet::from_slice(&[2, 3, 4]);
    /// assert!(!set1.is_subset_of(&set3));
    /// assert!(set2.is_subset_of(&set3));
    ///
    /// let set4 = USet::new();
    /// assert!(set4.is_subset_of(&set1));
    /// assert!(set4.is_subset_of(&set4));
    /// ```
    pub fn is_subset_of(&self, other: &USet) -> bool {
        if self.len > other.len {
            false
        } else {
            self.iter().find(|id| !other.contains(*id)).is_none()
        }
    }

    /// Removes and returns the element at position `index` within the set.
    /// Returns `None` if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::from_slice(&[1, 2, 3]);
    /// assert_eq!(set.pop(1), Some(2));
    /// assert_eq!(set, USet::from_slice(&[1, 3]));
    /// ```
    pub fn pop(&mut self, index: usize) -> Option<usize> {
        let d = self.at_index(index);
        if let Some(id) = d {
            self.remove(id);
        }
        d
    }

    /// Returns an iterator over the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set = USet::from_slice(&[1, 2, 4]);
    /// let mut iterator = set.iter();
    ///
    /// assert_eq!(iterator.next(), Some(1));
    /// assert_eq!(iterator.next(), Some(2));
    /// assert_eq!(iterator.next(), Some(4));
    /// assert_eq!(iterator.next(), None);
    /// ```
    pub fn iter(&self) -> USetIter {
        USetIter {
            handle: self,
            index: 0,
            rindex: 0,
        }
    }

    /// Returns `true` if the set contains the given id.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::new();
    /// set.push(1);
    /// assert_eq!(set.contains(1), true);
    /// assert_eq!(set.contains(2), false);
    /// ```
    pub fn contains(&self, id: usize) -> bool {
        id >= self.min && id <= self.max && self.vec[id - self.offset]
    }

    /// The set allows to access its values by index.
    /// It's the same as if the user created the iterator and took the n-th element.
    /// `USet` does not implement the `Index` trait because I don't even.
    ///
    ///# Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let set = USet::from_slice(&[2,3,4]);
    /// assert_eq!(set.at_index(0), Some(2));
    /// assert_eq!(set.at_index(1), Some(3));
    /// assert_eq!(set.at_index(2), Some(4));
    /// assert_eq!(set.at_index(3), None);
    /// ```
    pub fn at_index(&self, index: usize) -> Option<usize> {
        if index >= self.len {
            None
        } else {
            let mut it = self.iter();
            for _i in 0..index {
                it.next();
            }
            it.next()
        }
    }

    /// Returns the smallest element in the set or None if the set is empty.
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::new();
    /// assert_eq!(set.min(), None);
    ///
    /// set.push(2);
    /// assert_eq!(set.min(), Some(2));
    ///
    /// set.push(3);
    /// assert_eq!(set.min(), Some(2));
    ///
    /// set.push(1);
    /// assert_eq!(set.min(), Some(1));
    /// ```
    pub fn min(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }

    /// Returns the largest element in the set or None if the set is empty.
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::new();
    /// assert_eq!(set.min(), None);
    ///
    /// set.push(2);
    /// assert_eq!(set.max(), Some(2));
    ///
    /// set.push(3);
    /// assert_eq!(set.max(), Some(3));
    ///
    /// set.push(1);
    /// assert_eq!(set.max(), Some(3));
    /// ```
    pub fn max(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }

    fn make_from_slice(slice: &[usize]) -> (usize, usize, usize, Vec<bool>) {
        match slice.iter().minmax() {
            MinMaxResult::NoElements => (0, 0, 0, Vec::<bool>::new()),
            MinMaxResult::OneElement(&min) => (min, min, 1, vec![true]),
            MinMaxResult::MinMax(&min, &max) => {
                let len = slice.len();
                let capacity = cmp::max(INITIAL_WORKING_CAPACITY, max + 1 - min);
                let mut vec = vec![false; capacity];
                slice.iter().for_each(|&id| vec[id - min] = true);
                (min, max, len, vec)
            }
        }
    }

    /// Creates a set from a slice of `usize`s.
    /// This is the same as the `from_iter` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let vec = vec![2usize, 4, 5];
    /// let set = USet::from_slice(&vec);
    /// assert_eq!(vec.len(), set.len());
    /// assert!(set.contains(vec[0]));
    /// assert!(set.contains(vec[1]));
    /// assert!(set.contains(vec[2]));
    /// ```
    pub fn from_slice(slice: &[usize]) -> Self {
        if slice.is_empty() {
            EMPTY_SET.clone()
        } else {
            let (min, max, len, new_vec) = USet::make_from_slice(slice);
            USet {
                vec: new_vec,
                len,
                offset: min,
                min,
                max,
            }
        }
    }

    /// Creates a set from a range of `usize`s.
    /// This is the same as the `from_iter` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let range = 3usize..6;
    /// let set = USet::from_range(range);
    /// assert_eq!(3, set.len());
    /// assert!(set.contains(3));
    /// assert!(set.contains(4));
    /// assert!(set.contains(5));
    /// ```
    pub fn from_range(r: Range<usize>) -> Self {
        if r.len() == 0 {
            // is_empty is unstable for ranges, don't let clippy tell you otherwise
            EMPTY_SET.clone()
        } else {
            let offset = r.start;
            let max = r.end;
            let len = r.len();
            let capacity = cmp::max(INITIAL_WORKING_CAPACITY, r.len());
            let mut vec = vec![false; capacity];
            r.for_each(|id| vec[id - offset] = true);
            USet {
                vec,
                len,
                offset,
                min: offset,
                max,
            }
        }
    }

    /// Creates a set from a vector of `boolean`s.
    /// The method treats the values in the vector as markers that the index at the given value
    /// should belong to the set. In other words, `vec[n] == set.contains(n + offset)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let vec = vec![false, false, true, true, false, true];
    /// let set = USet::from_fields(vec, 1); // offset == 1
    /// assert_eq!(3, set.len());
    /// assert!(set.contains(3));
    /// assert!(set.contains(4));
    /// assert!(set.contains(6));
    /// ```
    pub fn from_fields(vec: Vec<bool>, offset: usize) -> Self {
        if vec.is_empty() {
            EMPTY_SET.clone()
        } else {
            let len = vec.iter().filter(|&b| *b).count();
            let min = vec
                .iter()
                .enumerate()
                .find_map(|(id, b)| if *b { Some(id) } else { None })
                .unwrap()
                + offset;
            let max = vec
                .iter()
                .enumerate()
                .rev()
                .find_map(|(id, b)| if *b { Some(id) } else { None })
                .unwrap()
                + offset;
            USet {
                vec,
                len,
                offset,
                min,
                max,
            }
        }
    }

    /// Adds all elements in the slice to the set.
    ///
    /// It's equivalent to calling `push` for every element or to the `extend` method over the iterator,
    /// but it will be faster if the slice contains many elements which would require reallocation.
    /// In that case, `push_all` will perform reallocation only once.
    ///
    /// # Examples
    /// ```
    /// use self::uset::core::uset::*;
    ///
    /// let mut set = USet::new();
    ///
    /// let v1 = vec![2, 4];
    /// set.push_all(&v1);
    /// assert_eq!(2, set.len());
    ///
    /// let v2 = vec![3, 5];
    /// set.push_all(&v2);
    /// assert_eq!(4, set.len());
    ///
    /// assert!(set.contains(2));
    /// assert!(set.contains(3));
    /// assert!(set.contains(4));
    /// assert!(set.contains(5));
    /// ```
    pub fn push_all(&mut self, slice: &[usize]) {
        if !slice.is_empty() {
            if self.is_empty() {
                let (min, max, len, new_vec) = USet::make_from_slice(slice);
                self.min = min;
                self.max = max;
                self.offset = min;
                self.len = len;
                self.vec = new_vec;
            } else {
                let (min, max) = match slice.iter().minmax() {
                    MinMaxResult::NoElements => (0, 0), // should not happen
                    MinMaxResult::OneElement(&min) => (min, min),
                    MinMaxResult::MinMax(&min, &max) => (min, max),
                };

                if min >= self.min && max <= self.max {
                    slice.iter().for_each(|&id| {
                        if !self.vec[id - self.offset] {
                            self.vec[id - self.offset] = true;
                            self.len += 1;
                        }
                    })
                } else {
                    let new_min = cmp::min(self.min, min);
                    let new_max = cmp::max(self.max, max);
                    let mut new_vec = vec![false; new_max - new_min + 1];
                    self.iter()
                        .skip(self.min - self.offset)
                        .take(self.max - self.min + 1)
                        .for_each(|id| new_vec[id - new_min] = true);
                    slice.iter().for_each(|&id| {
                        if !new_vec[id - new_min] {
                            new_vec[id - new_min] = true;
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

    fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            if other.is_empty() {
                EMPTY_SET.clone()
            } else {
                other.clone()
            }
        } else if other.is_empty() {
            if self.is_empty() {
                EMPTY_SET.clone()
            } else {
                self.clone()
            }
        } else {
            let min: usize = cmp::min(self.min, other.min);
            let max: usize = cmp::max(self.max, other.max);

            let mut vec = vec![false; max + 1 - min];
            let mut len = 0usize;

            vec.iter_mut().enumerate().for_each(|(id, value)| {
                if self.contains(id + min) || other.contains(id + min) {
                    *value = true;
                    len += 1;
                }
            });

            USet {
                vec,
                len,
                offset: min,
                min,
                max,
            }
        }
    }

    fn difference(&self, other: &USet) -> Self {
        let mut vec = self.vec.clone();
        let mut len = self.len;

        other.iter().for_each(|id| {
            if self.contains(id) {
                vec[id - self.offset] = false;
                len -= 1;
            }
        });

        if len == 0 {
            EMPTY_SET.clone()
        } else {
            let min = vec
                .iter()
                .enumerate()
                .find_map(|(id, b)| if *b { Some(id) } else { None })
                .unwrap()
                + self.offset;
            let max = vec
                .iter()
                .enumerate()
                .rev()
                .find_map(|(id, b)| if *b { Some(id) } else { None })
                .unwrap()
                + self.offset;
            USet {
                vec,
                len,
                offset: self.offset,
                min,
                max,
            }
        }
    }

    fn common_part(&self, other: &USet) -> Self {
        if self.is_empty() || other.is_empty() {
            EMPTY_SET.clone()
        } else {
            let rough_range = cmp::max(self.min, other.min)..=cmp::min(self.max, other.max);
            let mn = rough_range
                .clone()
                .find(|&id| self.contains(id) && other.contains(id));
            let mx = rough_range
                .clone()
                .rev()
                .find(|&id| self.contains(id) && other.contains(id));
            if let Some(min) = mn {
                if let Some(max) = mx {
                    let mut vec = vec![false; max + 1 - min];
                    let mut len = 0usize;
                    for id in min..=max {
                        if self.contains(id) && other.contains(id) {
                            vec[id - min] = true;
                            len += 1;
                        }
                    }
                    USet {
                        vec,
                        len,
                        offset: min,
                        min,
                        max,
                    }
                } else {
                    EMPTY_SET.clone()
                }
            } else {
                EMPTY_SET.clone()
            }
        }
    }

    fn xor_set(&self, other: &USet) -> Self {
        if self.is_empty() && other.is_empty() {
            EMPTY_SET.clone()
        } else if self.is_empty() {
            other.clone()
        } else if other.is_empty() {
            self.clone()
        } else {
            let rough_range = cmp::min(self.min, other.min)..=cmp::max(self.max, other.max);
            let mn = rough_range.clone().find(|&id| {
                (self.contains(id) && !other.contains(id))
                    || (!self.contains(id) && other.contains(id))
            });
            let mx = rough_range.clone().rev().find(|&id| {
                (self.contains(id) && !other.contains(id))
                    || (!self.contains(id) && other.contains(id))
            });
            if let Some(min) = mn {
                if let Some(max) = mx {
                    let mut vec = vec![false; max + 1 - min];
                    let mut len = 0usize;
                    for id in min..=max {
                        if (self.contains(id) && !other.contains(id))
                            || (!self.contains(id) && other.contains(id))
                        {
                            vec[id - min] = true;
                            len += 1;
                        }
                    }
                    USet {
                        vec,
                        len,
                        offset: min,
                        min,
                        max,
                    }
                } else {
                    EMPTY_SET.clone()
                }
            } else {
                EMPTY_SET.clone()
            }
        }
    }
}

impl PartialEq for USet {
    fn eq(&self, other: &USet) -> bool {
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
                .all(|(&a, &b)| a == b)
    }
}

impl Eq for USet {}

impl<'a> Add for &'a USet {
    type Output = USet;
    fn add(self, other: &USet) -> USet {
        self.union(other)
    }
}

impl<'a> Sub for &'a USet {
    type Output = USet;
    fn sub(self, other: &USet) -> USet {
        self.difference(other)
    }
}

impl<'a> Mul for &'a USet {
    type Output = USet;
    fn mul(self, other: &USet) -> USet {
        self.common_part(other)
    }
}

impl<'a> BitXor for &'a USet {
    type Output = USet;
    fn bitxor(self, other: &USet) -> USet {
        self.xor_set(other)
    }
}

impl<'a> From<&'a [usize]> for USet {
    fn from(slice: &'a [usize]) -> Self {
        USet::from_slice(slice)
    }
}

impl From<Vec<usize>> for USet {
    fn from(vec: Vec<usize>) -> Self {
        USet::from_slice(&vec)
    }
}

impl Into<Vec<usize>> for USet {
    fn into(self) -> Vec<usize> {
        self.iter().collect()
    }
}

impl<T> From<UMap<T>> for USet
where
    T: Clone + PartialEq,
{
    fn from(map: UMap<T>) -> Self {
        map.keys()
    }
}

impl<'a> From<&'a Vec<usize>> for USet {
    fn from(vec: &'a Vec<usize>) -> Self {
        USet::from_slice(vec)
    }
}

impl From<Range<usize>> for USet {
    fn from(r: Range<usize>) -> Self {
        USet::from_range(r)
    }
}

impl FromIterator<usize> for USet {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let vec: Vec<usize> = iter.into_iter().collect();
        USet::from_slice(&vec)
    }
}

impl Extend<usize> for USet {
    fn extend<T: IntoIterator<Item = usize>>(&mut self, iter: T) {
        for id in iter {
            self.push(id);
        }
    }
}
