# Harbor

Harbor provides safety checks for Rust crates by finding all uses of `unsafe` code. To do this reliably, Rust code is parsed into it's AST and then walked. Using Rust's wonderful pattern matching abilities we can quickly visit all places code can be `unsafe`.

## Why

In coming to Rust, many people are drawn to it's promises of safety. I was curious to see how and why people were circumventing Rust's safety guarantees through the `unsafe` escape hatch. While I don't think we should reject any libraries that use unsafety, it is good to know where exactly things are unsafe so you can make an informed decision about the many libraries you may use.

## Try it

Example `curl`:

```
curl -H "Content-Type: application/json" \
  -X POST \
  https://vbdhx3mx0j.execute-api.us-west-2.amazonaws.com/v1/safety \
  -d "{\"git-url\": \"https://github.com/SergioBenitez/Rocket\"}"
```

Output:

```
{
    "repo_url": "https://github.com/SergioBenitez/Rocket",
    "status": "failed",
    "offenses": [
        {
            "kind": "unsafe_block",
            "occurences": "/tmp/rocket/codegen/src/decorators/derive_form.rs:36:29: 36:75\n`unsafe { transmute(&*lifetime.name.as_str()) }`\n"
        },
        {
            "kind": "unsafe_function",
            "occurences": "/tmp/rocket/lib/src/config/mod.rs:317:1: 343:2\n`unsafe fn private_init() {\n    let bail = |e: Con...`\n"
        },
        {
            "kind": "unsafe_block",
            "occurences": "/tmp/rocket/lib/src/config/mod.rs:351:5: 351:51\n`unsafe { CONFIG.as_ref().map(|c| c.active()) }`\n"
        },
        {
            "kind": "unsafe_block",
            "occurences": "/tmp/rocket/lib/src/config/mod.rs:307:5: 314:6\n`unsafe {\n        INIT.call_once(|| {\n            ...`\n"
        }
    ]
}

```

Usage with `cargo`:

```
rustup run nightly cargo run <git url> <optional commit sha>
```

## What's next?

### Badges!!

Want other's to know your code is safe? I'm working on a way to integrate a safety badge right into your README.md. Please get in touch on Twitter @alexkehayias if you are interested in adding one to your GitHub project.

Example:

[![safety badge](https://img.shields.io/badge/safe%20code-100%25-green.svg)]()

## License

Copyright © 2017 Alex Kehayias

Distributed under the Eclipse Public License either version 1.0 or (at your option) any later version.
