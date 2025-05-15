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

## Content

- [Code structure](#code-structure)
- [Specifications](#specifications)

## Code structure

The main logic of the vault is organized as follows. 

- The `Vault` is defined in `programs/vault/src/state.rs`

- Core vault operations are located in `programs/vault/src/operations`
 
  These operations implement the vault's asset and shares logic
  (independent of Solana runtime):

1. `vault_deposit_assets(tokens_amount)`
2. `vault_redeem_shares(shares_amout)`
3. `vault_update_reward(new_amount)`
4. `vault_process_slash(slash_amount)`

Additional functions that support fees:

- `vault_deposit_assets_with_fee(tokens_amount)`
- `vault_collect_fee`


Vault functions that operates on Solana `AccountInfo` are
implemented in `programs/vault/src/processor`:

1. `process_deposit(accounts, tokens_amount)`
2. `process_redeem_shares(accounts, shares_amount)`
3. `process_update_reward(accounts, new_amount)`
4. `process_slash(accounts, slash_amount)`

Fee-based processor functions are also provided.

## Specifications

All formal specifications are located in
`programs/vault/src/certora/specs`.

Notation:

- Vault assets: $\text{assets}$

- Vault shares: $\text{shares}$

- Tokens held in the SPL account: $\text{token}_{\text{amount}}$

- Total minted supply of SPL tokens: $\text{mint}_{\text{supply}}$

- fee in basis points (max = `10_000`): $\text{fee}_{\text{bps}}$
 
- fee deducted from a given operation: $\text{fee}$


### Simple properties 

Assets should not decrease:

$$ \text{assets}_{\text{post}}  \geq \text{assets}_{\text{pre}}
$$

Shares should not decrease:

$$ \text{shares}_{\text{post}}  \geq \text{shares}_{\text{pre}}
$$

Monotonicity of assets and shares 


$$\text{assets}_{\text{pre}} \leq \text{assets}_{\text{post}} \implies \text{shares}_{\text{pre}}  \leq \text{shares}_{\text{post}}$$

### Vault consistency

The vault must remain consistent with the underlying SPL accounts:

$$ 
\text{assets} \leq \text{token}_{\text{amount}} \wedge \text{shares} = \text{mint}_{\text{supply}}
$$

### No dilution 

To maintain proportional ownership (except during slashing
operations), the vault must ensure:


$$
\frac{\text{assets}_{\text{pre}}}{\text{shares}_{\text{pre}}} \leq \frac{\text{assets}_{\text{post}}}{\text{shares}_{\text{post}}}
$$

This guarantees that the asset-to-share ratio does not decrease after
any operation.

### Solvency

The vault needs to ensure the following invariant to remain solvent:


$$ \text{shares} \leq \text{assets}$$


### Fees

If a fee rate is configured, then a corresponding fee must be
deducted:

$$ \text{fee}_{\text{bps}} > 0 \implies \text{fee} > 0 
$$


### Inflation attack



## DISCLAIMER
The code and examples provided in this repository are for educational purposes only. They are not production-ready and may contain bugs or security vulnerabilities. Use at your own risk.
