//! Differential behavioural-equivalence tests: every translated target (and the helpers that
//! were rewritten) is run on this extraction crate AND on the ORIGINAL program
//! (`certora_vault_tutorial`, with its `PodU64`/Pubkey-laden `Vault`) over the same inputs, and
//! the outcomes are compared: `Ok(effect)` / `Err(variant)` plus the post-state of the four
//! projected fields (shares, assets, fee_bps, fee_amount) — including on the `Err` path, so any
//! partial in-place mutation must also agree.

use crate::operations as ext_ops;
use crate::state::Vault as ExtVault;
use crate::VaultError as ExtErr;

use original::operations as orig_ops;
use original::state::Vault as OrigVault;
use original::VaultError as OrigErr;

type Effect = (u64, u64, u64, u64, u64);
type State = (u64, u64, u64, u64);
type Outcome = (Result<Effect, u8>, State);

fn ext_err(e: ExtErr) -> u8 {
    match e {
        ExtErr::Unspecified => 0,
        ExtErr::MathOverflow => 1,
        ExtErr::GuardFail => 2,
        ExtErr::SelfTransfer => 3,
    }
}

fn orig_err(e: OrigErr) -> u8 {
    match e {
        OrigErr::Unspecified => 0,
        OrigErr::MathOverflow => 1,
        OrigErr::GuardFail => 2,
        OrigErr::SelfTransfer => 3,
    }
}

fn mk_ext(s: State) -> ExtVault {
    ExtVault {
        shares: s.0,
        assets: s.1,
        fee_bps: s.2,
        fee_amount: s.3,
    }
}

fn mk_orig(s: State) -> OrigVault {
    OrigVault {
        shares: s.0.into(),
        assets: s.1.into(),
        fee_bps: s.2.into(),
        fee_amount: s.3.into(),
        ..Default::default()
    }
}

fn ext_state(v: &ExtVault) -> State {
    (v.shares, v.assets, v.fee_bps, v.fee_amount)
}

fn orig_state(v: &OrigVault) -> State {
    (
        u64::from(v.shares),
        u64::from(v.assets),
        u64::from(v.fee_bps),
        u64::from(v.fee_amount),
    )
}

fn ext_run(
    s: State,
    f: impl Fn(&mut ExtVault) -> crate::VaultResult<ext_ops::VaultEffect>,
) -> Outcome {
    let mut v = mk_ext(s);
    let r = f(&mut v)
        .map(|e| {
            (
                e.shares_to_burn,
                e.shares_to_user,
                e.assets_to_vault,
                e.assets_to_user,
                e.assets_to_fee,
            )
        })
        .map_err(ext_err);
    (r, ext_state(&v))
}

fn orig_run(
    s: State,
    f: impl Fn(&mut OrigVault) -> original::VaultResult<orig_ops::VaultEffect>,
) -> Outcome {
    let mut v = mk_orig(s);
    let r = f(&mut v)
        .map(|e| {
            (
                e.shares_to_burn,
                e.shares_to_user,
                e.assets_to_vault,
                e.assets_to_user,
                e.assets_to_fee,
            )
        })
        .map_err(orig_err);
    (r, orig_state(&v))
}

/// Run all eight target operations + `is_solvent` on one input and compare.
fn check_all(s: State, amt: u64) {
    macro_rules! same {
        ($name:literal, $ext:expr, $orig:expr) => {{
            let a = ext_run(s, $ext);
            let b = orig_run(s, $orig);
            assert_eq!(a, b, "{} diverges on state={:?} amt={}", $name, s, amt);
        }};
    }
    same!(
        "vault_deposit_assets",
        |v| ext_ops::vault_deposit_assets(v, amt),
        |v| orig_ops::vault_deposit_assets(v, amt)
    );
    same!(
        "vault_redeem_shares",
        |v| ext_ops::vault_redeem_shares(v, amt),
        |v| orig_ops::vault_redeem_shares(v, amt)
    );
    same!(
        "vault_update_reward",
        |v| ext_ops::vault_update_reward(v, amt),
        |v| orig_ops::vault_update_reward(v, amt)
    );
    same!(
        "vault_process_slash",
        |v| ext_ops::vault_process_slash(v, amt),
        |v| orig_ops::vault_process_slash(v, amt)
    );
    same!(
        "vault_deposit_assets_with_fee",
        |v| ext_ops::vault_deposit_assets_with_fee(v, amt),
        |v| orig_ops::vault_deposit_assets_with_fee(v, amt)
    );
    same!(
        "vault_collect_fee",
        |v| ext_ops::vault_collect_fee(v),
        |v| orig_ops::vault_collect_fee(v)
    );
    same!(
        "vault_deposit_assets_exact",
        |v| ext_ops::vault_deposit_assets_exact(v, amt),
        |v| orig_ops::vault_deposit_assets_exact(v, amt)
    );
    same!(
        "vault_deposit_assets_with_fee_exact",
        |v| ext_ops::vault_deposit_assets_with_fee_exact(v, amt),
        |v| orig_ops::vault_deposit_assets_with_fee_exact(v, amt)
    );
    assert_eq!(
        mk_ext(s).is_solvent(),
        mk_orig(s).is_solvent(),
        "is_solvent diverges on {:?}",
        s
    );
    assert_eq!(
        mk_ext(s).check_invariant().map_err(ext_err),
        mk_orig(s).check_invariant().map_err(orig_err),
        "check_invariant diverges on {:?}",
        s
    );
}

