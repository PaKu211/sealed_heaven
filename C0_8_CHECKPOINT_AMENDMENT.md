# C0.8 checkpoint amendment

**Status:** Approved for this milestone by the C0.8 brief's stop rule. Revision 1 stays loadable and byte-identical.

## Incompatibility

`SH:C05:STATE` revision 1 is a closed payload. Its content hash binds the herb-delivery rules, and the decoder rejects any other revision, trailing bytes, or unknown intent tags. Health, a hostile actor, and combat events do not fit in that payload. Putting them only in the ASCII renderer would not survive save/load.

The C0.3 kernel checkpoint inside the C0.5 blob is not a combat store. The frozen `src` tree outside `player5` is unchanged.

## Amendment

`World::settlement()` still encodes revision 1. Existing C0.5 and C0.7 herb checkpoints restore through that path.

`World::conflict()` encodes revision 2, `SH:C05:STATE` with content hash `SH:C08:CONTENT:v1`. The tail after the embedded C0.3 bytes is: foe id `22`, foe x/y, player hp `u16`, player alive `u8`, foe hp `u16`, foe alive `u8`, then the SHA-256 of the preceding bytes. Guard is tick-local and is false at every checkpoint boundary.

Revision 2 intents, only valid in that payload: attack tag 5, guard tag 6, wait tag 7. Outcomes: waited 6, struck 7, guarded 8. Rejection `Incapacitated` is code 10. Damage is the published integers: strike 2, guard subtracts 1, player hp 6, foe hp 4, aggro Manhattan 4. Resolution order on a tick is NPC kernel, then the one player intent, then the foe. Tie-break for the foe's step is North, East, South, West.

A revision 1 file with no `shrine-supply.c03` sibling still fails the session load. This amendment does not invent that file.

## Fixture

Foe 22 starts at `(8,4)` on the herb route. The player still starts at `(0,4)`.
