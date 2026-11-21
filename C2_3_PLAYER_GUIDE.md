# C2.3 player guide

Default `sealed_agency` is the C2.3 campaign. `--c22` is the previous information campaign and does not accept `examine`, `ask`, or `tell`. `lang en` and `lang id` switch the terminal. `help` prints the short rule. `know` lists findings you hold and does not spend a turn.

Commands that spend a turn:

- `examine` — check the tile you are standing on. A real audit site you can reach can add that finding. The receipt says you examined it.
- `ask 2` — ask that person, if they are beside you, to pass on something they know. The answer arrives a couple of turns later.
- `tell 33 0` — offer finding 0 to actor 33, who must be beside you. You can only offer a finding `know` already lists.
- `n` — wait. The settlement keeps moving if you never speak.

Actors you will meet in this ridge: watchman 2, mentor 33, Reed Hall 36. The shrine is the medicine site. Standing on it during an inspection is different from seeing the roof from the road.

Weight of what you say:

- Something you witnessed, or examined on the tile, can make the listener open or update a dispute.
- Something you only heard is gossip. They may remember it, and it does not by itself suspend anyone.
- Saying the same thing again does not stack the penalty. A later true delivery can restore cooperation. The missed delivery stays in the record.

You do not have to intervene. Leaving the mentor uninformed leaves their later bids unchanged by your silence.

## Human checklist (not yet played)

About five to ten minutes at the keyboard. This list was not executed in this verification.

1. Start the default client. Confirm the help text says C2.3. Switch to Indonesian and back.
2. `know` at the entrance. The turn counter must stay 0.
3. Walk to the shrine with `d` and `s`. `examine`. Read whether the tile has a finding.
4. Walk to the watchman and `ask 2`. Wait with `n` until `know` changes or the ask is refused.
5. Walk to the mentor. `tell 33` plus the event number from `know`. Step away and repeat the tell. The repeat should be refused.
6. `n` several times. Open `infolog`. Note whether a dispute appeared only after you spoke.
7. Start again, walk the same route, and do not tell anyone. Compare `infolog`.
8. Quit, start with `--c22`, and confirm `examine` is refused and the banner stays C2.2.

## Known limits and the next milestone

Examining an empty tile uses the same sentence as an empty supply. A save is a journal replay: only actions that went through turns are reconstructed, so a hand-edited audit is rejected on load. Contracts still open on the world's own schedule; a short walk may not contain a failure to investigate. The next milestone can give the empty-tile sentence its own wording and a naturally timed shortage the player can reach without a test fixture.
