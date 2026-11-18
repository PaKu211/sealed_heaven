# C0.7 acceptance tests

| Gate | Proof |
|---|---|
| Conservation | `five_deliveries_conserve_twelve_herbs` |
| Five deliveries | same, shrine 5 and 5 observed tasks |
| Duplicate demand | `a_second_demand_does_not_change_the_first` |
| Duplicate task | `the_same_open_task_is_not_a_second_job` |
| Insufficient stock | `two_herbs_cannot_satisfy_a_demand_of_five` |
| Unknown worker | `an_unknown_worker_rolls_back` |
| Cancellation | `cancelling_the_open_haul_keeps_the_herb_total` |
| Save, load, replay | `save_load_and_replay_match` |
| No private balance | `the_panel_reads_the_checkpoint_not_a_private_balance` |
| Frozen C0.1–C0.6 | `tools/c05_verify.ps1` |
| Visible state | Panel text is `Supply::status`, which reads `View` and `resource_stock_quantity` |
| Multi-hauler | Not claimed. One actor only. |
| Native loop | `docs/evidence/c07-native` |
