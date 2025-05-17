pub mod constants;
pub mod errors;
pub mod instruction;
pub mod loaders;
pub mod operations;
pub mod processor;
pub mod state;
pub mod utils;

pub mod certora;

pub use errors::{VaultError, VaultResult};
use solana_program::declare_id;

declare_id!("CRTRcNtiG8u4EFNkVnQkKcFYRRkLa2LtFPbihbsrcbJY");
