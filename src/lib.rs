// Copyright 2016 anyvec Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! A growable list type with dynamic typing.
//!
//! It can store anything that implements the `Any` trait.

#![doc(html_root_url = "http://lschmierer.github.io/anyvec/")]

use std::result::Result;
use std::cmp;
use std::any::{Any, TypeId};
use std::mem;
use std::ptr;

struct AnyMeta {
    data_index: usize,
    type_id: TypeId,
    type_size: usize,
}

/// A growable list type with dynamic typing.
///
/// It can store anything that implements the `Any` trait.
pub struct AnyVec {
    data: Vec<u8>,
    meta: Vec<AnyMeta>,
}

impl AnyVec {
    /// Constructs a new, empty `AnyVec`.
    pub fn new() -> Self {
        AnyVec {
            data: Vec::new(),
            meta: Vec::new(),
        }
    }

    /// Constructs a new, empty `AnyVec` with specified capacity.
    ///
    /// Since we do not type sizes ahead, an average type size `avg_type_size` must be specified.
    pub fn with_capacity(capacity: usize, avg_type_size: usize) -> Self {
        AnyVec {
            data: Vec::with_capacity(capacity * avg_type_size),
            meta: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of elements the vector can hold without reallocating.
    pub fn capacity(&self, type_size: usize) -> usize {
        cmp::min(self.meta.capacity(), self.data.capacity() / type_size)
    }

    /// Reserves capacity for at least `additional` more elements.
    ///
    /// Since we do not type sizes ahead, an average type size `avg_type_size` must be specified.
    ///
    /// # Panics
    /// Panics if the new capacity overflows `usize`.
    pub fn reserve(&mut self, additional: usize, avg_type_size: usize) {
        self.data.reserve(additional * avg_type_size);
        self.meta.reserve(additional);
    }

    /// Reserves capacity for exactly `additional` more elements.
    ///
    /// Since we do not type sizes ahead, an average type size `avg_type_size` must be specified.
    ///
    /// # Panics
    /// Panics if the new capacity overflows `usize`.
    pub fn reserve_exact(&mut self, additional: usize, avg_type_size: usize) {
        self.data.reserve_exact(additional * avg_type_size);
        self.meta.reserve_exact(additional);
    }

    /// Shrinks the capacity of the vector as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    /// Shortens the vector to be `len` elements long.
    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(match self.meta.get(len) {
            Some(meta) => meta.data_index,
            None => return,
        });
        self.meta.truncate(len);
    }

    /// Inserts an element at position `index` in the vector.
    ///
    /// Shifts elements after position `index` to the right.
    ///
    /// # Panics
    /// Panics if `index` is greater than the vector's length.
    pub fn insert<T: Any>(&mut self, index: usize, element: T) {
        let type_id = TypeId::of::<T>();
        let type_size = mem::size_of::<T>();

        let data_index = match self.meta.get(index) {
            Some(meta) => meta.data_index,
            None => self.data.len(),
        };

        for i in index..self.meta.len() {
            self.meta[i].data_index += type_size;
        }
        self.meta.insert(index,
                         AnyMeta {
                             data_index: data_index,
                             type_id: type_id,
                             type_size: type_size,
                         });

        self.data.reserve(type_size);

        unsafe {
            ptr::copy(self.data.as_mut_ptr().offset(data_index as isize),
                      self.data.as_mut_ptr().offset((data_index + type_size) as isize),
                      self.data.len() - data_index);
            ptr::copy(&element as *const _ as *const _,
                      self.data.as_mut_ptr().offset(data_index as isize),
                      type_size);
            let new_len = self.data.len() + type_size;
            self.data.set_len(new_len);
        }
    }

    /// Removes and returns the element at position `index`.
    ///
    /// Shifts elements after position `index` to the left.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn remove<T: Any>(&mut self, index: usize) -> Result<T, String> {
        let type_id = self.meta[index].type_id;
        let type_size = self.meta[index].type_size;
        let data_index = self.meta[index].data_index;

        if type_id != TypeId::of::<T>() {
            return Err(format!("invalid type {:?}, expected {:?}",
                               TypeId::of::<T>(),
                               &self.meta[self.meta.len() - 1].type_id));
        }

        self.meta.remove(index);
        for i in index..self.meta.len() {
            self.meta[i].data_index -= type_size;
        }

        unsafe {
            let mut vec = Vec::with_capacity(type_size);

            ptr::copy(self.data.as_mut_ptr().offset(data_index as isize),
                      vec.as_mut_ptr(),
                      type_size);
            ptr::copy(self.data.as_mut_ptr().offset((data_index + type_size) as isize),
                      self.data.as_mut_ptr().offset(data_index as isize),
                      self.data.len() - (data_index + type_size));
            let new_len = self.data.len() - type_size;
            self.data.set_len(new_len);

            Ok(ptr::read(vec.as_ptr() as *const T))
        }

    }

