# M8 full-run coverage

Seeded end-to-end Carry My Luggage on one simulation clock. This note is the map for the `pass` rows in [3.1 Carry My Luggage](3.1-carry-my-luggage.md). A `pass` is that named test, not a hardware result and not a settled score. Open rows stay `not-run`. Hardware rows stay `not-run`.

The professor PDF is the rule. Section 3.1, printed pages 15–17, physical pages 20–22. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2024.2 and public 2025.1 are not the baseline. The raw PDF is not in git.

Prior art for a local follower, not a dependency and not a substitute rulebook: local sensing without a map (doi:10.1109/tac.1986.1104175), bug algorithms that keep the map out of the robot (https://www.cs.columbia.edu/~allen/F19/NOTES/Chap2-Bug.pdf), and a standoff behind a moving person as a pattern only (https://docs.nav2.org/jazzy/tutorials/general_tutorials/navigation2_dynamic_point_following/navigation2_dynamic_point_following/).

## Obstacles

The seeded shuffle is the referee order. The episode spaces those four classes along the outside path. The pitch is an assumption so a local follower can pass each disk; it is not a printed distance. Avoidance credit for a class requires three facts from that run: the robot was abreast of one of its disks (`ENCOUNTER_WINDOW_MM`, 340), that disk was inside the class sensor (800 mm for hard-to-see, 2500 mm otherwise), and the robot stayed in a 280 mm band (`CLOSE_BAND_MM`) outside the grown disk without a motion segment hitting it. The route passes 40 mm (`ROUTE_PASS_MM`) outside the grown disk. Those ranges are assumptions. Credit is not "the disk exists and x is outside the arena." The catalog value 50 is a printed line. It is not this distance proof.

A hit is a motion segment that meets a disk still in the world at that step. Removing every obstacle leaves every hit flag clear. Removing one class clears its credit and is not recorded as contact. A collision fault defaults to none and stays until another call changes it. `obstacles_for_planner` and `try_slide` omit that one class. `observe` still contains it. `note_step` and `note_encounter` read that live observation. Moving, reclassing, or injecting a disk changes that same live set. A car at `(2501, 1000)` has all four avoidance credits false. `CML-BAG-DEF` stays `not-run`: the fixture stores the bag kind and does not measure stiffness or weight.

## Two authorities

The scenario reads `truth().car()` and moves the operator along a lane that clears world disks. That lane is operator behavior. `robot_follow` takes a local sight and a cue. It has no car argument. `revealed_car` stays empty. From R1, the cue bit is a scripted gesture. The sensor reports it only when that person is inside the existing range. The scenario may choose to offer the gesture at the car. The controller function does not compare poses to the car.

`navigation::Cue::TakeBag` plus leaving the first-sight poses can still set the trip flag. That is the M7 component fixture. `EpisodeReport::main_passed` does not read the flag. It requires the operator to be on the designated car point, the pointed bag in operator custody, the robot within the labeled `ARRIVE_MM` of that operator, a locally seen cue, steps no longer than 200 mm, one shared clock at or under 300000 ms, and no jump. An early cue, a stop short of the point, an unseen operator, a jump, and the homeward-only direction fail that check even when the flag is set.

## Clock and return

Report, confirm, approach, pick, ready, follow, handoff, and the optional return share one millisecond stamp. The origin is `Begin`. Stamp 300000 can move. Stamp 300001 stores `TimeUp` on an unfinished trip and does not move. After `Complete`, a later stamp does not rewrite the phase and does not move. A blocked step consumes the stamp. Overflow does not wrap.

The optional return follows a crowd person through the labeled entrance and stays with that queue. The main trip can finish without it. Help on that return pays 0 on the bonus lines. The sheet catalog records printed lines. It does not award a run total and it returns `Undecided` for the seven open conflicts.

## Assumptions

Clearance, radii, sensor ranges, the 200 mm step, `ARRIVE_MM`, the lane offset, the entrance point, the abreast window, the close band, the route margin, and the three-step stay are assumptions. The hard-to-see disk uses the shorter local range. That is not a contrast measurement. Direct collision checks in the simulator use world disks. The planner receives only the local sight. With no collision fault, that sight is not pre-filtered by class. Direct `try_slide` can still take a long clear segment. The episode marks that as a jump and does not pass. `execute_pick` can still pair a world time of 400000 ms with trip stamp 10. That helper is outside the episode. The episode uses one clock.

`CML-SET-PLACE` fails if the armed pose is outside or moves after `Begin`. `CML-SET-BAGS` accepts a bag at exactly 2000 mm and rejects 2001 mm. `CML-SCORE-SEE` is one mock confirmation, standing for either a screen or speech, not both. `CML-GEN-LIST` is a person taking the free bag, with the gripper left empty. `CML-GEN-BONUS` is a non-navigator slide during the return; the bonus lines stay 0. A stamp at 300001 on an unfinished follow stores `TimeUp`. The same stamp on `Accompany` after completion returns `TimeUp`, leaves `Complete`, and does not move. The runner stopping at 300000 while still following is `budget`, not `TimeUp`.

The commands and the counts are in [WORKLOG](../../WORKLOG.md).
