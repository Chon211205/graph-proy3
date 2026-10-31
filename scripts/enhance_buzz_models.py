"""Create more detailed Buzz vehicle variants while preserving the source models."""

from __future__ import annotations

from pathlib import Path

from enhance_astra_models import Appender


ROOT = Path(__file__).resolve().parents[1]
BUZZ_DIR = ROOT / "assets" / "vehicles" / "buzz"


class BuzzAppender(Appender):
    def write(self, target: Path):
        lines = self.text.splitlines()
        for index, line in enumerate(lines):
            if line.startswith("mtllib "):
                lines[index] = f"mtllib {target.with_suffix('.mtl').name}"
                break
        target.write_text("\n".join(lines) + "\n" + "\n".join(self.parts) + "\n", encoding="utf-8")


MTL = """# Detailed Buzz Lightyear vehicle materials
newmtl buzz_white
Kd 0.82 0.87 0.91
Ks 0.86 0.90 0.94
Ns 220
illum 2

newmtl buzz_green
Kd 0.38 0.76 0.055
Ks 0.58 0.84 0.30
Ns 190
illum 2

newmtl buzz_purple
Kd 0.34 0.10 0.62
Ks 0.64 0.38 0.88
Ns 185
illum 2

newmtl buzz_red
Kd 0.88 0.035 0.055
Ks 0.82 0.24 0.28
Ns 175
illum 2

newmtl buzz_glass
Kd 0.04 0.24 0.34
Ks 0.78 0.92 1.00
Ns 230
d 0.78
Ni 1.46
illum 9

newmtl buzz_dark
Kd 0.025 0.035 0.055
Ks 0.40 0.48 0.58
Ns 115
illum 2

newmtl buzz_blue
Kd 0.035 0.28 0.82
Ks 0.30 0.62 0.94
Ns 170
illum 2

newmtl buzz_gold
Kd 0.96 0.64 0.06
Ks 0.88 0.68 0.26
Ns 205
illum 2

newmtl buzz_emission
Kd 0.48 1.00 0.10
Ke 0.28 0.92 0.04
Ns 210
illum 2
"""


def spaceship() -> BuzzAppender:
    model = BuzzAppender(BUZZ_DIR / "buzz_nave.obj")
    for side, x in (("L", -2.72), ("R", 2.72)):
        model.tube_z(f"Buzz_Ship_Engine_{side}", "buzz_dark", (x, 0.02, -3.78), 0.48, 0.64, 30)
        model.torus_z(f"Buzz_Ship_Guard_{side}", "buzz_purple", (x, 0.02, -4.02), 0.56, 0.075, 38)
        model.tube_z(f"Buzz_Ship_Core_{side}", "buzz_emission", (x, 0.02, -4.13), 0.30, 0.08, 26)
        model.box(f"Buzz_Ship_WingRail_{side}", "buzz_green", (x * 1.45, 0.22, -0.15), (0.15, 0.16, 4.7))
        model.sphere(f"Buzz_Ship_WingLight_{side}", "buzz_emission", (x * 2.25, 0.06, 0.28), (0.18, 0.14, 0.26), 20, 10)
    model.sphere("Buzz_Ship_CanopyLayer", "buzz_glass", (0, 1.25, 0.55), (0.84, 0.46, 1.25), 32, 16)
    model.box("Buzz_Ship_DorsalArmor", "buzz_white", (0, 1.70, -0.32), (1.25, 0.14, 3.3))
    model.box("Buzz_Ship_PurpleSpine", "buzz_purple", (0, 1.80, -0.48), (0.24, 0.10, 2.9))
    return model


def car() -> BuzzAppender:
    model = BuzzAppender(BUZZ_DIR / "buzz_carro.obj")
    for axle, z in (("Front", 2.15), ("Rear", -2.15)):
        for side, x in (("L", -2.45), ("R", 2.45)):
            model.tube_x(f"Buzz_Car_Rim_{axle}_{side}", "buzz_white", (x, -0.66, z), 0.54, 0.14, 32)
            model.tube_x(f"Buzz_Car_Hub_{axle}_{side}", "buzz_purple", (x * 1.02, -0.66, z), 0.23, 0.16, 24)
    for side, x in (("L", -0.88), ("R", 0.88)):
        model.sphere(f"Buzz_Car_Headlight_{side}", "buzz_emission", (x, 0.18, 3.68), (0.30, 0.19, 0.10), 22, 11)
        model.sphere(f"Buzz_Car_Taillight_{side}", "buzz_red", (x, 0.10, -3.54), (0.28, 0.17, 0.10), 22, 11)
        model.tube_z(f"Buzz_Car_Engine_{side}", "buzz_dark", (x, -0.12, -3.68), 0.22, 0.34, 24)
        model.tube_z(f"Buzz_Car_Core_{side}", "buzz_emission", (x, -0.12, -3.88), 0.14, 0.06, 22)
    model.sphere("Buzz_Car_CanopyLayer", "buzz_glass", (0, 1.50, 0.18), (0.90, 0.38, 1.25), 30, 15)
    model.box("Buzz_Car_Grille", "buzz_dark", (0, -0.02, 3.72), (1.32, 0.30, 0.08))
    model.box("Buzz_Car_GreenStripe", "buzz_green", (0, 1.84, 0.15), (0.25, 0.08, 3.25))
    return model


def submarine() -> BuzzAppender:
    model = BuzzAppender(BUZZ_DIR / "buzz_submarino.obj")
    for side, x in (("L", -2.28), ("R", 2.28)):
        model.tube_z(f"Buzz_Sub_Propeller_{side}", "buzz_dark", (x, -0.05, -3.76), 0.48, 0.62, 30)
        model.torus_z(f"Buzz_Sub_Guard_{side}", "buzz_purple", (x, -0.05, -4.02), 0.67, 0.085, 40)
        model.tube_z(f"Buzz_Sub_Core_{side}", "buzz_emission", (x, -0.05, -4.14), 0.29, 0.08, 26)
        model.sphere(f"Buzz_Sub_Porthole_{side}", "buzz_glass", (x * 1.10, 0.68, 0.36), (0.18, 0.38, 0.38), 24, 12)
    for side, x in (("L", -0.72), ("R", 0.72)):
        model.sphere(f"Buzz_Sub_Headlight_{side}", "buzz_emission", (x, 0.16, 3.70), (0.27, 0.21, 0.11), 22, 11)
    model.sphere("Buzz_Sub_CanopyLayer", "buzz_glass", (0, 1.62, 0.26), (0.86, 0.43, 1.20), 30, 15)
    model.box("Buzz_Sub_DorsalRail", "buzz_green", (0, 2.45, 0.05), (0.24, 0.15, 3.45))
    model.tube_z("Buzz_Sub_Sonar", "buzz_white", (0, 2.76, -0.34), 0.19, 0.48, 22)
    return model


def generate(filename: str, builder) -> None:
    target = BUZZ_DIR / filename
    builder().write(target)
    target.with_suffix(".mtl").write_text(MTL, encoding="utf-8")
    print(f"Generated {target.relative_to(ROOT)}")


if __name__ == "__main__":
    generate("buzz_nave_detallada.obj", spaceship)
    generate("buzz_carro_detallado.obj", car)
    generate("buzz_submarino_detallado.obj", submarine)
