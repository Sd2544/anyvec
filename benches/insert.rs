#![feature(test)]

extern crate test;
use test::Bencher;

extern crate anyvec;
use anyvec::*;

#[bench]
fn insert_front(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = AnyVec::new();
        for _ in 0..1000 {
            vec.insert(0, "Test");
        }
    });
}

#[bench]
fn insert_middle(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = AnyVec::new();
        for i in 0..1000 {
            vec.insert(i / 2, "Test");
        }
    });
}

#[bench]
fn push(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = AnyVec::new();
        for _ in 0..1000 {
            vec.push("Test");
        }
    });
}

#[bench]
fn std_vec_insert_front(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::new();
        for _ in 0..1000 {
            vec.insert(0, "Test");
        }
    });
}
#[bench]
fn std_vec_insert_middle(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::new();
        for i in 0..1000 {
            vec.insert(i / 2, "Test");
        }
    });
}

#[bench]
fn std_vec_push(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::new();
        for _ in 0..1000 {
            vec.push("Test");
        }
    });
}
