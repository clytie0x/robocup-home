# Camera depth

This slice casts an ideal pinhole into the same boxes and base plates the lidar uses. It is the camera half of stage 4. It is not a calibrated camera, not a photograph, not a detector, and not a replay of a recorded frame.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin camera-depth --locked --offline -- --out OUT
```

`OUT` is a directory. The program creates it. Exit 0 writes `model.csv`, `trace.csv`, `fault.csv`, `side.svg`, `depth-row.svg`, and `depth-grid.svg`. Exit 2 is a usage error. Exit 1 is a cast or write error. Nothing is read from a capture and nothing is sent.

## What was taken from the robot files

The private files are not in git. The plan in [WORKLOG.md](../../WORKLOG.md) names each one. The seven `camera/*.png` files and the method-8 `camera.npz` were not decoded. No pixel from them is in this model. The professor PDF is the frozen table in [3.1-carry-my-luggage.md](3.1-carry-my-luggage.md). This slice does not reopen it.

The image size is 320 by 240, from the `sensors.json` model string. The horizontal field is 58.59 deg and the vertical field is 45.64 deg. Both are cited OpenNI properties. `build.py` rounds a longer vertical constant to 45.64. This slice keeps the rounded value. The image is mirrored, so image right is robot left. A pixel index is the center of that pixel. The optical axis sits halfway between columns 159 and 160, and between rows 119 and 120. That is the same center `pixel_bearing_deg` already uses. The session's optical row of about 153.6 is recorded and is not applied. There is no stored focal length in metres. The tangents come from the field and that center. Distortion, noise, and a device maximum range are unknown, so none of them is added.

Depth is axial z in the optical frame, in millimetres. Zero is no return. The Euclidean length of the ray is longer once the pixel leaves the axis. The near gate is 0.5 m, the cited OpenNI property. A positive sample under that gate stays in the buffer and is classed `BelowGate`. It is not wiped to zero. The note that the device has no depth under 0.5 m is that class. The blocked frames were not opened, so this slice does not claim the device's zero fill. There is no far cutoff.

## Two mounts

`measured-mount` is x 0.14 m, y 0, z 0.282 m, pitched 6.7 deg down. That height and pitch are the floor-plane fit in `sensors.json`, repeated by the URDF `camera_link` origin. This run does not refit the plane. `configured-mount` is x 0.14 m, y 0, z 0.27 m, pitch 0, from `pi robot.json`. `mount_confirmed` in that file is a flag, not a measurement. The picture uses the measured mount and does not choose it as the installed camera.

The URDF `camera_optical_joint` is `rpy -π/2 0 -π/2` from `camera_link`. That is the usual optical frame: z forward, x right, y down, before the mirror. The cast uses that convention once. It does not apply the joint a second time on top of the pinhole. Positive pitch is a right-handed turn about the camera's left axis, so the forward axis gains a negative z. The URDF pitch of 0.116937 rad is the same 6.7 deg down.

The camera housing, 35 by 178 by 45 mm, is an assumed product size. It is not an occluder. The clamp stand is omitted. The lidar head stays a picture, not a ray target.

## Projection

The camera origin is the mount in `base_link`, then the base pose. Yaw of the base is counterclockwise from world +x. A pixel's image-right tangent is `(column - center) / center * tan(horizontal field / 2)`. Image-down uses the vertical field the same way. Columns 0 and 319 are the horizontal field edges. Rows 0 and 239 are the vertical field edges. A tangent past either edge is not a pixel. The mirrored ray's local components are forward 1, left equal to image right, and up equal to the negation of image down. The mount pitch is applied, then the base yaw. The ray is normalised. The nearest positive hit on a scene box or, when plates are included, on a base plate, wins. A box behind the camera is a miss.

Axial depth is the hit distance times the ray's component along the optical axis. On a wall perpendicular to that axis, every pixel that hits the wall has the same axial depth, and an edge pixel has a longer range. `floor_hit` remains the horizontal ground distance `z / tan(down)`. On the optical axis the ray length to the floor is `z / sin(pitch)`, and the axial depth equals that length because the ray is the axis. The two numbers disagree. A level configured axis does not meet the floor.

## Faults

Dropout is a list of sample indexes. Those posted millimetre values become 0. The ideal axial depth on the same sample stays, and the boxes do not move. A stale, missing, malformed, or reversed delivery uses the existing freshness helper. `controller_note` counts valid depth only when the view is fresh. The note still has no scene name, no box, and no person. `sent` stays false. The person state stays `NotInPostedView`.

## What the controller sees

`trace.csv` and `fault.csv` have `col`, `row`, `axial_mm`, `range_m`, `depth_class`, `fresh`, and `sent`. They do not name a box or a plate. The side picture does name the fixture, and it says those labels are not posted. Colour on the depth grid is an axial-metre scale. It is not a fixture identity and not a detector.

## Pictures

`side.svg` is an equal-metre side view. The three plates are drawn by their x extent, so the corner cut is not in that view. The red ray is image row 239 and meets the low box. The green ray is the level lidar at 0.22 m and does not. `depth-row.svg` plots axial metres against the column index of row 120. The horizontal ticks are indexes, not metres. A no-return is omitted rather than drawn as 0 m. `depth-grid.svg` casts every 16th pixel. Each cell is that cast. The stride does not interpolate. The demo does not cast all 320 by 240 pixels. That cap is not a timing benchmark.

## What this does not claim

No lens distortion, no noise, no exposure, and no photoreal colour. No person, bag, or policy is read from the depth. The two mounts are not an installed extrinsics calibration. A simulation hit is not a device sample. Section 3.1 stays 45 pass and 11 not-run. Stage 5, replaying a real recording through this path, is not built. Stages 6 and 7 are not closed.

## Decisions

The posted sample keeps a below-gate millimetre value instead of copying an unread device zero. The principal point stays the image center already used for bearings, and the 153.6 row fit is left unused. Plates can occlude a wall. The camera body cannot. The picture uses the measured mount and leaves the configured mount in the same profile list.
