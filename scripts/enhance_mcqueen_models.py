"""Create detailed Rayo McQueen vehicle variants and matching materials."""

from __future__ import annotations

from pathlib import Path

from enhance_astra_models import Appender


ROOT = Path(__file__).resolve().parents[1]
MCQUEEN_DIR = ROOT / "assets" / "vehicles" / "mcqueen"


MTL = """# Detailed Rayo McQueen materials
newmtl mcqueen_red
Kd 0.82 0.025 0.035
Ks 0.92 0.28 0.24
Ns 225
illum 2

newmtl mcqueen_darkred
Kd 0.32 0.012 0.020
Ks 0.68 0.16 0.15
Ns 175
illum 2

newmtl mcqueen_yellow
Kd 1.00 0.70 0.035
Ks 0.92 0.72 0.26
Ns 210
illum 2

newmtl mcqueen_orange
Kd 1.00 0.22 0.018
Ke 0.82 0.10 0.005
Ns 190
illum 2

newmtl mcqueen_white
Kd 0.92 0.95 0.98
Ks 0.88 0.92 0.96
Ns 220
illum 2

newmtl mcqueen_glass
Kd 0.035 0.22 0.40
Ks 0.78 0.92 1.00
Ns 230
d 0.80
Ni 1.46
illum 9

newmtl mcqueen_blue
Kd 0.035 0.24 0.82
Ks 0.28 0.58 0.96
Ns 175
illum 2

newmtl mcqueen_dark
Kd 0.022 0.026 0.035
Ks 0.42 0.48 0.56
Ns 120
illum 2

newmtl mcqueen_emission
Kd 1.00 0.32 0.025
Ke 1.00 0.18 0.008
Ns 220
illum 2
"""


def replace_group_material(model: Appender, groups: set[str], material: str) -> None:
    """Replace glass on eye-bearing windshield groups before details are appended."""
    current_group = ""
    rewritten = []
    for line in model.text.splitlines():
        if line.startswith("g "):
            current_group = line[2:].strip()
        if line.startswith("usemtl ") and current_group in groups:
            line = f"usemtl {material}"
        rewritten.append(line)
    model.text = "\n".join(rewritten)


def spaceship() -> Appender:
    model = Appender(MCQUEEN_DIR / "mcqueen_nave.obj")
    replace_group_material(model, {"Cabina_azul"}, "mcqueen_blue")
    for side, x in (("L", -1.95), ("R", 1.95)):
        model.tube_z(f"McQueen_Ship_Engine_{side}", "mcqueen_dark", (x, -0.02, -3.78), 0.46, 0.60, 30)
        model.torus_z(f"McQueen_Ship_Guard_{side}", "mcqueen_darkred", (x, -0.02, -4.00), 0.54, 0.075, 38)
        model.tube_z(f"McQueen_Ship_Core_{side}", "mcqueen_emission", (x, -0.02, -4.11), 0.29, 0.08, 26)
        model.box(f"McQueen_Ship_WingRail_{side}", "mcqueen_yellow", (x * 1.52, 0.18, -0.05), (0.14, 0.13, 4.5))
        model.sphere(f"McQueen_Ship_WingLight_{side}", "mcqueen_emission", (x * 2.75, 0.02, 0.30), (0.18, 0.14, 0.28), 20, 10)
    model.box("McQueen_Ship_DorsalArmor", "mcqueen_red", (0, 1.57, -0.32), (1.20, 0.14, 3.15))
    model.box("McQueen_Ship_95Stripe", "mcqueen_yellow", (0, 1.68, -0.42), (0.22, 0.09, 2.65))
    return model


def car() -> Appender:
    model = Appender(MCQUEEN_DIR / "mcqueen_carro.obj")
    replace_group_material(model, {"Parabrisas"}, "mcqueen_blue")
    for axle, z in (("Front", 2.35), ("Rear", -2.35)):
        for side, x in (("L", -2.16), ("R", 2.16)):
            model.tube_x(f"McQueen_Car_Rim_{axle}_{side}", "mcqueen_white", (x, -0.42, z), 0.52, 0.14, 32)
            model.tube_x(f"McQueen_Car_Hub_{axle}_{side}", "mcqueen_red", (x * 1.02, -0.42, z), 0.22, 0.16, 24)
    for side, x in (("L", -0.90), ("R", 0.90)):
        model.sphere(f"McQueen_Car_Headlight_{side}", "mcqueen_white", (x, 0.20, 3.88), (0.31, 0.19, 0.10), 22, 11)
        model.sphere(f"McQueen_Car_Taillight_{side}", "mcqueen_red", (x, 0.12, -3.84), (0.28, 0.17, 0.10), 22, 11)
        model.tube_z(f"McQueen_Car_Exhaust_{side}", "mcqueen_dark", (x, -0.18, -3.96), 0.22, 0.34, 24)
        model.tube_z(f"McQueen_Car_Core_{side}", "mcqueen_emission", (x, -0.18, -4.16), 0.14, 0.06, 22)
    model.box("McQueen_Car_Grille", "mcqueen_dark", (0, -0.02, 3.92), (1.36, 0.28, 0.08))
    model.box("McQueen_Car_CenterStripe", "mcqueen_yellow", (0, 1.74, 0.12), (0.22, 0.08, 3.25))
    return model


def submarine() -> Appender:
    model = Appender(MCQUEEN_DIR / "mcqueen_submarino.obj")
    replace_group_material(model, {"Cupula"}, "mcqueen_blue")
    for side, x in (("L", -2.05), ("R", 2.05)):
        model.tube_z(f"McQueen_Sub_Propeller_{side}", "mcqueen_dark", (x, -0.08, -3.82), 0.47, 0.62, 30)
        model.torus_z(f"McQueen_Sub_Guard_{side}", "mcqueen_darkred", (x, -0.08, -4.08), 0.66, 0.085, 40)
        model.tube_z(f"McQueen_Sub_Core_{side}", "mcqueen_emission", (x, -0.08, -4.20), 0.29, 0.08, 26)
        model.sphere(f"McQueen_Sub_Porthole_{side}", "mcqueen_glass", (x * 1.10, 0.64, 0.34), (0.18, 0.37, 0.37), 24, 12)
    for side, x in (("L", -0.72), ("R", 0.72)):
        model.sphere(f"McQueen_Sub_Headlight_{side}", "mcqueen_white", (x, 0.12, 3.72), (0.27, 0.21, 0.11), 22, 11)
    model.box("McQueen_Sub_DorsalRail", "mcqueen_yellow", (0, 2.38, 0.05), (0.23, 0.15, 3.35))
    model.tube_z("McQueen_Sub_Sonar", "mcqueen_white", (0, 2.68, -0.34), 0.19, 0.48, 22)
    return model


def generate(filename: str, builder) -> None:
    target = MCQUEEN_DIR / filename
    builder().write(target)
    target.with_suffix(".mtl").write_text(MTL, encoding="utf-8")
    print(f"Generated {target.relative_to(ROOT)}")


if __name__ == "__main__":
    generate("mcqueen_nave_detallada.obj", spaceship)
    generate("mcqueen_carro_detallado.obj", car)
    generate("mcqueen_submarino_detallado.obj", submarine)