const EDGES: [u64; 15] = [
    0,
    1,
    2,
    3,
    7,
    100,
    101,
    9_999,
    10_000,
    10_001,
    1 << 32,
    u64::MAX / 2,
    u64::MAX / 2 + 1,
    u64::MAX - 1,
    u64::MAX,
];

const FEES: [u64; 8] = [0, 1, 500, 5_000, 9_999, 10_000, 10_001, u64::MAX];

/// Exhaustive cross product over the edge values: shares==assets (identity conversion,
/// incl. 0==0), shares==0 with assets>0 (div by zero), insolvent states, amount 0, u64::MAX
/// (add overflow, quotient > u64::MAX), fee_bps at/around 10000.
#[test]
fn ops_agree_on_edge_grid() {
    for &sh in EDGES.iter() {
        for &as_ in EDGES.iter() {
            for &fb in FEES.iter() {
                for &fa in [0u64, 5, u64::MAX].iter() {
                    for &amt in EDGES.iter() {
                        check_all((sh, as_, fb, fa), amt);
                    }
                }
            }
        }
    }
}

/// xorshift64* — deterministic pseudo-random inputs (no external RNG crate).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Mix of magnitudes so that small vaults, near-boundary and huge values all occur.
    fn val(&mut self) -> u64 {
        let r = self.next();
        match r % 6 {
            0 => r >> 60,              // tiny
            1 => (r >> 40) % 1_000,    // small
            2 => r >> 32,              // ~u32
            3 => u64::MAX - (r >> 56), // near max
            4 => EDGES[(r >> 8) as usize % EDGES.len()],
            _ => self.next(), // full range
        }
    }
}

#[test]
fn ops_agree_on_random_inputs() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..300_000 {
        let sh = rng.val();
        // bias towards solvent vaults (the properties' precondition) but keep insolvent ones too
        let as_ = if rng.next() % 4 == 0 {
            rng.val()
        } else {
            sh.saturating_add(rng.val() >> (rng.next() % 64))
        };
        let fb = if rng.next() % 2 == 0 {
            rng.next() % 10_002
        } else {
            FEES[(rng.next() as usize) % FEES.len()]
        };
        let fa = rng.val();
        let amt = rng.val();
        check_all((sh, as_, fb, fa), amt);
    }
}

/// The rewritten helper itself: explicit range check vs `try_into().map_err(..)`, including
/// quotients exactly at / just above u64::MAX and the zero divisor.
#[test]
fn mul_div_floor_agrees() {
    let ext = |a, b, c| crate::utils::math::mul_div_floor(a, b, c).map_err(ext_err);
    let orig = |a, b, c| original::utils::math::mul_div_floor(a, b, c).map_err(orig_err);
    for &a in EDGES.iter() {
        for &b in EDGES.iter() {
            for &c in EDGES.iter() {
                assert_eq!(ext(a, b, c), orig(a, b, c), "mul_div_floor({a},{b},{c})");
            }
        }
    }
    // q == u64::MAX exactly, and q == u64::MAX + 1
    assert_eq!(ext(u64::MAX, 1, 1), Ok(u64::MAX));
    assert_eq!(ext(u64::MAX, 2, 1), Err(1));
    assert_eq!(ext(u64::MAX, 2, 2), Ok(u64::MAX));
    assert_eq!(ext(1 << 63, 2, 1), Err(1));
    assert_eq!(ext(5, 7, 0), Err(1));
    let mut rng = Rng(42);
    for _ in 0..300_000 {
        let (a, b, c) = (rng.val(), rng.val(), rng.val());
        assert_eq!(ext(a, b, c), orig(a, b, c), "mul_div_floor({a},{b},{c})");
    }
}

/// FeeBps: TryFrom guard and `apply` (floor fee, net = amt - fee).
#[test]
fn fee_bps_agrees() {
    use original::utils::math::FeeBps as OFee;
    use crate::utils::math::FeeBps as EFee;
    for &fb in FEES.iter().chain(EDGES.iter()) {
        let e: Result<EFee, ExtErr> = fb.try_into();
        let o: Result<OFee, OrigErr> = fb.try_into();
        match (e, o) {
            (Ok(e), Ok(o)) => {
                for &amt in EDGES.iter() {
                    let ea = e.apply(amt).map(|g| (g.net_amount, g.fee)).map_err(ext_err);
                    let oa = o.apply(amt).map(|g| (g.net_amount, g.fee)).map_err(orig_err);
                    assert_eq!(ea, oa, "apply fee_bps={fb} amt={amt}");
                }
            }
            (Err(e), Err(o)) => assert_eq!(ext_err(e), orig_err(o)),
            _ => panic!("FeeBps::try_from({fb}) diverges"),
        }
    }
}

/// `then_some(()).ok_or(GuardFail)` rewrite + the PodU64 -> u64 representation: the default
/// vault and the ordering boundary.
#[test]
fn solvency_boundary() {
    assert!(mk_ext((0, 0, 0, 0)).is_solvent());
    assert!(ExtVault::default().is_solvent());
    for &(s, a) in [(5, 5), (5, 4), (4, 5), (u64::MAX, u64::MAX - 1), (0, u64::MAX)].iter() {
        check_all((s, a, 0, 0), 0);
    }
}
