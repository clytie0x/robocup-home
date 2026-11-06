# M7 unit coverage

Observed following and a simulated base. This note does not change a row in [3.1 Carry My Luggage](3.1-carry-my-luggage.md). Every acceptance `result` there stays `not-run`. A unit or harness result is not a full task pass and not a hardware result. Scoring, the optional return, and the queue are not this milestone.

`navigation` reads a local sight and returns one proposal. The sight has people and disks. It has no route and no car. `sim_runtime` owns the base, the disks, and custody. `harness::step_drive` is the only path that applies a proposal to the base. `task_3_1` is unchanged. `Ok` from `Trip::apply` is not success. A deadline is `FeedError::TimeUp`, the phase is `TimeUp`, and the base does not move.

The professor PDF states the four obstacle classes, a crowd of three or four people, an arbitrary order, an unknown car, and a five-minute maximum. It states no radius, clearance, speed, or sensor range. The numbers below are assumptions. `REACH_MM` stays 2500.

| Assumption | Value |
| --- | --- |
| Robot disk | `ROBOT_RADIUS_MM` 250 |
| Standoff | `FOLLOW_MM` 800, on the side opposite the operator's last step |
| Arrival | `Cue::TakeBag` and the followed person inside `ARRIVE_MM` 1200. Not a car coordinate |
| Step cap | `STEP_MM` 200. The harness chooses the millisecond stamp. This is not a printed speed |
| Sensor | `SENSOR_MM` 2500. Hard-to-see uses `HARD_SENSOR_MM` 800 |
| Disks | small 80, hard-to-see 60, barrier area 350, person 200, crowd pitch 600 |
| Crowd count | 3 + `seed % 2`, so 3 or 4. One disk per person |
| Stuck | 8 blocked proposals. A blocked step consumes the stamp and does not move the base |
| Approach margin | 100 mm inside `REACH_MM` |

A straight step that only checked its ends would cross a disk. The segment test and the slide both reject that. The follower takes a tangent and keeps that side until the straight step is clear. It does not search a map.

On `step_drive`, the world clock and the trip stamp are one count. `Begin` is the budget origin. A split, a backward stamp, a same stamp, and a terminal trip do not move the base. Stamp 300000 can move. Stamp 300001 stores `TimeUp` and does not move. The pick path can still hold a world time of 400000 ms beside a trip stamp of 10. That path is not navigation.

The seven open scoring rows stay open: `CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`. No score is chosen. M9.3 was not run.

The commands and the counts are in [WORKLOG](../../WORKLOG.md).

| requirement | source | test | result | gate |
| --- | --- | --- | --- | --- |
| Follow the observed operator. A nearer stranger is not the target. Lost and wrong-person are explicit | Printed p.15 procedure and p.16, three to four people. Issue 14. The standoff is an assumption | `the_followed_id_wins_over_a_nearer_stranger`, `a_missing_operator_is_lost_and_another_person_is_not_followed`, `loss_refind_and_the_cue_are_not_a_car_lookup`, `cancel_and_the_wrong_person_do_not_complete` | unit, see WORKLOG | sim |
| Arrival is a cue while that person is near. Emitting follow is not arrival. The car is not an input | Printed pp.15–16, the car is unknown until the operator is followed. `ARRIVE_MM` is an assumption. Issue 14 | `a_follow_plan_does_not_arrive_without_the_cue`, `the_cue_arrives_only_when_the_same_person_is_near`, `ready_does_not_arrive_and_one_step_moves_the_held_bag`, `the_same_stamp_is_not_a_second_step` | unit, see WORKLOG | sim |
| Four classes, each a disk. The crowd is 3 or 4 persons. Order does not open a tunnel | Printed p.15 and p.16. Radii are assumptions. Seeds 0 through 7 show both crowd counts | `seeded_obstacles_are_disks_and_the_crowd_is_three_or_four_people`, `each_obstacle_class_is_skirted_and_order_does_not_open_a_tunnel`, `the_sensor_keeps_a_far_hard_object_out_and_keeps_a_crowd_of_people` | unit, see WORKLOG | sim |
| Clearance is the whole swept segment | Printed p.17 names avoiding and not a distance. `CML-ASM-AVOID`. Issue 14 | `a_segment_cannot_tunnel_through_a_disk_between_clear_ends`, `a_straight_step_cannot_tunnel_between_clear_ends`, `a_jump_through_a_disk_is_rejected_and_a_clear_step_moves_the_held_bag`, `the_harness_does_not_slide_through_a_disk` | unit, see WORKLOG | sim |
| A blocked step consumes time, then the follower is stuck. The clock does not freeze | Issue 14. `STUCK_LIMIT` is an assumption | `blocked_headings_count_to_stuck_and_do_not_loop_inside_one_plan`, `blocked_steps_consume_time_until_stuck` | unit, see WORKLOG | sim |
| The navigation path uses one stamp. 300000 can move. 300001 does not. Split, regression, same stamp, overflow, and a finished trip do not move the base | Printed p.16, five minutes. Milliseconds are `CML-ASM-CLOCK`. Issue 14 | `split_clocks_and_a_backward_stamp_do_not_move`, `the_budget_allows_300000_and_stops_at_300001_without_moving`, `cumulative_steps_and_a_start_near_the_top_share_one_clock` | unit, see WORKLOG | sim |
| A bag at `(0, 3000)` with the operator at `(0, 1000)` is legal and outside 2500 mm. The base approaches. The reach stays 2500. A held bag moves with the base. A free bag does not | Printed p.15, within 2 m. Issue 14. `REACH_MM` is unchanged | `approach_stops_inside_the_given_reach_and_does_not_enlarge_it`, `a_legal_bag_behind_the_operator_is_reached_by_moving_the_base`, `a_legal_two_meter_placement_behind_the_operator_is_outside_the_seeded_reach` | unit, see WORKLOG | sim |
