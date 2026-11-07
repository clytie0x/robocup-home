# Scan to navigation adapter

This adapter posts one captured lidar revolution into the existing navigation view. It is not a 3D scene, not a mission controller, and not a grasp. The scan is a file. It is not a live lidar, and the run does not open a port.

## What it does

`post-scan` reads a chunk file. Each record is a little-endian `f64` second, a little-endian `u32` length, and that many payload bytes. Those seconds are the capture monotonic clock. They are not wall time. The payload bytes are LDS-03 packets. Metres are the 16-bit range word divided by 4000. That divisor is the protocol. It is not a tape calibration.

The caller passes `--profile measured-front` or `--profile configured-front`. There is no default. Measured front is 178.7 deg, from `lidar.py` `FRONT_LASER_DEG`. Configured front is 183.5 deg, from `robot.json`. Neither value is selected as the physical front. The view uses only the named profile. A count of samples whose framed point differs under the other profile is an audit. It is not a second input to the planner.

A sample becomes a scan-frame point with `wrap(front - raw clockwise)`, x forward and y left, then rounded to millimetres. The origin of that frame is the sensor. The configured lidar mount `(0.175, 0, 0.22)` m is recorded provenance. It is not added to the point, and the installed lever arm is not replaced with 0. No reference speed scale is applied. A zero range, a negative range, a non-finite sample, a range that does not fit in `i32` millimetres, and a positive range that rounds to the origin stay in the audit and are not obstacle disks. A posted hit is a disk of radius 0 at that millimetre point. The radius is not a measured object size and not the chassis. Points farther than the navigation sensor limit of 2500 mm stay in the audit and stay out of the view.

The view robot is the scan origin, `(0, 0)` mm. The planner treats that posted origin as the robot origin. `navigation::assume::ROBOT_RADIUS_MM` is 250 and `SENSOR_MM` is 2500. Both are planner assumptions around the posted origin. The 250 mm disk does not describe a measured chassis and does not certify physical clearance. People, bags, and gesture stay empty. The scan does not contain them. `navigation::accept` decides whether the capture time is usable. The age limit is that crate's 1000 ms assumed delay profile. The capture script's 0.5 s stream drop is not used. `plan_follow` then runs on an accepted view. With no person it returns `Lost`. `Lost` is the policy output. It is not a clear field and not a mission result. `sent` stays false. There is no command text and no port.

A missing stamp, a non-finite stamp, or a reversed chunk order does not become a fresh view. Raw samples are still counted. Absence of a scan is not sensor health.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin post-scan --locked --offline -- --chunks CHUNK.bin --profile measured-front --out OUT
```

`--profile` is required. The process writes `post.csv`, `points.csv`, and `post.svg` in `OUT`. `post.csv` names the profile, the capture-monotonic clock, `captured_ms`, `decision_ms`, `stale_decision_ms`, the file and revolution counts, and the reject counts `zero_returns`, `boundary_origin`, `beyond_sensor`, `negative_ranges`, `non_finite`, and `extreme`, then `view_points`. For one posted revolution those reject counts plus `view_points` equal `revolution_samples`. The SVG axes are metres. The cross is the posted scan origin, which this planner calls the robot origin. The caption calls the marks sensor returns. It is not a measured obstacle radius and not the chassis disk. The caption also prints the decision time and the stale decision. The picture is the posted view, not a photograph. Exit 0 writes those files. Exit 2 is a usage error. Exit 1 is a read or parse error.

`course_lidar.bin` has no chunk clock, so it is not a freshness proof. A chunk file from the static capture is the file with monotonic stamps. The decision clock for the fresh row is that stamp, measured from the first chunk. The stale row is the same stamp plus 1001 ms on a separate history. Both rows are the capture clock. They are not wall time.

Camera PNG files and method-8 `camera.npz` stay blocked. This binary does not open them and does not decode them.

## What the controller can use now

An accepted scan can supply obstacle points in the robot frame, a capture-monotonic time, and the named front as metadata. The existing follow plan can see those points. It cannot see a person, a bag, or a gesture from this file.

## Still open

The shared mission controller is not started. It still has to post a person, a bag, and a gesture from their own sensors, keep this scan on the same `accept` clock, and refuse scene truth. It does not send a command from this adapter. Wheel radius, mass, installed yaw, and the mount lever arm stay unknown. This adapter does not build a 3D body from a capture. A separate synthetic static scene can feed the same `frame_sample` and `post_samples` path; that slice is [static-scene-3d.md](static-scene-3d.md). It is not a physics engine and not a grasp. Camera PNG files stay undecoded. Section 3.1 stays 45 `pass` and 11 `not-run`.
