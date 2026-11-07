# Camera frame

This slice fills one 320 by 240 synthetic image and one axial-depth buffer from the pinhole in [camera-depth.md](camera-depth.md). Every pixel calls that cast. The colour is an assumed flat colour of the nearest solid. It is not a photograph, not a calibrated camera, not a decoded recording, and not a detector. Stage 4 is not complete.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin camera-frame --locked --offline -- --out OUT
```

`OUT` is a directory. The program creates it. Exit 0 writes `frame-model.csv`, `measured.rcf`, `configured.rcf`, `faulted.rcf`, `measured-rgb.ppm`, `measured-class.ppm`, `configured-rgb.ppm`, `configured-class.ppm`, `fault.csv`, `class-legend.svg`, `fault.svg`, and `timing.txt`. Exit 2 is a usage error. Exit 1 is a cast or write error. Nothing is read from a capture and nothing is sent.

## What was taken from the robot files

The private files are not in git. The plan in [WORKLOG.md](../../WORKLOG.md) names each one. `sensors.json` calls the device an ASUS Xtion class RGB-D at 320 by 240. `camera/index.json` lists the same size and was not used to open a PNG. The session README describes `camera.npz` as 39 colour frames of 240 by 320 by 3 `uint8` and 39 depth frames of `uint16` millimetres, with one monotonic time array. This slice does not open that archive, so it does not verify channel order, endianness, alignment, or a shared shutter. The seven `camera/*.png` files stay unread. The recorded optical row of about 153.6 is not applied.

The fields, the mirror, the 0.5 m gate, and the two mounts are the values already frozen for the depth slice. Material colour is unknown. The palette in this file is an assumption.

## The buffers

Width is 320 and height is 240. The count is 76800. The function does not take a larger size. Index `row * 320 + col` is the center of that pixel. Row 0 is the top row, because image down grows with the row index in the existing tangent. Image right is robot left. That mirror is inside `cast_pixel` and is not applied a second time.

Depth is a little-endian `u16` in millimetres. Posted 0 is no return. It is not a distance of 0 m. A positive word under 500 mm stays in the buffer and is `BelowGate`. It is not usable depth. A word at or above 500 mm is `Valid`. The word still cannot store a hit past 65.535 m, so that hit posts 0. That is the existing word, not a new far cutoff.

RGB is three bytes, R then G then B, at the same index. The colour is the assumed flat colour of the nearest box or plate. A miss is `(0, 0, 0)`. A solid that is not in the palette is one shared grey, not a hash of its name. The posted bytes do not contain a body name, a box id, or a class label. A fixture colour is render input. It is not a semantic channel and not the depth class.

The two buffers match because each pixel is one call to `cast_pixel`. That registration is by construction. It is not a measured extrinsics calibration and not a claim that a real colour stream and a real depth stream share a shutter.

`measured-mount` and `configured-mount` both cast. Neither is chosen as the installed camera. The synthetic stamp is 0. It is not a capture clock. Depth freshness and RGB freshness are separate. Marking one stale does not change the other and does not change the stamp.

## Faults

Depth dropout zeros the posted millimetre and leaves the colour. RGB dropout zeros the three colour bytes and leaves the posted millimetre. The same index cannot be on both lists. A black pixel with a positive depth word is an RGB dropout, not a miss. A coloured pixel with posted 0 is a depth dropout, not a depth of 0 m. A geometric miss is black and posted 0, and it is not on the fault list.

A stale depth buffer keeps every word and contributes no fresh valid depth. A stale RGB buffer keeps its bytes. `controller_note` reads the depth view only. It does not read RGB, and it still reports no person and no sent command.

## File

`RCF1` version 1 is little-endian. The header is 32 bytes: magic, version, width, height, depth freshness, RGB freshness, depth age in milliseconds, RGB age in milliseconds, and the synthetic stamp. Freshness codes are 0 fresh, 1 stale, 2 missing, 3 malformed, and 4 reversed. Then 76800 depth words, then 76800 RGB triples. It is not an `.npy` and not a camera PNG.

`measured-rgb.ppm` is a view of the RGB bytes. `measured-class.ppm` is the posted class: blue at or above 500 mm, amber from 1 to 499 mm, and gray for posted 0. The class picture is 320 by 240. Its legend is a separate SVG and is not extra rows in the buffer. Gray means no return, not zero metres. The class colours are not the fixture colours.

`fault.svg` is the two demo faults. The depth-dropout row shows the ideal millimetres as not posted. Those millimetres are not a column in `fault.csv` and are not the posted word. The demo drops column 0 of row 120 on depth and column 319 of row 120 on colour, then marks the depth buffer stale and leaves the colour buffer fresh.

`timing.txt` is the elapsed time of the two casts on the machine that wrote the directory. It is not an acceptance threshold.

## What this does not claim

No lens distortion, no noise, no exposure, and no photoreal colour. No person, bag, or policy is read from the frame. The two mounts are not an installed extrinsics calibration. A simulation hit is not a device sample. The depth-slice demo, one row plus a stride of 16, is unchanged and is not this frame. Section 3.1 stays 45 pass and 11 not-run. Stage 4 is not complete. Stage 5, replaying a real recording through this path, is not built.
