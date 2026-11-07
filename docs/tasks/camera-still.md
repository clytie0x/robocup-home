# Saved still pairs

This slice reads the seven saved colour and depth pairs. It does not read `camera.npz`. It does not call `cast_pixel`, `controller_note`, or a task crate. It does not write `RCF1`. A decoded pair is not a fresh observation, not a device clock, and not a person. Stage 4 is not complete. Stage 5 is not complete.

## How to run

From the repository root, with the pinned toolchain:

```text
cargo run -p physical --bin still-pair --locked --offline -- PNG DEPTH
```

Exit 0 prints one line: width, height, the SHA-256 of the 230400 RGB bytes, the SHA-256 of the 153600 little-endian depth bytes, and the counts of posted 0, words from 1 to 499, and words at or above 500. Exit 1 is a read or decode error. Exit 2 is a usage error. The program does not print pixels.

`--check-rgb RGBFILE --check-depth DEPTHFILE` compares those files with the decoded buffers and prints `rgb_mismatch` and `depth_mismatch` as `none` or a byte index. It still does not print a pixel.

## What the files are

The plan in [WORKLOG.md](../../WORKLOG.md) measured the seven pairs. Each PNG is 320 by 240, bit depth 8, colour type 2, compression 0, filter method 0, and no interlace. The inflated scan is 230640 bytes. Every filter byte is 0. Each depth file is a NumPy 1.0 `<u2` array, C order, shape `(240, 320)`, and the file ends at the payload. `index.json` stores `size` as `[320, 240]`. That tuple is not the array shape. `load_depth_u16` still accepts a swapped rank-2 shape. `read_still_depth` does not.

PNG colour type 2 is R, then G, then B ([PNG specification](https://www.w3.org/TR/png-3/)). The capture script names the OpenNI format-200 buffer `rgb`. No colour chart in that plan checks a known red object. The same row order for colour and depth is the writer's loop, not a measured homography. PNG scanline 0 is the top row. Posted 0 stays 0. A word of 486 stays 486.

The IDAT is zlib-wrapped deflate ([RFC 1950](https://www.rfc-editor.org/rfc/rfc1950), [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951)). The inflater is in this crate. It follows the puff bit reader and the canonical Huffman decode, and it stops at 230640 output bytes. `miniz_oxide` and `flate2` were not added. Either one would be this workspace's first crates.io dependency, and `artifacts/check_workspace.py` rejects a dependency that has a source. OpenCV is allowed by the repository rules and is not installed. This measured subset does not need it. The test encoder writes stored blocks only. One fixture is a 262-byte dynamic block from Node `zlib.deflateSync`, and the fixed-Huffman check is the separate `hello` stream from that same library.

## The buffers

`read_still_pair` returns width 320, height 240, 76800 little-endian depth words, and 230400 RGB bytes. Index `row * 320 + col` is that pixel. The counts do not rewrite a word. There is no stamp and no freshness byte.

A PNG fails closed unless the signature matches, the only chunks are one `IHDR`, one or more consecutive `IDAT` chunks, and one empty `IEND`, every chunk CRC matches, and nothing follows `IEND`. The header must be the measured subset. A claimed chunk length above 262144 bytes is rejected before a buffer of that length is taken from the file. The zlib header must be method 8, with no preset dictionary, and the Adler32 must match. Truncated input, leftover compressed bytes, a filter byte other than 0, and an inflate past 230640 bytes are rejected. The depth file must be `<u2`, not Fortran, shape `[240, 320]`, and no byte may follow the payload.

## What this does not claim

The pair is not registered colour and depth. The script did not set image registration, and these files do not store the two OpenNI timestamps. The stills have a laptop-local `saved` string. That string is not a capture clock. Mirror, field of view, and the two mounts stay the frozen pinhole values. They are not fields in these files. The optical row 153.6 is not in the arrays. `camera.npz` stays blocked. `zip_entries` still stops on method 8 and on a size of `u32::MAX`. Section 3.1 stays 45 pass and 11 not-run. Stage 4 is not complete. Stage 5 is not complete.

| requirement | source | test | result | gate |
| --- | --- | --- | --- | --- |
| Colour is 320 by 240, 8-bit, colour type 2, no interlace, filter byte 0, row-major R then G then B | PNG specification, colour type 2 and filter type 0. The measured IHDR in the decode plan | `a_dynamic_png_keeps_channel_order_and_row_major_pixels`, `a_stored_png_round_trips_every_pixel`, `two_idat_chunks_are_one_stream` | pass, see WORKLOG | sim |
| A bad CRC, a bad Adler32, a short stream, leftover compressed bytes, a byte after `IEND`, a non-zero filter, another colour type, interlace, a second `IHDR`, an `IDAT` before `IHDR`, a short chunk, a huge chunk length, and an inflate past 230640 bytes are rejected | PNG chunk CRC. RFC 1950 Adler32. RFC 1951 block types. The measured subset in the decode plan | `a_bad_chunk_crc_is_rejected`, `a_bad_adler_is_rejected`, `a_truncated_zlib_is_rejected`, `trailing_compressed_bytes_are_rejected`, `trailing_file_bytes_are_rejected`, `a_non_zero_filter_is_rejected`, `colour_type_and_interlace_outside_the_measured_subset_are_rejected`, `a_second_ihdr_and_an_idat_before_ihdr_are_rejected`, `a_chunk_length_past_the_file_is_rejected_without_a_huge_buffer`, `an_inflated_stream_past_the_scan_cap_is_rejected`, `output_past_the_cap_is_rejected` | pass, see WORKLOG | sim |
| Depth is `<u2`, C order, shape `(240, 320)`. Shape `(320, 240)`, Fortran order, a big-endian descr, and a trailing byte are rejected. Words 0 and 486 stay unchanged | NumPy `.npy` format. The measured header in the decode plan. `load_depth_u16` still accepts the swapped shape | `depth_keeps_zero_and_486_and_rejects_a_swapped_shape`, `a_pair_keeps_the_png_bytes_and_the_depth_words` | pass, see WORKLOG | sim |
| The stored test encoder is not the dynamic fixture. A fixed-Huffman stream from Node zlib inflates to `hello` | RFC 1951 block types. Node `zlib.deflateSync` for the dynamic fixture and for `hello` | `a_fixed_huffman_hello_is_not_the_stored_test_encoder`, `a_dynamic_png_keeps_channel_order_and_row_major_pixels` | pass, see WORKLOG | sim |
| The seven private pairs match an independent Node decode on every RGB byte and every depth word | The files named in the decode plan. They stay outside git | `compare.mjs` against `still-pair --check-rgb --check-depth` | pass, see WORKLOG | sim |
