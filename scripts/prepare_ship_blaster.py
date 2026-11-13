"""Prepare a short, repeatable ship-laser cue from the supplied WAV recording.

The source recording is kept unchanged. This script extracts its opening shot,
applies a short fade to avoid clicks, and writes the compact PCM cue used by the
game's low-latency Windows playback path.
"""

from array import array
from pathlib import Path
import wave


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets" / "sounds" / "ship_blaster_source.wav"
OUTPUT = ROOT / "assets" / "sounds" / "ship_blaster.wav"
START_SECONDS = 0.46
END_SECONDS = 1.96
FADE_IN_SECONDS = 0.008
FADE_OUT_SECONDS = 0.12
GAIN = 0.72


def main() -> None:
    with wave.open(str(SOURCE), "rb") as source:
        if source.getsampwidth() != 2 or source.getcomptype() != "NONE":
            raise ValueError("Expected uncompressed 16-bit PCM WAV input")

        params = source.getparams()
        channels = params.nchannels
        rate = params.framerate
        total_frames = source.getnframes()
        start = min(int(START_SECONDS * rate), total_frames)
        end = min(int(END_SECONDS * rate), total_frames)
        if end <= start:
            raise ValueError("The source WAV is shorter than the selected shot")

        source.setpos(start)
        samples = array("h")
        samples.frombytes(source.readframes(end - start))

    frame_count = len(samples) // channels
    fade_in = max(1, int(FADE_IN_SECONDS * rate))
    fade_out = max(1, int(FADE_OUT_SECONDS * rate))
    for frame in range(frame_count):
        gain = GAIN
        if frame < fade_in:
            gain *= frame / fade_in
        remaining = frame_count - frame
        if remaining < fade_out:
            gain *= remaining / fade_out
        for channel in range(channels):
            index = frame * channels + channel
            samples[index] = max(-32768, min(32767, round(samples[index] * gain)))

    with wave.open(str(OUTPUT), "wb") as output:
        output.setnchannels(channels)
        output.setsampwidth(2)
        output.setframerate(rate)
        output.writeframes(samples.tobytes())

    print(f"Wrote {OUTPUT.relative_to(ROOT)} ({frame_count / rate:.2f}s)")


if __name__ == "__main__":
    main()
