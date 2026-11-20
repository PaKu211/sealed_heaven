# C0.7 implementation spec

Branch `feat/c07-shrine-supply` from C0.6 `e4598385291abd2e87133c97aa03ca49e91e1a1d`.

## What this is

One hauler (actor 1) starts with 12 spirit herbs, definition 7, in personal stock. The shrine site stock of the same definition starts at 0. Empty kernel drives move one herb per haul onto that site stock. A desk records demand 5 and the kernel task id it has already seen. It stores no quantity.

This is the C0.3 actor-owned ledger. It is not a room storehouse and it is not the C0.5 player herb stack of 3.

The village settlement and this hauler are different checkpoints. Both advance when the player steps. The panel names the hauler so he is not mistaken for the village healer.

## Lifecycle labels

These names are read from the checkpoint. They are not a second task record.

| Label | Kernel fact |
|---|---|
| Requested | Demand is set and the hauler's task id is 0 |
| Assigned | A new non-zero task id was just observed |
| In progress | The same task id is still open |
| Completed | Shrine quantity is at least the demand and the task id is 0 |
| Rejected | `drive` returned an error and the checkpoint bytes are unchanged |
| Cancelled | Actor 1's `CancelReservation(0)` left the task id at 0 |

After completion the session stops driving this scenario. The kernel has no demand quota; further empty drives would haul the remaining 7 herbs.

## Not in this contract

Multi-hauler contention. The checkpoint has one actor. A second hauler is not claimed.

Daylight at tick 720 was not recaptured. Reaching it at the 2 Hz presentation rate takes six minutes, and the C0.6 clock tests already cover the boundary.