    /// Returns if element at position `index` is of type `T`,
    /// or `None` if the index is out of bounds.
    pub fn is<T: Any>(&self, index: usize) -> Option<bool> {
        let meta = match self.meta.get(index) {
            Some(meta) => meta,
            None => return None,
        };
        Some(meta.type_id == TypeId::of::<T>())
    }

    /// Returns element at position `index` or `None` if the index is out of bounds.
    pub fn get<T: Any>(&self, index: usize) -> Result<Option<&T>, String> {
        let meta = match self.meta.get(index) {
            Some(meta) => meta,
            None => return Ok(None),
        };
        if meta.type_id != TypeId::of::<T>() {
            Err(format!("invalid type {:?}, expected {:?}",
                        TypeId::of::<T>(),
                        meta.type_id))
        } else {
            unsafe { Ok(Some(ptr::read(&&self.data[meta.data_index] as *const _ as *const &T))) }
        }
    }

    /// Returns mutable reference to element at position `index`,
    /// or `None` if the index is out of bounds.
    pub fn get_mut<T: Any>(&self, index: usize) -> Result<Option<&mut T>, String> {
        let meta = match self.meta.get(index) {
            Some(meta) => meta,
            None => return Ok(None),
        };
        if meta.type_id != TypeId::of::<T>() {
            Err(format!("invalid type {:?}, expected {:?}",
                        TypeId::of::<T>(),
                        meta.type_id))
        } else {
            unsafe {
                Ok(Some(ptr::read(&&self.data[meta.data_index] as *const _ as *const &mut T)))
            }
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    /// Panics if the number of elements in the vector overflows a `usize`.
    pub fn push<T: Any>(&mut self, value: T) {
        let index = self.meta.len();
        self.insert(index, value);
    }

    /// Returns the last element of the vector, or `None` if it is empty.
    pub fn pop<T: Any>(&mut self) -> Result<Option<T>, String> {
        if self.is_empty() {
            Ok(None)
        } else {
            let index = self.meta.len() - 1;
            match self.remove(index) {
                Ok(element) => Ok(Some(element)),
                Err(err) => Err(err),
            }
        }
    }

    /// Moves all the elements of `other` into `Self`, leaving `other` empty.
    ///
    /// # Panics
    /// Panics if the number of elements in the vector overflows a `usize`.
    pub fn append(&mut self, other: &mut AnyVec) {
        let org_meta_size = self.meta.len();

        self.meta.append(&mut other.meta);
        for meta in self.meta.iter_mut().skip(org_meta_size) {
            meta.data_index += self.data.len();
        }

        self.data.append(&mut other.data);
    }

    /// Clears the vector.
    pub fn clear(&mut self) {
        self.data.clear();
        self.meta.clear();
    }

    /// Returns the number of elements in the vector.
    pub fn len(&self) -> usize {
        self.meta.len()
    }

    /// Returns if the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
    }

    /// Splits the collection into two at the given index.
    ///
    /// # Panics
    /// Panics if `at > len`.
    pub fn split_off(&mut self, at: usize) -> Self {
        let other_data = self.data.split_off(self.meta[at].data_index);
        let mut other_meta = self.meta.split_off(at);

        for meta in other_meta.iter_mut() {
            meta.data_index -= self.data.len();
        }

        AnyVec {
            data: other_data,
            meta: other_meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    struct TestData<'a> {
        a: u64,
        b: &'a str,
    }

    #[test]
    fn capacity() {
        assert_eq!(AnyVec::with_capacity(8, 64).capacity(64), 8);
        assert_eq!(AnyVec::with_capacity(8, 64).capacity(32), 8);
        assert_eq!(AnyVec::with_capacity(16, 64).capacity(64), 16);
        assert_eq!(AnyVec::with_capacity(16, 32).capacity(64), 8);
        assert_eq!(AnyVec::with_capacity(8, 20).capacity(16), 8);
        assert_eq!(AnyVec::with_capacity(8, 16).capacity(20), 6);
    }

    #[test]
    fn reserve() {
        let mut vec = AnyVec::new();
        vec.reserve(8, 64);
        assert!(vec.capacity(64) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve(8, 64);
        assert!(vec.capacity(32) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve(16, 64);
        assert!(vec.capacity(64) >= 16);
        let mut vec = AnyVec::new();
        vec.reserve(16, 32);
        assert!(vec.capacity(64) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve(8, 20);
        assert!(vec.capacity(16) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve(8, 16);
        assert!(vec.capacity(20) >= 6);
    }

    #[test]
    fn reserve_exact() {
        let mut vec = AnyVec::new();
        vec.reserve_exact(8, 64);
        assert!(vec.capacity(64) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve_exact(8, 64);
        assert!(vec.capacity(32) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve_exact(16, 64);
        assert!(vec.capacity(64) >= 16);
        let mut vec = AnyVec::new();
        vec.reserve_exact(16, 32);
        assert!(vec.capacity(64) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve_exact(8, 20);
        assert!(vec.capacity(16) >= 8);
        let mut vec = AnyVec::new();
        vec.reserve_exact(8, 16);
        assert!(vec.capacity(20) >= 6);
    }

    #[test]
    fn shrink_to_fit() {
        let mut vec = AnyVec::with_capacity(4, 1);
        vec.push(0 as u8);
        vec.push(1 as u8);
        vec.shrink_to_fit();
        assert_eq!(vec.capacity(1), 2);

        let mut vec = AnyVec::with_capacity(8, 2);
        vec.push(0 as u16);
        vec.push(1 as u16);
        vec.push(2 as u16);
        vec.shrink_to_fit();
        assert_eq!(vec.capacity(2), 3);

        let mut vec = AnyVec::with_capacity(8, mem::size_of::<TestData>());
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 1, b: "Test" });
        vec.shrink_to_fit();
        assert_eq!(vec.capacity(mem::size_of::<TestData>()), 2);
    }

    #[test]
    fn truncate() {
        let mut vec = AnyVec::new();
        vec.push(0);
        vec.push(1);
        vec.push(2);
        vec.push(3);
        vec.truncate(2);
        assert_eq!(vec.len(), 2);

        let mut vec = AnyVec::new();
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 1, b: "Test" });
        vec.push(TestData { a: 2, b: "Test" });
        vec.push(TestData { a: 3, b: "Test" });
        vec.truncate(3);
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn insert() {
        let mut vec = AnyVec::new();
        vec.insert(0, TestData { a: 1, b: "Test" });
        vec.insert(1, TestData { a: 2, b: "Test" });
        vec.insert(0, TestData { a: 0, b: "Test" });
        vec.insert(3, TestData { a: 3, b: "Test" });
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().b, "Test");
        assert_eq!(vec.get::<TestData>(1).unwrap().unwrap().a, 1);
        assert_eq!(vec.get::<TestData>(2).unwrap().unwrap().a, 2);
        assert_eq!(vec.get::<TestData>(3).unwrap().unwrap().a, 3);
    }

    #[test]
    fn remove() {
        let mut vec = AnyVec::new();
        vec.insert(0, TestData { a: 1, b: "Test" });
        vec.insert(1, TestData { a: 2, b: "Test" });
        vec.insert(0, TestData { a: 0, b: "Test" });
        vec.insert(3, TestData { a: 3, b: "Test" });

        assert_eq!(vec.remove::<TestData>(2).unwrap().a, 2);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        assert_eq!(vec.remove::<TestData>(1).unwrap().a, 1);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        assert_eq!(vec.remove::<TestData>(0).unwrap().a, 0);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 3);
    }

    #[test]
    fn is() {
        let mut vec = AnyVec::new();
        vec.push(TestData { a: 0, b: "Test" });
        vec.push("Test");
        vec.push(0 as u8);

        assert!(vec.is::<TestData>(0).unwrap());
        assert!(vec.is::<&str>(1).unwrap());
        assert!(!vec.is::<TestData>(1).unwrap());
        assert!(vec.is::<u8>(2).unwrap());
    }

    #[test]
    fn get() {
        let mut vec = AnyVec::new();
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 0, b: "Test" });

        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        vec.get_mut::<TestData>(0).unwrap().unwrap().a += 1;
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 1);
        assert_eq!(vec.get::<TestData>(2).unwrap().unwrap().a, 0);
    }

    #[test]
    fn push_pop() {
        let mut vec = AnyVec::new();
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 1, b: "Test" });
        vec.push(TestData { a: 2, b: "Test" });

