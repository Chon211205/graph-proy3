"""Render a quick orthographic PNG preview of a material-grouped OBJ."""

from __future__ import annotations

import sys
from pathlib import Path

import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d.art3d import Poly3DCollection


COLORS = {
    "hull_white": "#dce3e8", "wing_metal": "#626c73",
    "fin_blue": "#173e9a", "cockpit_blue": "#09253f",
    "dark_metal": "#111722", "engine_cyan": "#20d9ef",
    "warning_yellow": "#eab20a",
}


def main() -> None:
    source = Path(sys.argv[1])
    target = Path(sys.argv[2]) if len(sys.argv) > 2 else source.with_suffix(".png")
    vertices, groups, material = [], [], "hull_white"
    for line in source.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if not fields:
            continue
        if fields[0] == "v":
            vertices.append(tuple(map(float, fields[1:4])))
        elif fields[0] == "usemtl":
            material = fields[1]
        elif fields[0] == "f":
            groups.append((material, [vertices[int(item.split("/")[0]) - 1] for item in fields[1:]]))

    figure = plt.figure(figsize=(10, 7), facecolor="#eef1f4")
    axis = figure.add_subplot(111, projection="3d", computed_zorder=False)
    for name, polygon in groups:
        axis.add_collection3d(Poly3DCollection([polygon], facecolor=COLORS.get(name, "#cccccc"), edgecolor="#17212b", linewidth=0.28))
    values = [coordinate for vertex in vertices for coordinate in vertex]
    radius = max(max(values) - min(values), 1) * 0.45
    centers = [sum(vertex[index] for vertex in vertices) / len(vertices) for index in range(3)]
    axis.set_xlim(centers[0] - radius, centers[0] + radius)
    axis.set_ylim(centers[1] - radius, centers[1] + radius)
    axis.set_zlim(centers[2] - radius, centers[2] + radius)
    axis.set_box_aspect((1, 1, 0.55))
    axis.view_init(elev=28, azim=-125)
    axis.set_axis_off()
    figure.tight_layout(pad=0)
    figure.savefig(target, dpi=180, bbox_inches="tight", pad_inches=0.05)
    print(target)


if __name__ == "__main__":
    main()
