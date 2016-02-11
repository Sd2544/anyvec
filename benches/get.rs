#![feature(test)]

extern crate test;
use test::Bencher;

extern crate anyvec;
use anyvec::*;

#[bench]
fn get(b: &mut Bencher) {
    let mut vec = AnyVec::new();
    for _ in 0..1000 {
        vec.push("Test");
    }
    b.iter(|| {
        for i in 0..vec.len() {
            test::black_box(vec.get::<&str>(i).unwrap().unwrap().len());
        }
    });
}

#[bench]
fn std_vec_get(b: &mut Bencher) {
    let mut vec = Vec::new();
    for _ in 0..1000 {
        vec.push("Test");
    }
    b.iter(|| {
        for i in 0..vec.len() {
            test::black_box(vec.get(i).unwrap().len());
        }
    });
}
