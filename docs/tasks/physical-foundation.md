# Physical foundation

This crate is the offline foundation for a later section 3.1 simulator. It reads posted captures, keeps disagreeing profiles side by side, and writes a report. It does not award a score, send a command, or claim a hardware result. The seven approved stages are listed at the end. This document covers the foundation only.

Committed text is English. The 2026-11-06 repository rule still applies. A parent may explain the same material privately in another language. That explanation is not a file in this repository.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin replay --locked -- --session SESSION --out OUT
cargo run -p physical --bin replay --locked -- --session SESSION --out OUT --arm-urdf FILE
```

`SESSION` is a directory outside git. `OUT` is the report directory. `--arm-urdf` is optional and is also outside git. Exit 0 means the report was written. Exit 2 means the arguments were wrong. Exit 1 means the session directory is missing or the report could not be written. A malformed file inside the session is recorded in `report.md` and does not invent a replacement.

The private course session used for the numbers below is not in git. The command and the artifact directory are in [WORKLOG.md](../../WORKLOG.md).

## What the controller is allowed to see

`controller_note` takes a posted scan and an optional posted depth view. It has no scene-truth argument. A fresh scan contributes its positive ranges. A stale, missing, malformed, or reversed scan keeps its samples and contributes none. Depth class `0` is no-return. A positive value below the configured 500 mm gate is below-gate. A value at or above the gate is valid. Person state in this milestone is `not in posted view`, because there is no detector. Actuation is `not sent`.

A log line that contains the word `lost` is captured text. It is not proof that a person ceased to exist.

## Components

`kind` names the status of a number: measured, configured, derived, assumed, specified-scenario, reference-code, inferred, unknown, upstream-nominal, or blocked. A chart or table that uses a number keeps that name.

`profile` freezes two lidar fronts and two camera mounts. The measured lidar front is 178.7 deg. The configured front is 183.5 deg. The crate does not choose one. Their difference is 4.8000 deg, and the lateral separation at 1 m is 0.0837 m, both derived. Lidar xyz is (0.175, 0, 0.22) m, configured, and the same xyz appears in two files, so the second file is not a second measurement. Range in metres is the 16-bit word divided by 4000. That divisor is protocol text, not a tape calibration.

The measured camera is x 0.140 m, z 0.282 m, pitched 6.7 deg down. The configured camera is x 0.140 m, z 0.270 m, pitched 0 deg. `mount_confirmed` is a flag in the config file and does not settle the disagreement. Horizontal field of view 58.59 deg and vertical field of view 45.64 deg are cited OpenNI properties, not remeasured here. The image is mirrored, so image right is robot left. The principal point used for a pixel bearing is the image center. That is an assumption. The session's 153.6 row fit is not recomputed.

Floor range is computed. The lower ray is pitch down plus half the vertical field of view. Ground range is height divided by the tangent of that downward angle when the angle is positive. The measured mount's lower ray meets the floor at 0.4980 m, and its optical axis meets the floor at 2.4006 m. The configured mount's lower ray meets the floor at 0.6417 m. Its optical axis, at pitch 0, has no floor hit. These are derived. They are not a copy of a stored constant, and this run does not refit the plane.

The footprint width derived from the octagon is 0.400 m. The front face at x 0.20 m is 0.320 m wide. A specified-scenario opening of 0.28 m blocks both. That opening is not a width measured on 2026-11-07. A specified-scenario top at 0.10 m is below the 0.22 m lidar plane, so that plane misses it. A top equal to the plane is not missed. Wheel radius and robot mass are unknown, so there is no rolling model and no contact claim. A half-track of 0.155 m appears only as a sentence in a private README and is not used.

`lidar` parses the posted packet: header `AA 55`, count 1 through 40, first and last angle words, XOR checksum, then three bytes per sample. A bad checksum is a reject and the parser resumes one byte later. A count outside 1 through 40 is a reject. A short tail is unparsed, not a reject. Zero ranges stay. Raw angle is clockwise. Base angle is `wrap(front_raw - raw)` into (-180, 180], with 180 kept and -180 mapped to 180. Several samples in one packet interpolate from the first angle to the last with a positive span. Revolutions split when the raw angle falls more than 180 deg behind the previous sample. `encode_packet` exists so tests can build a packet whose checksum matches the parser. The control type byte is not filtered.

`chunk` reads little-endian seconds, a little-endian byte length, then that many bytes. A non-finite time or a short payload is an error after the good prefix is kept.

`command` reproduces the reference text format. Linear is clamped to 0.15 m/s and angular to 0.26 rad/s. Both absolute values under 0.001 become `S#`. Otherwise the text is `F` or `B`, three truncated digits, a sign, three truncated digits, and `#`. Truncation is not rounding: 0.149 m/s becomes `F014|+000#`. 0.199 m/s is clamped to 0.15 m/s and becomes `F015|+000#`. `wire_preview` applies the reference linear scale 1.28 and angular scale 1.50 and then formats. A request of 0.117 m/s previews as `F014|+000#`. That text is not sent. Yaw scale 1.16 and distance scale 1/0.88 are printed as reference-code knobs and are not applied to the reported odometry.

