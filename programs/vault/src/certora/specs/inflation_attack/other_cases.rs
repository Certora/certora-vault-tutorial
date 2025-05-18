use crate::{operations::*, state::Vault};
use cvlr::{mathint::NativeInt, prelude::*};

/// Show that it is possible for Bob to donate, yet not lose funds
///
/// Run inflation attack scenario.
/// Force Bob to donate non-zero amount.
/// Assert that Bob did not lose any funds
///
/// This fails with a cex that shows how Bob can remain even
/// This is even when virtual shares are minted upfront
#[rule]
pub fn rule_inflation_bob_loss() {
    let mut vault: Vault = nondet();

    cvlr_assume!(vault.num_shares() == vault.num_assets());
    // -- assume some virtual shares
    cvlr_assume!(vault.num_shares() > 1);

    // -- Bob starts with some assets
    let mut bob_assets: u64 = nondet();

    // -- save original bob assets
    let bob_assets_pre: NativeInt = bob_assets.into();
    clog!("Initial state", bob_assets_pre, vault);

    // -- Bob buys some shares
    let bob_initial_deposit: u64 = nondet();
    bob_assets = bob_assets.checked_sub(bob_initial_deposit).unwrap();
    let effects = vault_deposit_assets(&mut vault, bob_initial_deposit).unwrap();
    let mut bob_shares = effects.shares_to_user;
    clog!(
        "Bob bought shares",
        vault,
        bob_initial_deposit,
        bob_shares,
        bob_assets
    );

    // -- Bob donates some assets to inflate the vault
    let bob_donate: u64 = nondet();
    // -- Force non-zero donation
    cvlr_assume!(bob_donate > 1);

    bob_assets = bob_assets.checked_sub(bob_donate).unwrap();
    let new_vault_asset_amt = vault.num_assets().checked_add(bob_donate).unwrap();
    vault_update_reward(&mut vault, new_vault_asset_amt).unwrap();
    clog!("Inflation", bob_donate, vault, bob_assets);

    // -- Alice buys some shares
    let mut alice_assets: u64 = nondet();
    let alice_assets_pre: NativeInt = alice_assets.into();
    let effects = vault_deposit_assets(&mut vault, alice_assets).unwrap();
    alice_assets = alice_assets
        .checked_sub(effects.assets_to_vault)
        .unwrap()
        .checked_sub(effects.assets_to_fee)
        .unwrap();
    let alice_shares = effects.shares_to_user;
    clog!(
        "Alice purchase",
        alice_assets_pre,
        vault,
        alice_assets,
        alice_shares
    );

    // -- Bob redeems shares
    let effects = vault_redeem_shares(&mut vault, bob_shares).unwrap();
    bob_shares = bob_shares.checked_sub(effects.shares_to_burn).unwrap();
    bob_assets = bob_assets.checked_add(effects.assets_to_user).unwrap();

    let bob_assets_post: NativeInt = bob_assets.into();
    clog!(
        "Bob redeems shares",
        vault,
        bob_shares,
        bob_assets_pre,
        bob_assets_post
    );

    // -- Bob did not profit (this succeeds)
    cvlr_assert_le!(bob_assets_post, bob_assets_pre);

    // -- Bob did lose (this fails)
    cvlr_assert_lt!(bob_assets_post, bob_assets_pre);

    // -- Therefore, Bob can cause harm, but might not profit directly from it
}
