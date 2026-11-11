# Download fixtures

Generated locally with ffmpeg `lavfi testsrc` (64×48). These files carry no third-party content. ffmpeg is not a runtime or test dependency.

- `sample.png`, `sample.jpg`: one test pattern frame.
- `truncated.jpg`: the first 80 bytes of `sample.jpg`; must fail complete decoding.
- `sample.webm`: two VP9 frames at 4 fps, yuv420p; exercises browser first-frame decoding.
- `sample-h264.mp4`: two H.264 Constrained Baseline frames at 4 fps, 64×48, yuv420p. Generated once with `ffmpeg -f lavfi -i 'testsrc=size=64x48:rate=4' -t 0.5 -c:v libx264 -profile:v baseline -level:v 1.0 -pix_fmt yuv420p -movflags +faststart -an sample-h264.mp4`.

The H.264 acceptance test records `passed` only after decoding and registering both video and poster. Missing browsers or explicit decoder errors record `unverified` after checking failure cleanup; other errors fail the test. On GitHub Actions, the test writes `<target>-media-h264.json` to the existing `local/phoenix-ui/ci-prepare/` artifact directory, with workflow and commit identity. Captured test stdout is not the CI evidence source.
