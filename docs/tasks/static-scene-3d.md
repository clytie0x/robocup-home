# Static 3D scene

This slice builds one synthetic room-scale fixture, casts lidar rays at the boxes, encodes those ranges as LDS-03 packets, and posts the parsed packets through the same `frame_sample` and `post_samples` path as a captured scan. It is the first static geometry slice. It is not a robot mesh, not a physics engine, not an interactive viewer, and not a mission.

## Frames

World axes are x forward, y left, z up, metres, right-handed. For this fixture the synthetic base is the same frame as the world. That coincidence is labeled synthetic. It is not a measured installation.

The scan frame origin is the sensor. Scan x is sensor-forward, scan y is sensor-left, and scan z is sensor-up. A bearing is counterclockwise from sensor +x in the scan plane. The packet angle is clockwise, and `frame_sample` recovers the bearing with `wrap(front - raw)`.

The mount is one named rigid transform: translation of the sensor in the base, yaw counterclockwise about z, and pitch down about the sensor y axis. The rotation is `Rz(yaw) * Ry(pitch_down)`. Positive pitch down tilts sensor +x toward -z. The world ray is that rotation applied to the scan-plane direction `(cos bearing, sin bearing, 0)`. The reported range is the distance along that 3D line. `frame_sample` then places the range back in the scan plane. A world top-down and a scan plot are different origins.

The demonstration mount is `synthetic-lidar` at `(0.40, -0.12, 0.50)` m, yaw 15 deg, pitch 0. Its provenance string says it is a synthetic fixture. It is not the configured `LIDAR_X_M` of 0.175 m, and it is not an installed measurement. The installed lever arm stays unknown. This slice does not write 0 or 0.175 into that unknown. Wheels, mass, and the arm mount stay unknown. No chassis outline is drawn as a validated body.

## What is intersected

The bodies are axis-aligned boxes. The caster keeps the nearest positive entry into a box, including the z slab. A box that does not cross the ray in z is a miss. A nearer face hides a farther face. An empty body list is a miss. A non-finite mount, a non-finite bearing, or a negative maximum range is an error and does not become a sample. A miss is encoded as range 0, which the adapter already treats as a zero return rather than a body.

The demonstration has two boxes. `pillar` spans x 1.60–1.90 m, y -0.25–0.25 m, z 0–1.20 m. `low-plank` spans x 1.10–1.30 m, y -0.40–0.40 m, z 0–0.20 m. The horizontal beam is at z 0.50 m, so the plank is below it and the pillar crosses it. Those sizes are chosen for the fixture. They are not a measured room.

The navigation policy never receives these boxes. It receives the posted scan points, or nothing. A test may add a fixture person after the view exists. That person is an injected observation. It is not a camera detection and not a perception result.

## Pictures

`static-scene` writes four SVG files from the 3D coordinates:

- `oblique.svg` is a cabinet projection. Changing z moves the screen point. Red rays use the named mount yaw. Blue rays use yaw 0 at the same translation. Box edges, axes, the mount name, and the box dimensions are drawn.
- `top.svg` is the world top-down. Its origin is the synthetic base, not the sensor.
- `side.svg` is world x and z. The beam is horizontal, the plank is below it, and the pillar crosses it.
- `scan.svg` is the scan frame after `frame_sample`. Its origin is the sensor. The dashed circle is the planner's 250 mm assumption around that posted origin. It is not a measured chassis and it does not certify clearance.

These files are projections of the coordinates. They are not a text schematic and not a photograph. `rays.csv` holds the geometric hit, in metres, and the scan-frame point. `packet-points.csv` holds the quantized packet range and the adapter classification. `posted.csv` is the `post_samples` result. `mount.csv` records the transform and says the installed lever arm is unknown.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin static-scene --locked --offline -- --out OUT
```

The scene is compiled into the binary. There is no capture argument. The clock is the supplied 2500 ms injected replay clock, not wall time. The front is the explicit `measured-front` profile, 178.7 deg. The other front is not selected. Ranges are quantized by the existing divisor 4000 when they are written into a packet. That divisor is the protocol. It is not a tape calibration. Exit 0 writes the files. Exit 2 is a usage error. Exit 1 is a cast or write error.

The fresh post has no person, so `plan_follow` returns `Lost`. `sent` is false. There is no command text and no port.

## What a passing run shows

A hit range is the distance to a box face along the 3D ray. The tests check a wall at 2.000 m, a yaw of 90 deg, a pitched ray against an independent matrix, occlusion of a far face by a near face, a low box that the beam misses until its top is raised through the beam, and an empty scene. The same tests post the resulting range through `frame_sample`. A 0.15 m hit becomes the scan point `(150, 0)` mm, not the world point `(550, 100)` mm. With a fixture person ahead, that point changes a straight `Move` into `Blocked` or a side step, and removing the point restores the straight `Move`.

The packet test encodes the cast ranges, parses them back, and checks the parsed range against the quantized ray. It does not store a hand-written hit list for the planner.

## What it does not show

It does not validate the installed lidar lever, the chassis, the wheels, the mass, or the arm mount. It does not claim that the 250 mm planner disk clears the physical robot. It does not decode a camera image. The seven old camera PNGs and method-8 `camera.npz` stay blocked. It does not simulate dynamics, contact, or a full Carry My Luggage run. Section 3.1 stays 45 `pass` and 11 `not-run`. A later controller must still be able to run without this scene's boxes.
