use crate::{operations::*, state::Vault};
use cvlr::{mathint::NativeInt, prelude::*};

/// Inflation attack scenario
///
/// Bob is the attacker, Alice is the victim. Bob gets some shares, donates some amount, and waits for Alice.
/// Alice purchases some shares, and losses some assets due to rounding. Bob
/// redeems shares and profits at least the total amount that Bob committed
#[rule]
pub fn rule_inflation_attack() {
    let mut vault: Vault = nondet();

    cvlr_assume!(vault.num_shares() == vault.num_assets());

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
    // -- Bob did not make profit: Bob assets did not increase
    cvlr_assert_le!(bob_assets_post, bob_assets_pre);
}

/// This rule attempts to identify the maximum loss of Alice
///
/// The scenario is
///     - Alice buys shares
///     - Alice redeems shares
///     - Alice checks balance difference
///     - Alice checks that the loss is bounded by 42
/// The rule fails. Trying other bounds shows that loss can be arbitrary.
#[rule]
pub fn rule_inflation_alice_loss_bound() {
    let mut vault: Vault = nondet();

    cvlr_assume!(vault.num_shares() <= vault.num_assets());
    clog!();
    clog!("Initial vault", vault);

    // -- Alice buys some shares
    let mut alice_assets: u64 = nondet();
    let alice_assets_pre: NativeInt = alice_assets.into();
    let effects = vault_deposit_assets(&mut vault, alice_assets).unwrap();
    alice_assets = alice_assets
        .checked_sub(effects.assets_to_vault)
        .unwrap()
        .checked_sub(effects.assets_to_fee)
        .unwrap();
    let mut alice_shares = effects.shares_to_user;
    let alice_assets_post: NativeInt = alice_assets.into();
    clog!();
    clog!(
        "Alice purchase",
        alice_assets_pre,
        alice_assets_post,
        vault,
        alice_assets,
        alice_shares
    );

    // -- Alice redeems shares
    let effects = vault_redeem_shares(&mut vault, alice_shares).unwrap();
    alice_shares = alice_shares.checked_sub(effects.shares_to_burn).unwrap();
    alice_assets = alice_assets.checked_add(effects.assets_to_user).unwrap();

    let alice_assets_post: NativeInt = alice_assets.into();

    clog!(
        "Alice redeems shares",
        vault,
        alice_shares,
        alice_assets_pre,
        alice_assets_post
    );

    // -- deposit + redeem does not make profit
    cvlr_assert_le!(alice_assets_post, alice_assets_pre);

    // -- deposit + redeem creates no loss
    // violated, can lose on rounding
    cvlr_assert_ge!(alice_assets_post + 42, alice_assets_pre);
}

/// This rule attempts to identify the maximum loss of Alice
///
/// The scenario is
///     - Alice buys shares
///     - Alice redeems shares
///     - Alice checks balance difference
///     - Alice claims that the loss is bounded by the value of a single share
///     at the beginning of the scenario
///
/// This rule is not violated. This is independent of whether virtual share
/// protection is used. Protection only harms Bob. However, Bob can still
/// harm Alice.
#[rule]
pub fn rule_inflation_max_loss() {
    let mut vault: Vault = nondet();

    cvlr_assume!(vault.num_shares() <= vault.num_assets());
    clog!();
    clog!("Initial vault", vault);

    // -- compute share price using Prover's integers
    let vault_num_shares_pre: NativeInt = vault.num_shares().into();
    let vault_num_assets_pre: NativeInt = vault.num_assets().into();
    let share_price_pre: NativeInt = vault_num_assets_pre.div_ceil(vault_num_shares_pre);

    // -- Alice buys some shares
    let mut alice_assets: u64 = nondet();
    let alice_assets_pre: NativeInt = alice_assets.into();
    let effects = vault_deposit_assets(&mut vault, alice_assets).unwrap();
    alice_assets = alice_assets
        .checked_sub(effects.assets_to_vault)
        .unwrap()
        .checked_sub(effects.assets_to_fee)
        .unwrap();
    let mut alice_shares = effects.shares_to_user;
    let alice_assets_post: NativeInt = alice_assets.into();
    clog!();
    clog!(
        "Alice purchase",
        share_price_pre,
        alice_assets_pre,
        alice_assets_post,
        vault,
        alice_assets,
        alice_shares
    );

    // -- Alice redeems shares
    let effects = vault_redeem_shares(&mut vault, alice_shares).unwrap();
    alice_shares = alice_shares.checked_sub(effects.shares_to_burn).unwrap();
    alice_assets = alice_assets.checked_add(effects.assets_to_user).unwrap();

    let alice_assets_post: NativeInt = alice_assets.into();

    clog!(
        "Alice redeems shares",
        vault,
        alice_shares,
        alice_assets_pre,
        alice_assets_post
    );

    // -- deposit + redeem does not make profit
    cvlr_assert_le!(alice_assets_post, alice_assets_pre);

    // -- loss is limited to the current share price rounded up
    cvlr_assert_ge!(alice_assets_post + share_price_pre, alice_assets_pre);

    // uncomment to check that the bound is strict
    // decreasing even by one is no longer an upper bound on loss
    // cvlr_assert_ge!(alice_assets_post + share_price_pre - 1u64.into(), alice_assets_pre);
}

/// Show impossibility of inflation attack with exact deposit
///
/// Inflation attack is only possible if Alice can has an unbounded (or large) loss.
/// No loss means no attack since Bob gets a portion of assets lost by Alice.
/// With exact deposit, Alice is deducted only the require amount (rounded up)
/// Thus, the loss is limited to one atom of rounding
#[rule]
pub fn rule_inflation_no_loss_on_exact() {
    let mut vault: Vault = nondet();

    cvlr_assume!(vault.num_shares() <= vault.num_assets());
    clog!();
    clog!("Initial vault", vault);

    // -- Alice buys some shares
    let mut alice_assets: u64 = nondet();
    let alice_assets_pre: NativeInt = alice_assets.into();
    let effects = vault_deposit_assets_exact(&mut vault, alice_assets).unwrap();
    alice_assets = alice_assets
        .checked_sub(effects.assets_to_vault)
        .unwrap()
        .checked_sub(effects.assets_to_fee)
        .unwrap();
    let mut alice_shares = effects.shares_to_user;
    let alice_assets_post: NativeInt = alice_assets.into();
    clog!();
    clog!(
        "Alice purchase",
        alice_assets_pre,
        alice_assets_post,
        vault,
        alice_assets,
        alice_shares
    );

    // -- Alice redeems shares
    let effects = vault_redeem_shares(&mut vault, alice_shares).unwrap();
    alice_shares = alice_shares.checked_sub(effects.shares_to_burn).unwrap();
    alice_assets = alice_assets.checked_add(effects.assets_to_user).unwrap();

    let alice_assets_post: NativeInt = alice_assets.into();

    clog!(
        "Alice redeems shares",
        vault,
        alice_shares,
        alice_assets_pre,
        alice_assets_post
    );

    // -- deposit + redeem does not make profit
    cvlr_assert_le!(alice_assets_post, alice_assets_pre);

    // -- loss is limited to the current share price rounded up
    cvlr_assert_ge!(alice_assets_post + 1, alice_assets_pre);

    // -- uncomment to check that accounting for rounding is required
    // cvlr_assert_ge!(alice_assets_post, alice_assets_pre);
}
