# Install and setup

Running `cargo add froql` is enough to add the crate as a dependency.

For improved compilation speed during iterative development it's recommended to add the following to your `Cargo.toml`:

```toml
[profile.dev.build-override]
opt-level = 3
```

This compiles proc_macros (including froql's query macro) in release mode.