        assert_eq!(vec.pop::<TestData>().unwrap().unwrap().a, 2);

        vec.push(TestData { a: 3, b: "Test" });

        assert_eq!(vec.pop::<TestData>().unwrap().unwrap().a, 3);
        assert_eq!(vec.pop::<TestData>().unwrap().unwrap().a, 1);
        assert_eq!(vec.pop::<TestData>().unwrap().unwrap().a, 0);
    }

    #[test]
    fn append() {
        let mut vec1 = AnyVec::new();
        vec1.push(TestData { a: 0, b: "Test" });
        vec1.push(TestData { a: 1, b: "Test" });
        vec1.push(TestData { a: 2, b: "Test" });

        let mut vec2 = AnyVec::new();
        vec2.push(TestData { a: 3, b: "Test" });
        vec2.push(TestData { a: 4, b: "Test" });
        vec2.push(TestData { a: 5, b: "Test" });
        vec2.push("Test");

        vec1.append(&mut vec2);
        for i in 0..6 {
            assert_eq!(vec1.get::<TestData>(i).unwrap().unwrap().a, i as u64);
        }
        assert!(vec1.is::<&str>(6).unwrap());
    }

    #[test]
    fn clear() {
        let mut vec = AnyVec::new();
        vec.push(TestData { a: 0, b: "Test" });
        vec.push(TestData { a: 1, b: "Test" });
        vec.push(TestData { a: 2, b: "Test" });
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }

    #[test]
    fn split_off() {
        let mut vec1 = AnyVec::new();
        vec1.push(TestData { a: 0, b: "Test" });
        vec1.push(TestData { a: 1, b: "Test" });
        vec1.push(TestData { a: 2, b: "Test" });
        vec1.push(TestData { a: 3, b: "Test" });
        vec1.push(TestData { a: 4, b: "Test" });
        vec1.push(TestData { a: 5, b: "Test" });

        let vec2 = vec1.split_off(4);
        assert_eq!(vec1.len(), 4);
        assert_eq!(vec2.len(), 2);
        assert_eq!(vec2.get::<TestData>(0).unwrap().unwrap().a, 4);
    }
}
