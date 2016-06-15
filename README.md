# AnyVec
[![Build Status](https://travis-ci.org/lschmierer/anyvec.svg)](https://travis-ci.org/lschmierer/anyvec)
[![Crates.io](http://meritbadge.herokuapp.com/anyvec)](https://crates.io/crates/anyvec)

[Documentation](http://lschmierer.github.io/anyvec/)

A growable list type with dynamic typing.
It can store anything that implements the `Any` trait.

About 4 to 5 times slower than `Vec<T>`.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]

anyvec = "0.2.1"
```

Usage:

```rust
extern crate anyvec;
use anyvec::AnyVec;

let mut vec = AnyVec::new();

vec.push("Test");

assert_eq!(vec.get::<&str>(0).unwrap().unwrap(), "Test");
```


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
