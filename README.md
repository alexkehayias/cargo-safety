# `cargo safety` plugin

This crate provides a subcommand for `cargo` that performs safety checks for Rust projects by finding all uses of `unsafe` code in dependencies. To do this reliably, the dependency tree is parsed by `cargo`, code is parsed into it's AST and then walked. Using Rust's wonderful pattern matching abilities we can quickly visit all places code can be `unsafe`.

## Why

In coming to Rust, many people are drawn to it's promises of safety. I was curious to see how and why people were circumventing Rust's safety guarantees through the `unsafe` escape hatch. While I don't think we should reject any libraries that use unsafety, it is good to know where exactly things are unsafe so you can make an informed decision about the many libraries you may use.

## Try it

Note: nightly build required

```
cargo install cargo-safety && cargo safety
```

Output (json):

```
[
  {
    "lib_name": "gcc",
    "status": "failed",
    "offenses": [
      {
        "occurences": "\/Users\/alexkehayias\/.cargo\/registry\/src\/github.com-1ecc6299db9ec823\/gcc-0.3.40\/src\/registry.rs:73:1: 73:29\n`unsafe impl Sync for Repr {}`\n",
        "kind": "unsafe_impl"
      }
	]
  }
]
```

## License

Copyright © 2018 Alex Kehayias

Distributed under the Eclipse Public License either version 1.0 or (at your option) any later version.
