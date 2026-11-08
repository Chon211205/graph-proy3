"""Build transparent result-card titles from the supplied YOU DIED reference."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/minigames/tower_guardian/source/you-died-reference.png"
OUTPUT = ROOT / "assets/minigames/tower_guardian/ui"
INK = (90, 0, 6)


def prepare_died() -> None:
    image = Image.open(SOURCE).convert("RGB")
    red_pixels = [
        (x, y)
        for y in range(image.height)
        for x in range(image.width)
        if image.getpixel((x, y))[0] > 20
        and image.getpixel((x, y))[0] > image.getpixel((x, y))[1] * 2
    ]
    left = min(x for x, _ in red_pixels)
    top = min(y for _, y in red_pixels)
    right = max(x for x, _ in red_pixels) + 1
    bottom = max(y for _, y in red_pixels) + 1
    crop = image.crop((left, top, right, bottom))
    result = Image.new("RGBA", crop.size, (0, 0, 0, 0))
    src, dst = crop.load(), result.load()
    for y in range(crop.height):
        for x in range(crop.width):
            red, green, blue = src[x, y]
            alpha = min(255, max(0, round(red * 255 / INK[0])))
            if alpha and red > green * 2:
                dst[x, y] = (*INK, alpha)
    result.save(OUTPUT / "you-died.png", optimize=True)


def prepare_win() -> None:
    font_path = Path("C:/Windows/Fonts/times.ttf")
    font = ImageFont.truetype(str(font_path), 58)
    probe = Image.new("RGBA", (8, 8))
    bounds = ImageDraw.Draw(probe).textbbox((0, 0), "YOU WIN", font=font)
    width, height = bounds[2] - bounds[0] + 8, bounds[3] - bounds[1] + 8
    result = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    ImageDraw.Draw(result).text((4 - bounds[0], 4 - bounds[1]), "YOU WIN", font=font, fill=(*INK, 255))
    result.save(OUTPUT / "you-win.png", optimize=True)


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    prepare_died()
    prepare_win()


if __name__ == "__main__":
    main()
