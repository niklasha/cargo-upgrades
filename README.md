# `cargo upgrades`

Shows which dependencies in `Cargo.toml` can be upgraded to a newer version. It's similar to [cargo-outdated](https://lib.rs/cargo-outdated), but has a simpler implementation, so it won't complain about path dependencies or potential version conflicts. Simply checks whether there is a newer (stable) version for each dependency.


## Installation

```sh
cargo install -f cargo-upgrades
```

## Usage

In in a Rust/Cargo project directory:

```sh
cargo upgrades
```

or

```sh
cargo upgrades --manifest-path=/path/to/Cargo.toml
```

## Bonus

If you have `cargo-edit` installed, you can run `cargo upgrade` (without `s`) to automatically bump all dependencies to their latest versions.
