# C1.3 Reed Hall

This Stage E slice connects local service, cultivation, an authored school, and succession to the existing Riverlands simulation. The source and regression scenarios are ready for native verification.

## Player loop

| Action | Requirement | Effect |
| --- | --- | --- |
| `join` | The teacher is on duty. The player has practiced breath and brings one herb. | The player becomes an apprentice and the herb enters Hall stock. |
| `duty` at the Hall | Apprentice with an unassigned duty and one herb in Hall stock. | The school entrusts the player with one herb as separate cargo. |
| `duty` at the Qinghe shrine | The player carries the Hall cargo. | One herb enters the shrine's stock. |
| `duty` back at the Hall | The shrine received the cargo and the teacher is on duty. | The apprentice becomes a disciple and earns the teacher's regard. |
| `study` at the Hall | Disciple rank, mastered breath, and two personal herbs. | Learn Reed Strike. |
| `center` | Knows Reed Strike. | Gain one Qi at the cost of a turn, up to three. |
| `form <id>` | Knows Reed Strike, has two Qi, and targets a living adjacent actor. | A three-point strike enters the normal combat and witness rules. |

The pixel window uses **Y** for duty, **Z** for study, **X** to center and **P** to strike. The normal Bevy and terminal commands remain in [C1.2's play guide](C1_2_RIVERLANDS.md). Hall cargo stays separate from the player's herbs, which can still be spent or traded. The apprentice can deliver even if a vacancy occurs while on the road. The school needs a teacher on duty to recognize the delivery.

## Succession and saves

Reed Hall initially belongs to Ren Zhi. If he dies, the living deputy, Luo Xiu, takes office and must travel to the teaching position under the existing movement and scheduling rules. If there is no living deputy, the office becomes vacant. Existing actors, cargo, and social memory remain in the world. A deputy who witnessed an attack can inherit the office and still refuse the attacker.

New campaigns write checkpoint revision 4. Revisions 1–3 reopen their original campaigns, with the original action set and roster. The revision 4 digest covers authority, the latest succession, duty, form and Qi. The journal reconstructs both the order of actions and their effects. Reset enters a fresh Reed Hall campaign.

## Verification and remaining work

Regression scenarios cover conservation across an outward and return trip, duplicate delivery, a save with cargo in transit, rank and Qi requirements, and succession from real player commands. Compilation, Rust tests and native playtesting remain pending. Stage E still has open work in competing schools, independent companion goals, and wider sect politics. Stage F remains open.