`json` is a small parser for the posted objects and arrays. It rejects trailing junk, non-finite numbers, and leading zeros. It is enough for the session files. It is not a general JSON library.

`npy` inspects version 1, 2, and 3 headers. Depth loads only little-endian `u16`, rank 2, and `fortran_order` false. Zeros stay. Fortran order can be inspected and cannot be loaded as depth.

`zipnote` reads a local zip header. Method 0 can be skipped. Method 8, a data-descriptor flag, or a compressed size of `u32::MAX` stops the listing and is labeled blocked. The size is compared as a `u32` before it is widened. The course `camera.npz` begins with method 8 on `color.npy`. No frame is invented. This milestone does not inflate.

`speed` reads runs of request, duration, two odometry triples, and two scans. The file does not name odometry units. The report gives raw dx, dy, dyaw, the chord, and a body-forward value that uses the start yaw. Body-forward is inferred. The stored scan angles are already `wrap(measured front 178.7 deg - raw clockwise)`. Adding the 4.8000 deg disagreement reconstructs configured-front angles for the same ranges. That reconstruction is unselected. The shifted column is that alternative profile. It is not an adopted speed estimate, and the stored angles and the reference scales stay as they were.

Scan correspondence uses 1 deg bins and ranges in (0.05, 20) m. Three gates are assumed, and none is a calibration: ±20 deg with range change under 0.40 m, ±30 deg under 0.80 m, and ±45 deg under 1.50 m. Fewer than 8 pairs produces no speed. Two projections are both derived. Ray projection divides the range change by cos(angle). It matches a small move when the same bearing still sees the same point. Frontal plane multiplies the range change by cos(angle). It matches a wall perpendicular to the forward axis. The median absolute deviation is inside one model. It does not include the gap between the models. The crate does not select a model.

