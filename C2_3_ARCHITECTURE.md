# C2.3 architecture amendment

Revision 14 is `Rules::PlayerAgency`. It includes every C2.2 information rule plus three player intents.

| Intent tag | Payload | Meaning |
| --- | --- | --- |
| 26 | actor id | Ask that NPC |
| 27 | 0 | Examine the current tile |
| 28 | actor in the high byte, event id in the low 24 bits | Testify |

Event ids at or above 2^24 cannot be packed into testimony and are refused. Outcome tags are 32 Learned, 33 Briefed, and 34 Testified.

`World::player_agency` is the default ASCII campaign and writes `checkpoint.c23`. `run_c22` keeps `World::information_disputes` and `checkpoint.c22`. `Session::new` stays on revision 13 so existing terminal tests keep their campaign. `new_seeded_world` checks player agency before the C2.2 branch, so `seed` does not drop back to revision 13.

Player delivery does not look the player up in the NPC map. `participant_alive` and `participant_position` cover actor 21. Automatic firsthand learning runs only for `PlayerAgency`, and only when the player is in reach and can see the site, so revision 13 digests stay the same.

Testimony weight:

- Firsthand becomes a one-hop report whose origin is the observer.
- A one-hop receipt whose sender is the player stays one hop. Examined evidence can still open a dispute.
- Any other receipt gains one hop. Heard gossip becomes hop 2 and does not open a dispute.

Pending reports still use `(recipient, event)` and `tick + REPORT_DELAY`. Equal or weaker repeats do not queue a second report. Hot disputes and the monotonic `next_event` counter are unchanged from C2.2.1.
