# Vault Specification

## Overview

The vault holds one type of token, called assets. Users deposit assets and get vault shares in return. Users can later redeem their shares to get assets back. The vault also collects fees, receives rewards that add assets, and can be slashed, which removes assets.

## Properties

### Solvency (invariant)

The number of shares that the vault issued must never be greater than the number of assets that the vault holds.

In the code, the vault stores these two numbers in the `shares` and `assets` fields of `Vault` in `programs/vault/src/state.rs`.

## Scope

The entry points are the processors in `programs/vault/src/processor/`. A processor reads a Solana instruction and its accounts, and then calls a vault function. The vault and all code that the vault depends on are in scope. This includes the vault state, the vault operations, and the math helpers.

Solana operations in the processors, such as SPL token transfers and mints, are out of scope. The properties are about the internal state of `Vault` only. The token balances of on-chain accounts are not part of any property.
