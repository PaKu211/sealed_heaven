# C2.2 Windows verification commands

Run from the unpacked C2.2 source root on rustc 1.100.0 (or the project toolchain file).

```powershell
cargo fmt --all -- --check
cargo test --lib living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --lib -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features -- -D warnings
```

Optional scripted ASCII smoke (seed 0, English then Indonesian empty infolog):

```powershell
$env:SEALED_ASCII_COMMANDS = "help;infolog;lang id;infolog;q"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features
```

Legacy isolation smoke:

```powershell
$env:SEALED_ASCII_COMMANDS = "accountlog;q"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features -- --c21
```

Expected living count: **126** tests.  
Expected playable `--lib` count: **48** tests.  
Expected full playable `--no-default-features` (lib+integration crates listed in the audit report): **79** tests.

Patch check (from a clean C2.1 `7694ce2` tree):

```powershell
git checkout 7694ce2
patch -p1 < C2_2_FROM_C2_1.patch
```
