"""Create the detailed animated sun asset used by the game.

Run with:
    blender --background --python scripts/create_sun_blender.py
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "assets" / "planets"


def material(name: str, color: tuple[float, float, float, float], strength: float):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = color
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = color
    shader.inputs["Roughness"].default_value = 0.42
    shader.inputs["Emission Color"].default_value = color
    shader.inputs["Emission Strength"].default_value = strength
    return mat


def smooth(obj, mat):
    obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def create_surface(materials):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=5, radius=55.0)
    surface = bpy.context.object
    surface.name = "Sun_Plasma_Surface"
    for mat in materials:
        surface.data.materials.append(mat)
    for vertex in surface.data.vertices:
        direction = vertex.co.normalized()
        wave = (
            math.sin(direction.x * 19.0 + direction.z * 7.0)
            + math.sin(direction.y * 23.0 - direction.x * 11.0) * 0.55
            + math.sin((direction.x + direction.y + direction.z) * 37.0) * 0.22
        )
        vertex.co = direction * (55.0 + wave * 0.72)
    for index, polygon in enumerate(surface.data.polygons):
        band = (index * 37 + index // 11) % 13
        polygon.material_index = 0 if band < 7 else (1 if band < 11 else 2)
        polygon.use_smooth = True
    return surface


def create_prominence(name, direction, major_radius, rotation, mat):
    center = Vector(direction).normalized() * 51.5
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major_radius,
        minor_radius=1.15,
        major_segments=28,
        minor_segments=7,
        location=center,
        rotation=rotation,
    )
    obj = smooth(bpy.context.object, mat)
    obj.name = name
    return obj


def create_flare(name, direction, length, radius, mat):
    direction = Vector(direction).normalized()
    center = direction * (55.0 + length * 0.42)
    bpy.ops.mesh.primitive_cone_add(
        vertices=12,
        radius1=radius,
        radius2=0.12,
        depth=length,
        location=center,
    )
    obj = smooth(bpy.context.object, mat)
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(direction)
    return obj


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    hot = material("sun_hot_plasma", (1.0, 0.56, 0.035, 1.0), 7.0)
    bright = material("sun_bright_cells", (1.0, 0.88, 0.20, 1.0), 10.0)
    dark = material("sun_dark_cells", (0.82, 0.16, 0.015, 1.0), 4.0)
    flare = material("sun_corona_flare", (1.0, 0.30, 0.025, 1.0), 12.0)

    create_surface((hot, bright, dark))
    prominence_specs = [
        ((1, 0.18, 0.12), 8.5, (0.0, math.pi / 2, 0.2)),
        ((-1, -0.24, 0.20), 7.2, (0.25, math.pi / 2, -0.35)),
        ((0.20, 1, -0.12), 6.8, (math.pi / 2, 0.1, 0.0)),
        ((-0.18, -1, -0.22), 9.0, (math.pi / 2, -0.2, 0.35)),
        ((0.16, 0.10, 1), 7.6, (0.2, 0.1, 0.0)),
        ((-0.22, 0.14, -1), 6.5, (-0.15, 0.25, 0.0)),
    ]
    for index, (direction, radius, rotation) in enumerate(prominence_specs):
        create_prominence(f"Sun_Prominence_{index:02}", direction, radius, rotation, flare)

    flare_directions = [
        (1, 0.3, 0.1), (-1, 0.2, -0.2), (0.2, 1, 0.15),
        (-0.1, -1, 0.25), (0.15, 0.2, 1), (-0.25, -0.1, -1),
        (0.7, -0.6, 0.25), (-0.65, 0.55, -0.3),
    ]
    for index, direction in enumerate(flare_directions):
        create_flare(f"Sun_Corona_Ray_{index:02}", direction, 7.0 + index % 3 * 1.8, 1.6, flare)

    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT_DIR / "sol_detallado.blend"))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(OUTPUT_DIR / "sol_detallado.obj"),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    print("Created", OUTPUT_DIR / "sol_detallado.obj")


if __name__ == "__main__":
    main()
