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

pub struct AnyVec {
    data: Vec<u8>,
    meta: Vec<AnyMeta>,
}

impl AnyVec {
    pub fn new() -> Self {
        AnyVec {
            data: Vec::new(),
            meta: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize, avg_type_size: usize) -> Self {
        AnyVec {
            data: Vec::with_capacity(capacity * avg_type_size),
            meta: Vec::with_capacity(capacity),
        }
    }

    pub fn capacity(&self, type_size: usize) -> usize {
        cmp::min(self.meta.capacity(), self.data.capacity() / type_size)
    }

    pub fn reserve(&mut self, additional: usize, type_size: usize) {
        self.data.reserve(additional * type_size);
        self.meta.reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize, type_size: usize) {
        self.data.reserve_exact(additional * type_size);
        self.meta.reserve_exact(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(match self.meta.get(len) {
            Some(meta) => meta.data_index,
            None => return,
        });
        self.meta.truncate(len);
    }

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

    pub fn remove(&mut self, index: usize) {
        let type_size = self.meta[index].type_size;

        for _ in 0..type_size {
            self.data.remove(self.meta[index].data_index);
        }
        self.meta.remove(index);

        for i in index..self.meta.len() {
            self.meta[i].data_index -= type_size;
        }
    }

    pub fn is<T: Any>(&self, index: usize) -> Option<bool> {
        let meta = match self.meta.get(index) {
            Some(meta) => meta,
            None => return None,
        };
        Some(meta.type_id == TypeId::of::<T>())
    }

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

    pub fn push<T: Any>(&mut self, value: T) {
        let index = self.meta.len();
        self.insert(index, value);
    }

    pub fn pop<T: Any>(&mut self) -> Result<Option<T>, String> {
        let meta = match self.meta.pop() {
            Some(meta) => meta,
            None => return Ok(None),
        };
        if meta.type_id != TypeId::of::<T>() {
            Err(format!("invalid type {:?}, expected {:?}",
                        TypeId::of::<T>(),
                        &self.meta[self.meta.len() - 1].type_id))
        } else {
            unsafe {
                let element = ptr::read(&self.data[meta.data_index] as *const _ as *const T);
                let new_len = self.data.len() - meta.type_size;
                self.data.set_len(new_len);
                Ok(Some(element))
            }
        }
    }

    pub fn append(&mut self, other: &mut AnyVec) {
        let org_meta_size = self.meta.len();

        self.meta.append(&mut other.meta);
        for meta in self.meta.iter_mut().skip(org_meta_size) {
            meta.data_index += self.data.len();
        }

        self.data.append(&mut other.data);
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.meta.clear();
    }

    pub fn len(&self) -> usize {
        self.meta.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
    }

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

        vec.remove(2);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        vec.remove(1);
        assert_eq!(vec.get::<TestData>(0).unwrap().unwrap().a, 0);
        vec.remove(0);
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
