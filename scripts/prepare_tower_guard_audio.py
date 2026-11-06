"""Generate short, deterministic PCM feedback sounds for Torre: Última Guardia."""

from math import pi, sin
from pathlib import Path
import struct
import wave


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/minigames/tower_guardian/audio"
RATE = 22050


def synth(kind: str, duration: float, low: float, high: float, overtone: float = 0.0) -> bytes:
    count = round(RATE * duration)
    samples = bytearray()
    phase = 0.0
    for i in range(count):
        t = i / RATE
        progress = i / max(count - 1, 1)
        frequency = low + (high - low) * progress
        phase += 2 * pi * frequency / RATE
        envelope = (1.0 - progress) ** (2.2 if kind == "shot" else 1.6)
        tone = sin(phase) + overtone * sin(phase * 2.01)
        grit = (((i * 1103515245 + 12345) >> 16) & 255) / 127.5 - 1.0
        if kind in ("shot", "damage"):
            tone = tone * 0.7 + grit * 0.3 * (1.0 - progress)
        value = max(-1.0, min(1.0, tone * envelope * 0.72))
        samples.extend(struct.pack("<h", int(value * 32767)))
    return bytes(samples)


def write(name: str, raw: bytes) -> None:
    with wave.open(str(OUT / name), "wb") as audio:
        audio.setnchannels(1)
        audio.setsampwidth(2)
        audio.setframerate(RATE)
        audio.writeframes(raw)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    write("shot.wav", synth("shot", 0.105, 980, 170, 0.24))
    write("impact.wav", synth("impact", 0.13, 720, 360, 0.32))
    write("damage.wav", synth("damage", 0.24, 210, 72, 0.44))
    write("boss-alert.wav", synth("alert", 0.32, 320, 610, 0.28))
    print(f"Generated four feedback cues in {OUT}")


if __name__ == "__main__":
    main()
