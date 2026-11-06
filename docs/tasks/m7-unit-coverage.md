# M7 unit coverage

Observed following and a simulated base. This note does not change a row in [3.1 Carry My Luggage](3.1-carry-my-luggage.md). Every acceptance `result` there stays `not-run`. A unit or harness result is not a full task pass and not a hardware result. Scoring, the optional return, and the queue are not this milestone.

`navigation` reads a local sight and returns one proposal. The sight has people and disks. It has no route and no car. `sim_runtime` owns the base, the disks, and custody. `harness::step_drive` is the only path that applies a proposal to the base. `task_3_1` is unchanged. `Ok` from `Trip::apply` is not success. A deadline is `FeedError::TimeUp`, the phase is `TimeUp`, and the base does not move.

The professor PDF states the four obstacle classes, a crowd of three or four people, an arbitrary order, an unknown car, and a five-minute maximum. It states no radius, clearance, speed, or sensor range. The numbers below are assumptions. `REACH_MM` stays 2500.

| Assumption | Value |
| --- | --- |
| Robot disk | `ROBOT_RADIUS_MM` 250 |
| Standoff | `FOLLOW_MM` 800, on the side opposite the operator's last step |
| Arrival | Component cue only. `Cue::TakeBag`, person inside `ARRIVE_MM` 1200, both away from the first-sight poses, and the observed segment clear. This is not rulebook arrival at the unknown car. A short move such as operator `(0, 1001)` can still set the trip flag. M8 must supply a destination observation from the operator script without giving the car coordinate to the navigator, and must not invent a distance quota |
| Step cap | `STEP_MM` 200, strict. Heading arithmetic uses wide integers so every `i32` coordinate stays inside that cap in debug and release. The harness chooses the millisecond stamp. This is not a printed speed |
| Sensor | `SENSOR_MM` 2500. Hard-to-see uses `HARD_SENSOR_MM` 800 |
| Disks | small 80, hard-to-see 60, barrier area 350, person 200, crowd pitch 600 |
| Crowd count | 3 + `seed % 2`, so 3 or 4. One disk per person |
| Stuck | 8 blocked proposals. A blocked step consumes the stamp and does not move the base |
| Approach margin | 100 mm inside `REACH_MM` |

A straight step that only checked its ends would cross a disk. The segment test and the slide both reject that. The follower takes a tangent and keeps that side until the straight step is clear. It does not search a map.

On `step_drive`, the world clock and the trip stamp are one count for follow, approach, and cancel. `Begin` is the budget origin. A split, a backward stamp, a same stamp, and a terminal trip do not move the base and do not set cancel. Stamp 300000 can move. Stamp 300001 stores `TimeUp` and does not move, including when the command is cancel. An in-budget cancel consumes that stamp, marks the follower cancelled, and leaves the pose. The pick path can still hold a world time of 400000 ms beside a trip stamp of 10. That path is not navigation. `try_slide` itself still accepts a long clear segment; the harness rejects a proposal longer than 200 mm before it slides. The operator is not a collision disk. A hard-to-see disk whose center is outside 800 mm is absent from the sight, so the follower can stop rather than walk around it.

`main_complete` is the trip flag after a component handoff. It is not a score and not an end-to-end pass. The 56 acceptance rows stay `not-run`, so acceptance stays fail-closed. M8 is mandatory before any claim that the robot reached the car: the scenario follows the operator to the unknown place, and the destination observation is not a hidden coordinate. M9.3 was not run and remains mandatory before milestone closure.

The seven open scoring rows stay open: `CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`. No score is chosen.

The commands and the counts are in [WORKLOG](../../WORKLOG.md).

