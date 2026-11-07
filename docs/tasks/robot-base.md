# Robot base

This slice builds the three base plates from the private robot model, checks a kinematic step against those plates, and mounts the simulated lidar on the same base. It is the base part of stage 3. It is not a finished robot, not a dynamics model, not a camera, and not a mission.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin robot-base --locked --offline -- --out OUT
```

`OUT` is a directory. The program creates it. Exit 0 writes `model.csv`, `trace.csv`, `top.svg`, `side.svg`, `oblique.svg`, `pose.svg`, and `sweep.svg`. Exit 2 is a usage error. Exit 1 is a cast or write error. Nothing is read from a capture and nothing is sent.

## What was taken from the robot files

The private files are not in git. The plan in [WORKLOG.md](../../WORKLOG.md) names each one.

`base_link` is the model's name for the wheel midpoint on the floor: x forward, y left, z up, metres. The wheels themselves are not modelled. Layers 1 and 2 use the configured octagon from `pi robot.json`: front and rear 0.20 m, half width 0.20 m, corner cut 0.04 m. Layer 1's underside is the taped 0.030 m. The 0.002 m aluminium thickness and the 0.003 m acrylic thickness are assumed. Layer 3's underside is that layer-1 top plus the taped 0.21 m gap, so z 0.242–0.245 m. Layer 2's top is that underside minus the taped 0.12 m gap, so z 0.120–0.122 m. Layer 3's 0.05 m corner cut is from the photos. Those three prisms are the only collision solids.

The lidar xyz `(0.175, 0, 0.22)` m is the configured value in `pi robot.json`. `sensors.json` repeats it. It is not a second measurement and it is not written down as an installed lever arm. The measured front is 178.7 deg. The configured front is 183.5 deg. Bearing 0 for the measured profile is `base_link` +x. Bearing 0 for the configured profile is 4.8 deg off that axis. The URDF link yaw of 178.7 deg is the raw-angle zero already used by `frame_sample`. It is not applied a second time. The lidar head, a cylinder of radius 0.028 m and height 0.04 m, is an assumed picture. It is not a collision solid and it is not a ray target.

The half track of 0.155 m is a driver constant and is not used. Wheel radius, mass, friction, the caster, the standoffs, and the arm mount are unknown. No number was invented for them. The camera pose is recorded in the earlier profile and is not cast here.

## Frames

A base pose is the `(x, y, yaw)` of `base_link` on the floor. Yaw is counterclockwise from world +x. The sensor point is the configured xyz in `base_link`, then the base pose. `frame_sample` still returns scan-frame points. The planner's 250 mm disk stays on that scan origin. It is not moved to the base centre and it is not chassis clearance. A `Move` is a translation in the scan frame. The rigid base takes the same translation. The adapter does not add or subtract 0.175 m.

## Collision

A step may translate at most 1.0 m and yaw at most 90 deg. A longer step, or a non-finite step, is rejected and the pose stays. Those caps are not the configured 0.15 m/s and 15 deg/s, and they are not a motor limit. The origin moves in a straight line and the yaw changes linearly with the same parameter. Every supported step, including a tiny yaw, uses one test. The convex hull of a plate at the two ends contains the straight chords of its vertices. A vertex of radius `r` and yaw change `δ` stays within `r (1 - cos(|δ| / 2))` of its chord, because the arc is at most 90 deg. The whole plate stays inside that neighbourhood of the hull. The check expands each obstacle by that distance in x and in y. That square contains the disk, so a hit inside the disk is not missed. A diagonal can also block a point as far as `√2` times the sagitta. That extra band is a false block, not a clearance. It is not mass, torque, or slip. A fixed 0.5 mm pad does not replace the sagitta: at 90 deg the footprint sagitta is about 75 mm.

Interior overlap at the start is `overlap`. A shared boundary with no interior is `contact`. Either blocks the step. A later meeting is `sweep`, including a thin wall between two clear poses and a yaw whose end poses are clear while a middle angle is not. The pose does not change. Removing the solid lets the same step through. A solid in the vertical gap between plates does not block. A solid behind the base can block the plates while lying outside the 250 mm disk around the sensor. `sent` stays false. No command text is produced.

## What the controller sees

The cycle casts rays, encodes them as lidar packets, parses those packets, and posts them with `post_samples`. The policy sees the posted points and no person, so the intent is `Lost`. It does not receive the plates or the box. A low box under the lidar plane is absent from that view and still blocks a later `Move`. The trace keeps the sensor count, the policy name, the attempted pose, and the accepted pose on separate columns.

## Pictures

`top.svg` and `side.svg` use one equal metre scale and the same tick strokes as the static scene. `oblique.svg` puts metre ticks on each world axis and says the cabinet projection is not one screen scale. The dashed head is the assumed placeholder. The blue outline on the top view is the attempted step that was not accepted. The red rays are misses: they cross the low box in the top view and pass above it in the side view. `pose.svg` plots attempted and accepted x as marks on the step index. The index is not a measured clock, and nothing is drawn between the marks. `sweep.svg` shows the 0.80 m, 5 deg ends with the thin wall between them, and points at the 3 mm corner chip. The chip is smaller than the line width at that scale.

## What this does not claim

No wheel, no mass, no friction, no slip, and no physical clearance. The sagitta is a geometric bound on this parameterisation, not a measurement and not a dynamics model. The arm is not modelled. Grasp stays not validated. Camera depth is not cast in this file. The ideal pinhole is in [camera-depth.md](camera-depth.md). Section 3.1 stays 45 pass and 11 not-run. This file does not close stages 4 through 7.

## Decisions

The plates use the README's split of taped gaps and assumed thickness, not the coarser `source: measured` tag on every part in `robot.json`. The ray does not add the URDF yaw on top of `frame_sample`. A `Move` does not change the lever arm. Yaw above 90 deg is rejected rather than clamped. The policy column stays `Lost` when no person was posted, even if a later kinematic step is blocked.
