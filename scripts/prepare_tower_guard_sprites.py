"""Cut the near-white exterior from the Tower: Ultima Guardia character art.

The source PNGs are kept unchanged. Pillow's flood fill starts at the canvas
corners, so enclosed whites (teeth, eyes, cloth highlights) remain opaque.
Run from any directory with: python scripts/prepare_tower_guard_sprites.py
"""

from pathlib import Path
from collections import deque
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "assets/minigames/tower_guardian/enemy_manifest.tsv"
PAD = 10
WHITE_THRESHOLD = 42
ENCLOSED_WHITE_MIN_AREA = 96


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

    # Some illustrations contain large, enclosed pockets of the white source
    # canvas between limbs. Remove those matte islands while preserving the
    # small white details (teeth, eyes and highlights) in the face area.
    pixels = image.load()
    width, height = image.size
    visited = bytearray(width * height)
    for start_y in range(height):
        for start_x in range(width):
            start = start_y * width + start_x
            if visited[start]:
                continue
            r, g, b, alpha = pixels[start_x, start_y]
            if alpha == 0 or min(r, g, b) < 238 or max(r, g, b) - min(r, g, b) > 22:
                visited[start] = 1
                continue

            component = []
            min_x = max_x = start_x
            min_y = max_y = start_y
            queue = deque([start])
            visited[start] = 1
            while queue:
                index = queue.popleft()
                component.append(index)
                x, y = index % width, index // width
                min_x, max_x = min(min_x, x), max(max_x, x)
                min_y, max_y = min(min_y, y), max(max_y, y)
                for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                    if not (0 <= nx < width and 0 <= ny < height):
                        continue
                    neighbor = ny * width + nx
                    if visited[neighbor]:
                        continue
                    nr, ng, nb, na = pixels[nx, ny]
                    visited[neighbor] = 1
                    if na and min(nr, ng, nb) >= 238 and max(nr, ng, nb) - min(nr, ng, nb) <= 22:
                        queue.append(neighbor)

            center_y = (min_y + max_y) * 0.5
            box_width, box_height = max_x - min_x + 1, max_y - min_y + 1
            aspect = box_width / box_height
            fill_ratio = len(component) / (box_width * box_height)
            looks_like_matte_hole = 0.48 <= aspect <= 2.1 and fill_ratio >= 0.42
            if (
                len(component) >= ENCLOSED_WHITE_MIN_AREA
                and center_y >= height * 0.38
                and looks_like_matte_hole
            ):
                for index in component:
                    x, y = index % width, index // width
                    pixels[x, y] = transparent

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
