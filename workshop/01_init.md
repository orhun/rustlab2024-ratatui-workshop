# Initialization

Let's start by creating a new Rust project in the workspace:

```sh
cargo new tui
```

This will create a new directory named `tui` with the following structure:

```
tui
├── Cargo.toml
└── src
    └── main.rs
```

`tui/Cargo.toml` is the manifest file for the project where we will be defining the dependencies and other configurations. Go and take a look at it :)

Also, the top-level `Cargo.toml` in the workspace should now look like this:

```toml
[workspace]
resolver = "2"
members = ["common", "server", "tui"]
```

Since our new `tui` project is the member of the workspace now, we can try running it via:

```sh
cargo run -p tui
```

If everything goes right, you should see the output `Hello, world!` in the terminal. Voila! You have just created a Rust project.

But don't get too excited, we have a long way to go.
