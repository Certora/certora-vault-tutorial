import Lake
open Lake DSL

require aeneas from "/opt/aeneas/backends/lean"

package «CertoraVault» where

@[default_target]
lean_lib «CertoraVault» where
  globs := #[.andSubmodules `CertoraVault]
