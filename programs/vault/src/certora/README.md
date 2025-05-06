# Certora Specification Integration Guide

This guide explains how to integrate Certora specifications into a Solana smart contract project using the Certora Solana Spec Template.

## Overview

This directory contains formal verification specifications and all supporting files needed to verify Solana programs using Certora's tools.

## Step-by-Step Installation

### 1. Clone the Specification Template

Navigate to your package's `src` directory and clone the Certora spec template repository under the name `certora`.

For example, if your package is in `WORKSPACE_ROOT/programs/contract`:

```sh
cd WORKSPACE_ROOT/programs/contract/src
git clone https://github.com/Certora/solana-spec-template certora
```

### 2. Run the Setup Script

Use the provided setup script to configure the specifications for your workspace and package.

```sh
cd certora
python certora-setup.py --workspace WORKSPACE_ROOT --package-name contract
```

### 3. Apply the Configuration

If everything looks good, re-run the setup script with the `--execute` flag to apply the changes:

```sh
python certora-setup.py --workspace WORKSPACE_ROOT --package-name contract --execute
```

### 4. Enable Specification Use in Code

To include the specifications in your Rust crate, add the following to your `lib.rs`:

```rust
pub mod certora;
```

And add `certora` feature under `[features]` in `Cargo.toml` for the package

```toml
[features]
...
certora = ["no-entrypoint", "dep:cvlr", "dep:cvlr-solana"]
```

This makes the Certora specification code available during compilation (when the `certora` feature is enabled).

## Notes

- Make sure to enable the `certora` feature in your `Cargo.toml` to include dependencies and settings necessary for formal verification.
- Specification files are expected to be under `src/certora/specs`.
- Adjust `solana_inlining.txt` and `solana_summaries.txt` files in the `envs` folder to fine-tune inlining and function summaries for verification.