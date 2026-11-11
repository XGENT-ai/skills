# Download fixtures

Generated locally with ffmpeg `lavfi testsrc` (64×48). These files carry no third-party content. ffmpeg is not a runtime or test dependency.

- `sample.png`, `sample.jpg`: one test pattern frame.
- `truncated.jpg`: the first 80 bytes of `sample.jpg`; must fail complete decoding.
- `sample.webm`: two VP9 frames at 4 fps, yuv420p; exercises browser first-frame decoding.
