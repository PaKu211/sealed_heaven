# C2.4 — seen crime

Revision 15, `Rules::Consequences`. `--c23` stays revision 14. Default ASCII is C2.4.

An empty `examine` on this campaign says there is no institutional finding on the tile. Revision 14 still reports that case with the old shared empty-supply sentence, so existing C2.3 saves keep their outcome bytes.

`take <id>` moves every herb on a dead body the player can reach onto the player. A living carrier refuses, and the cargo stays where the turn would have left it. A second take finds an empty body. A dead player cannot steal. People who can see both the player and the body learn who did it and lose regard. People who cannot see it learn nothing. The missing herbs are the only remote consequence.

The herb total of a theft turn matches the same turn without the theft. The fixture that places herbs on a corpse is not a public-command playthrough. The public command test checks the empty-tile sentence in English and Indonesian, the C2.4 help text, and that naming an unseen body does not spend a turn.

## Verification actually run

Linux, rustc 1.100.0, `HOME=/root`, worktree `/root/sealed_heaven_c22`, branch `feat/c24-seen-crime`.

- `cargo fmt --all`
- `cargo clippy --offline --all-targets -- -D warnings` clean
- `cargo test --offline --lib`: 272 passed, 1 ignored
- `cargo clippy --offline --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings` clean
- playable `--no-default-features`: 51 lib, 6 adapter, 9 agency, 3 clock, 9 supply, 2 conflict, 2 memory (82 passed)
- kernel paths versus `7694ce2`: no diff

Not run: Windows, human playtest. `PENDING HUMAN PLAYTEST`. `PENDING WINDOWS VERIFICATION`.

Parent C2.3 tip: `1fd97df7fa45395d3008aa6979b34aab5239a13e`.
Implementation commit: `21345966c8ea5547c87202debae225a5a60787dd`.
