# Certora Vault Example Tutorial

This tutorial builds a simple Tokenized Vault, inspired by the  [ERC-4626
TOKENIZED VAULT STANDARD](https://eips.ethereum.org/EIPS/eip-4626) on Ethereum.


```mermaid
flowchart TD
    subgraph TokenAccounts["Token Accounts"]
        AliceAssets["Alice assets<br/><br/>amount: 100"]
        AliceShares["Alice shares<br/><br/>amount: 75"]
        BobAssets["Bob assets<br/><br/>amount: 50"]
        BobShares["Bob shares<br/><br/>amount: 25"]
        VaultTokenAccount["Vault<br/><br/>amount: 250"]
    end

    subgraph MintAccounts["Mint Accounts"]
        VaultAssetsMint["Vault assets mint<br/><br/>supply: 10_000"]
        VaultSharesMint["Vault shares mint<br/><br/>supply: 100"]
    end
    subgraph Vault["Vault"]
    VaultX["assets: 200<br/>shares: 100"]
    end

TokenAccounts --- Vault
MintAccounts --- Vault
```

The basic operations of the vault are 
  * `deposit` -- deposit assets and obtain shares
  * `redeem` -- burn shares and obtain assets
  * `reward` -- process reward that is accumulated in the vault asset account
  * `slash` -- deduct some assets from the vault

The basic functionality is extended to include:
  * __fees__ -- fees are deducted in assets during deposit
  * __fee collection__ -- fee is optionally stored in the vault and can be claimed by a designated operator
  * __exact deposit__ -- only takes as much assets as needed to cover shares obtained by the user. This protects from an inflation attack.

Throughout the tutorial, you are invited to add the missing functionality, and formally verify correctness of your implementation. Partial implementation of the features described above is provided as a reference.

## Content

- [Installation](#installation)
- [Code structure](#code-structure)

## Installation
1. First, visit [Certora.com](https://www.certora.com) and sign up for a free account [here](https://www.certora.com/signup).
2. You will receive an email with a temporary password and a personal access key. Use the password to login to Certora following the link in the email.
3. During the workshop, we recommend following along by using Github Codespaces that you can launch from this repository.
4. Once your codespace has launched, you should have a web-based editor that looks like VSCode.
5. From the terminal of this editor, use the personal access key (from Step 2) to set a temporary environment variable like so `export CERTORAKEY=<personal_access_key>`.

## Code structure

The main logic of the vault is organized as follows. 

- The `Vault` is defined in `programs/vault/src/state.rs`

- Core vault operations are located in `programs/vault/src/operations`
 
  These operations implement the vault's asset and shares logic
  (independent of Solana runtime):

1. `vault_deposit_assets(vault, tokens_amount)`
2. `vault_redeem_shares(vault, shares_amout)`
3. `vault_update_reward(vault, new_amount)`
4. `vault_process_slash(vault, slash_amount)`

Additional functions that support fees:

- `vault_deposit_assets_with_fee(vault, tokens_amount)`
- `vault_collect_fee`


Vault functions that operates on Solana `AccountInfo` are
implemented in `programs/vault/src/processor`:

1. `process_deposit(accounts, tokens_amount)`
2. `process_redeem_shares(accounts, shares_amount)`
3. `process_update_reward(accounts, new_amount)`
4. `process_slash(accounts, slash_amount)`

Fee-based processor functions are also provided.

## DISCLAIMER
The code and examples provided in this repository are for educational purposes only. They are not production-ready and may contain bugs or security vulnerabilities. Use at your own risk.