`observe` holds freshness, depth class, the controller note, pixel bearing, and the line sanitizer. A line containing `@`, `ts.net`, `users\`, `/home/`, `ssh `, or an IPv4 address is replaced by `[omitted: address-like token]`. Reports use file basenames.

`arm` names SO-ARM100 from TheRobotStudio, commit `a758567c3978dfeefe282ede0500085a48fe8f78`. The recorded URDF SHA-256 is `f7050417ebbb79358f113ddd4bca2b457b2c6d5048193331db5a04a0066439ed`. The license at that commit is Apache-2.0. The repository root listing contained `LICENSE` and no `NOTICE` file. This commit does not add the meshes or the URDF. Upstream joint limits and the phrase `Approximate inertial values` are upstream-nominal. Link masses are not summed into a robot weight. Mount to this base, installed calibration, payload, and grasp stay unknown or not validated. `simulated_gripper_is_this_arm` is false. Grasp status is always `NotValidated`, including when a test URDF has a different hash.

`plot` writes SVG with the standard library. The scan is a plan in metres whose axis cross is the robot origin `(0 m, 0 m)`, with numeric ticks on an equal scale. It is not the center of the plotted points. Other charts use data bounds, numeric ticks, wrapped titles and captions, and a legend swatch for each series. The frame schematic is text, not a photograph and not a mesh. Every title names the dataset, the unit, and the kind. PNG inventory for the replay is `camera/*.png` only. `figures/` is not read.

`sha256` is the hash used to compare a passed URDF with the recorded digest. The empty-string and `abc` vectors are tested.

`replay` is the binary and the report writer. It compares numeric fields in `pi_config/robot.json` with the frozen profiles and does not copy string fields. The drawing scan prefers `course_lidar.bin`. Counts use every sample. The longest revolution is the drawing only. A 2.0 s early split on the static recording tests a session note. It is not a sensor constant. Revolution rate is the reciprocal of the median positive period, and only when chunk stamps are fresh.

## Private course replay

These numbers come from one replay of the 2026-11-07 course session. They are not adopted as controller constants.

| item | result | kind |
| --- | --- | --- |
| `course_lidar.bin` | 8044 samples, 146 zeros, 342 packets, 0 rejects, 21 revolutions | measured |
| static `lidar_raw.bin` | 70252 samples, 1784 zeros, 2994 packets, 0 rejects, 177 revolutions | measured |
| static chunks | 343 chunks, fresh stamps, payload 240762 bytes, equal to the raw file | measured |
| static rate | median period 0.100239 s, rate 9.9762 Hz | derived |
| static early 2.0 s | 208 zeros in 8085 samples | derived split, not a sensor constant |
| static later | 1576 zeros in 62167 samples | derived split |
| static pose | 49 rows, max absolute step 0, duration field 15.0 | recorded odometry, idle |
| depth frames | seven `320x240` `u16` frames, zeros retained | measured, gate configured |
| PNG | seven `camera/*.png` files, bytes counted, pixels not decoded. `figures/` was not read | blocked, camera inventory only |
| `camera.npz` | `color.npy` method 8 | blocked |
| URDF | 9110 bytes, hash match true, 6 revolute joints including the gripper, 7 mass tags | upstream-nominal |
| logs | four files transcribed, `sent=false` on every command row | captured, not sent |
| `loop_timing.log` | 48 positive stamp deltas, median 0.0600 s, min 0.0500 s, max 0.0700 s, one line contains the word lost | derived from that file |
| `fake_follow.log` | 197 positive stamp deltas, median 0.0700 s, min 0.0600 s, max 0.2700 s | derived from that file |

`course_depth_mm.npy` has 10448 no-return, 0 below-gate, and 66352 valid pixels. `img3_depth_mm.npy` has 24672 no-return, 1227 below-gate, and 50901 valid. The other five frames have no below-gate pixels. The gate is the configured 500 mm value.

Speed, in metres. Commanded distance is request times recorded duration. Odom chord is the raw chord. Scan medians are the two models at the three assumed gates.

| run | commanded | odom chord | ray narrow / mid / wide | plane narrow / mid / wide |
| --- | ---: | ---: | --- | --- |
| `speed_test.json` 0, 0.0600 m/s for 2.5300 s | 0.1518 | 0.1720 | 0.1639 / 0.1832 / 0.1967 | 0.1544 / 0.1541 / 0.1507 |
| `speed_test.json` 1, 0.1170 m/s for 2.0100 s | 0.2352 | 0.2247 | 0.2235 / 0.2254 / 0.2716 | 0.2064 / 0.2064 / 0.2000 |
| `speed_test2.json` 0, 0.1170 m/s for 1.5300 s | 0.1790 | 0.1749 | 0.1948 / 0.1909 / 0.1915 | 0.1811 / 0.1527 / 0.1463 |

The mid-gate yaw shift, which is a sensitivity and not a measurement, is 0.1706 m and 0.1526 m for run 0, 0.2539 m and 0.2121 m for run 1, and 0.1859 m and 0.1585 m for the third run. The first number in each pair is ray projection. The second is the frontal plane. Wire previews were `F007|+000#`, `F014|+000#`, and `F014|+000#`. None was sent.

`prims6.json` has 17 drive values and 16 turn values. The drive numbers sum to 3.1440. The file has no unit field. The reference distance scale is not applied.

Map arrays were headers only: `occ4.npy` shape `[430, 280]`, `world_cam.npy` shape `[61573, 2]`, `world_cam_h.npy` shape `[61573]`, `world_lidar.npy` shape `[1007, 2]`. They are not given to a controller.

The artifact directory contains `report.md`, `scan-longest.csv`, `scan-longest.svg`, `frame-geometry.svg`, `speed.csv`, `speed.svg`, `yaw-sensitivity.csv`, `yaw-sensitivity.svg`, `depth.csv`, `depth.svg`, `zero-fraction.svg`, `commands.csv`, `raw_lidar-counts.csv`, and `arm-joints.csv`. Those files stay out of git because the scan drawing shows the room.

One debug replay recorded 5336 elapsed milliseconds. An earlier debug replay of the same session recorded 2418 milliseconds. The program does one pass over each posted file, does not decode PNG, and does not inflate the camera archive. The two timings are not a benchmark and not an acceptance threshold.

## Tests that must fail closed

A bad checksum does not become a range. A wild count does not become a sample. A short packet tail is unparsed. Zeros stay in lidar and in depth. Fortran order is refused for a depth load. A deflate zip member stops the listing. Fewer than eight scan pairs produce no speed. A reversed chunk stream stays reversed and is not treated as a fresh rate. The two fronts stay apart. The low top is missed and the 0.28 m opening blocks the chassis. The synthetic session plants `203.0.113.50` in a config and a log, and the output tree must not contain it. A URDF that is not the recorded file still reports grasp as not validated.

## Seven stages

1. Freeze measured, configured, and unknown values, including the angle conflict. This run encodes both profiles, compares `robot.json` numbers, and tests that the fronts stay apart. It does not close the freeze: wheel radius, mass, arm mount, installed joint calibration, and the reference scales remain unknown, and the speed scans are already stored at the measured front.
2. Reuse Rust logic and port only the reference behavior that the tests need. This run adds parsers and the command text beside the existing task crates. It does not replace `task_3_1`, `navigation`, or the simulated gripper.
3. Evidence-based 3D robot, scene occlusion, and collision, with unknown wheels and mass labeled. The base plates, the kinematic step, and the mounted lidar are in [robot-base.md](robot-base.md). Wheels, mass, the caster, the arm, and camera depth are not in that slice. It is not a reviewed closure of this stage.
4. Scene-generated lidar and camera depth, with the controller limited to sensors and fault tests. The lidar cast is the static scene. The ideal pinhole is in [camera-depth.md](camera-depth.md). It is not a reviewed closure of this stage, and it does not replay a recorded frame.
5. Replay real captures through the same control logic, including units, angle, mirror, and time, and compare commanded speed with estimated speed, with no actuation. Not done. This run compares captured scans, odometry, and command text. It does not call the task controller. Every transcribed command has `sent=false`.
6. Section 3.1 mission and field-failure tests, with a simulated pickup labeled as simulated. Not done. The gap, the low top, and the depth gate are inequalities on frozen numbers. There is no mission matrix and no pickup result.
7. Independent review before an application, a replay, or a decision log is delivered, plus labels that separate simulation, real comparison, and work that needs hardware. Not this run. The report's limitations section is the current label. It is not that review.

## Review and iteration

The first scan formula divided a frontal-wall range change by cos(angle) and did not recover the translation. The wall test now requires 0.200 m from the frontal-plane model and requires the ray model to disagree. The report prints both.

`format_command(0.199, 0)` was expected to show unclamped truncation. The reference encoder clamps to 0.15 m/s first, so the text is `F015|+000#`. The test now checks that clamp and the unclamped 0.149 m/s text `F014|+000#`.

Edition 2024 format strings do not accept a path or a call inside the braces. Those strings were rewritten before the crate compiled. Clippy 1.99 required `as_chunks` and collapsed `if` forms. Both are in the committed code.

The first report printed floor ranges with debug `Some(...)` text and called a word count `lost-lines`. The report body then printed metres or `no floor hit`. The frame schematic and the static duration line still printed `Some("0.4980")`, `Some("0.6417")`, and `Some(15.0)` on `389f28e`. Those lines now use metres, `no floor hit`, or `missing`.

The scan cross on that commit was the center of the nonzero points, about 2 m forward of the robot, and the grey lines had no tick labels. Captions on the scan, speed, and depth pictures ran past the right edge. Bar legends had no colour swatches. The yaw-sensitivity axis ran into the caption. The shared plot layout now wraps text, keeps axes inside the plot box, and labels ticks in the axis unit. Raster checks use `ffmpeg` with the librsvg decoder. They are layout checks. They are not a claim that the room geometry was accepted.

Deflate stayed blocked. Adding an inflate implementation would decode private frames this milestone does not need.

## What this does not claim

No 3D body, no scene raycast, no mission score, no hardware motion, and no selected lidar front or camera mount. PNG pixels in `camera/` are not decoded, and `figures/` is not read. Deflate frames are absent. Wheel radius and mass are unknown. The arm mount and grasp are not validated. The existing section 3.1 table is unchanged: 45 `pass` and 11 `not-run`, counted from that table. The legacy Python spec checkers were not run for this fix. That exclusion means this validation is not the historical full checker set.