| requirement | source | test | result | gate |
| --- | --- | --- | --- | --- |
| Follow the observed operator. A nearer stranger is not the target. Lost and wrong-person are explicit. Cancel uses the same clock admission as follow | Printed p.15 procedure and p.16, three to four people. Issue 14. The standoff is an assumption | `the_followed_id_wins_over_a_nearer_stranger`, `a_missing_operator_is_lost_and_another_person_is_not_followed`, `loss_refind_and_the_cue_are_not_a_car_lookup`, `cancel_and_the_wrong_person_do_not_complete` | unit, see WORKLOG | sim |
| A component cue can hand off after the base and person leave the first-sight poses and the observed segment is clear. That is not arrival at the unknown car. `main_complete` is only the trip flag | Printed pp.15–16. The car stays unknown. `ARRIVE_MM` is an assumption. M8 owns the destination observation. Issue 14 | `the_cue_arrives_only_when_the_same_person_is_near`, `a_cue_at_the_start_or_across_a_disk_does_not_hand_off`, `a_followed_clear_journey_can_hand_off_without_reading_the_car`, `ready_does_not_arrive_and_one_step_moves_the_held_bag` | unit, see WORKLOG | sim |
| Four classes, each a disk. The crowd is 3 or 4 persons. Order does not open a tunnel | Printed p.15 and p.16. Radii are assumptions. Seeds 0 through 7 show both crowd counts | `seeded_obstacles_are_disks_and_the_crowd_is_three_or_four_people`, `each_obstacle_class_is_skirted_and_order_does_not_open_a_tunnel`, `the_sensor_keeps_a_far_hard_object_out_and_keeps_a_crowd_of_people` | unit, see WORKLOG | sim |
| Clearance is the whole swept segment | Printed p.17 names avoiding and not a distance. `CML-ASM-AVOID`. Issue 14 | `a_segment_cannot_tunnel_through_a_disk_between_clear_ends`, `a_straight_step_cannot_tunnel_between_clear_ends`, `a_jump_through_a_disk_is_rejected_and_a_clear_step_moves_the_held_bag`, `the_harness_does_not_slide_through_a_disk` | unit, see WORKLOG | sim |
| A blocked step consumes time, then the follower is stuck. The clock does not freeze | Issue 14. `STUCK_LIMIT` is an assumption | `blocked_headings_count_to_stuck_and_do_not_loop_inside_one_plan`, `blocked_steps_consume_time_until_stuck` | unit, see WORKLOG | sim |
| The navigation path uses one stamp. 300000 can move. 300001 does not. Split, regression, same stamp, overflow, and a finished trip do not move the base | Printed p.16, five minutes. Milliseconds are `CML-ASM-CLOCK`. Issue 14 | `split_clocks_and_a_backward_stamp_do_not_move`, `the_budget_allows_300000_and_stops_at_300001_without_moving`, `cumulative_steps_and_a_start_near_the_top_share_one_clock` | unit, see WORKLOG | sim |
| A bag at `(0, 3000)` with the operator at `(0, 1000)` is legal and outside 2500 mm. The base approaches. The reach stays 2500. A held bag moves with the base. A free bag does not. A far bag still steps at most 200 mm along the line | Printed p.15, within 2 m. Issue 14. `REACH_MM` is unchanged. Coordinates through `i32` are in range | `approach_stops_inside_the_given_reach_and_does_not_enlarge_it`, `approach_toward_a_far_bag_stays_on_axis_and_inside_the_step`, `a_direct_follow_step_at_extreme_coordinates_stays_capped`, `random_clear_steps_match_an_independent_short_step`, `a_far_bag_approach_does_not_panic_or_overshoot`, `a_legal_bag_behind_the_operator_is_reached_by_moving_the_base` | unit, see WORKLOG | sim |
| A handoff across a world disk is refused from the full segment, including a hard-to-see disk the sight omitted. Endpoints may be clear. Custody, the trace, the fault flag, the clock, and the trip do not change. A deadline still wins. Direct `hand_to` uses the same guard | Printed p.15, the operator takes the bag. The grown radius is the same assumption as a slide. Issue 14 | `a_handoff_rejects_a_disk_between_clear_ends_and_leaves_the_world`, `an_unseen_disk_blocks_the_handoff_and_a_visible_disk_blocks_arrival` | unit, see WORKLOG | sim |
