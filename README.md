`textwidth`
===========

[![Documentation](https://docs.rs/textwidth/badge.svg)](https://docs.rs/textwidth)

A simple library to query a textwidth of a given font+size.

This is only supported on Linux and similar environments. Others are not planned
as the author does not have one. PRs welcome.

⚠️ You have to call `setup_multithreading` if you plan on using multiple `Context` in
a multi-threaded way, or if you are using `x11/xlib` in a multi-threaded fashion.

### Example

```rust
use textwidth::Context;

let ctx = Context::with_misc().unwrap();
assert!(ctx.text_width("Hello World") > 0);
```
