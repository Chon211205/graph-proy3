"""Cut the near-white exterior from the Tower: Ultima Guardia character art.

The source PNGs are kept unchanged. Pillow's flood fill starts at the canvas
corners, so enclosed whites (teeth, eyes, cloth highlights) remain opaque.
Run from any directory with: python scripts/prepare_tower_guard_sprites.py
"""

from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "assets/minigames/tower_guardian/enemy_manifest.tsv"
PAD = 10
WHITE_THRESHOLD = 42


def entries():
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        fields = line.split("|")
        if len(fields) != 8:
            raise ValueError(f"Invalid character manifest row: {line}")
        yield fields


def prepare(source: Path, destination: Path) -> tuple[int, int]:
    image = Image.open(source).convert("RGBA")
    transparent = (0, 0, 0, 0)

    # Flood only from the exterior; isolated near-white interior details stay.
    for seed in ((0, 0), (image.width - 1, 0), (0, image.height - 1), (image.width - 1, image.height - 1)):
        ImageDraw.floodfill(image, seed, transparent, thresh=WHITE_THRESHOLD)

    alpha = image.getchannel("A")
    bounds = alpha.getbbox()
    if bounds is None:
        raise ValueError(f"No opaque character pixels remain in {source}")
    left, top, right, bottom = bounds
    left = max(0, left - PAD)
    top = max(0, top - PAD)
    right = min(image.width, right + PAD)
    bottom = min(image.height, bottom + PAD)
    image = image.crop((left, top, right, bottom))
    destination.parent.mkdir(parents=True, exist_ok=True)
    image.save(destination, optimize=True)
    return image.size


def main() -> None:
    for row in entries():
        character_id, display, source_rel, sprite_rel, *_ = row
        size = prepare(ROOT / source_rel, ROOT / sprite_rel)
        print(f"{character_id}: {display} -> {size[0]}x{size[1]}")


if __name__ == "__main__":
    main()
