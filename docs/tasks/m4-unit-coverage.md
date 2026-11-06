# M4 unit coverage

Unit tests of the `task_3_1` trip. This note does not change a row in [3.1 Carry My Luggage](3.1-carry-my-luggage.md). Every acceptance `result` there stays `not-run`. A unit result is not a scenario-simulation acceptance pass and not a hardware result.

The machine does not score. It has no points type. These open rows stay open, with no choice made here: `CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`.

`CLOCK_START` is `begin`. That label is assumption `CML-ASM-CLOCK`. It is not a printed start signal. `ALT_START` is `unresolved`. `LIMIT_MS` is 300000. Elapsed time is the simulation stamp minus the `Begin` stamp. A stamp at that exact elapsed time is still inside the budget.

`Reenter` and `JoinQueue` return `Reject::OutOfScope`. They do not gate the main trip. After a confirmed handoff the carried bag is empty, and a later grasp is rejected.

The command and the counts are in [WORKLOG](../../WORKLOG.md).

| requirement | source | test | result |
| --- | --- | --- | --- |
| Identify, pick, ready, follow, then handoff, and do not treat a command as success | Printed p.15 procedure. Issue 8. | `main_trip_confirms_each_step_before_completion`, crate doctest | unit, see WORKLOG |
| Reject events before `Begin`, and a second `Begin` | Issue 8. `Begin` is the clock assumption above. | `nothing_before_begin_moves_the_trip`, `a_second_begin_is_a_duplicate` | unit, see WORKLOG |
| Wrong bag, and a person id is not a bag id | Printed pp.15 and 17. Issue 8. | `the_wrong_bag_is_not_identified_or_picked`, `a_wrong_grasp_is_not_ready`, `a_person_id_is_not_used_as_a_bag`, `a_bag_id_is_not_used_as_a_person` | unit, see WORKLOG |
| Grasp failure, drop, operator loss, handoff failure | Printed pp.16 and 17 name the facts. No score is chosen. | `a_failed_grasp_does_not_hold_the_bag`, `a_drop_is_not_a_handoff`, `a_drop_with_nothing_carried_is_rejected`, `loss_and_the_wrong_person_do_not_arrive`, `finding_the_same_operator_resumes_following`, `a_failed_handoff_still_holds_the_bag`, `receipt_after_a_failed_handoff_completes_without_carrying` | unit, see WORKLOG |
| Out-of-order, duplicate, and stale events leave the trip unchanged | Issue 8. | `out_of_order_arrival_does_not_finish`, `stale_and_duplicate_confirmations_do_not_advance`, `every_rejection_leaves_the_previous_trip` | unit, see WORKLOG |
| Five-minute budget, regression, exact limit, overflow, time after completion | Printed p.16 maximum. Milliseconds and `Begin` are `CML-ASM-CLOCK`. | `clock_regression_is_rejected`, `the_budget_includes_the_limit_millisecond_and_stops_after_it`, `a_stamp_at_u64_max_does_not_wrap`, `a_start_near_u64_max_stays_in_budget`, `stepped_timestamps_cross_the_limit_once`, `time_after_complete_does_not_undo_the_handoff`, `the_same_timestamp_is_not_a_regression`, `a_large_stamp_does_not_sleep` | unit, see WORKLOG |
| Optional return and queue are outside this machine | Printed p.15 optional goals. | `optional_events_are_out_of_scope` | unit, see WORKLOG |
| Same inputs replay the same trip | Issue 8. | `the_same_script_replays` | unit, see WORKLOG |
