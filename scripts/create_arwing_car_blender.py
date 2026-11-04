"""Build the detailed ground-racing version of the Arwing.

Run with Blender 4.x:
    blender --background --python scripts/create_arwing_car_blender.py
"""

from pathlib import Path
import math

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "vehicles" / "arwing"


def material(name, color, metallic=0.0, roughness=0.38, emission=None):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*emission, 1.0)
        shader.inputs["Emission Strength"].default_value = 7.0
    return mat


def finish(obj, name, mat, smooth=True):
    obj.name = name
    obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = smooth
    return obj


def cube(name, location, scale, mat, bevel=0.0, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, False)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("Edge bevel", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def uv(name, location, scale, mat, segments=24, rings=12):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1.0, location=location
    )
    obj = finish(bpy.context.object, name, mat)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def cylinder(name, location, radius, depth, mat, rotation=(math.pi / 2, 0.0, 0.0), vertices=24):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, mat)


def wedge(name, points, faces, mat, smooth=False):
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(points, [], faces)
    mesh.update()
    return finish(bpy.data.objects.new(name, mesh), name, mat, smooth)


def add_wedge(name, points, faces, mat):
    obj = wedge(name, points, faces, mat)
    bpy.context.collection.objects.link(obj)
    bevel = obj.modifiers.new("Panel bevel", "BEVEL")
    bevel.width = 0.08
    bevel.segments = 2
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=bevel.name)
    obj.select_set(False)
    return obj


def wheel(x, z, prefix, tire, rim, cyan):
    cylinder(prefix + "_Tire", (x, 0.78, z), 0.78, 0.52, tire)
    cylinder(prefix + "_Rim", (x * 1.002, 0.78, z), 0.43, 0.56, rim)
    cylinder(prefix + "_HubGlow", (x * 1.004, 0.78, z), 0.19, 0.58, cyan)


def build():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    white = material("ArwingCar_HullWhite", (0.78, 0.83, 0.88), 0.72, 0.24)
    silver = material("ArwingCar_WingMetal", (0.24, 0.29, 0.34), 0.88, 0.2)
    blue = material("ArwingCar_FinBlue", (0.025, 0.12, 0.48), 0.65, 0.2)
    glass = material("ArwingCar_CockpitBlue", (0.015, 0.10, 0.22), 0.35, 0.1)
    dark = material("ArwingCar_DarkMetal", (0.025, 0.035, 0.055), 0.8, 0.25)
    tire = material("ArwingCar_Tire", (0.012, 0.014, 0.018), 0.15, 0.72)
    cyan = material("ArwingCar_EngineCyan", (0.02, 0.55, 0.95), 0.3, 0.15, (0.02, 0.72, 1.0))
    red = material("ArwingCar_LaserRed", (0.7, 0.025, 0.02), 0.25, 0.18, (1.0, 0.02, 0.01))
    yellow = material("ArwingCar_WarningYellow", (0.95, 0.58, 0.02), 0.45, 0.25)

    # Low, arrow-shaped central hull. Forward is +Z for ModelAxes::Astra.
    points = [
        (-1.35, 0.55, -2.8), (1.35, 0.55, -2.8),
        (-1.8, 0.55, 1.4), (1.8, 0.55, 1.4),
        (-0.25, 0.72, 4.7), (0.25, 0.72, 4.7),
        (-1.0, 1.65, -1.8), (1.0, 1.65, -1.8),
        (-1.15, 1.45, 1.2), (1.15, 1.45, 1.2),
        (-0.18, 1.05, 4.45), (0.18, 1.05, 4.45),
    ]
    faces = [
        (0, 1, 3, 2), (6, 8, 9, 7), (2, 3, 5, 4), (8, 10, 11, 9),
        (0, 2, 4, 10, 8, 6), (1, 7, 9, 11, 5, 3),
        (0, 6, 7, 1), (4, 5, 11, 10),
    ]
    add_wedge("Arwing_Car_MainHull", points, faces, white)

    # Cockpit, armored spine and recognizable swept Arwing fenders.
    uv("Arwing_Car_Cockpit", (0.0, 1.65, 0.35), (0.92, 0.62, 1.45), glass)
    cube("Arwing_Car_Spine", (0.0, 1.05, -1.85), (0.65, 0.42, 1.25), silver, 0.16)
    for side in (-1.0, 1.0):
        sx = side
        label = "L" if side < 0 else "R"
        wing_points = [
            (sx * 1.0, 0.68, -2.4), (sx * 3.25, 0.62, -1.75),
            (sx * 3.65, 0.70, 1.45), (sx * 1.2, 0.72, 2.0),
            (sx * 1.0, 1.12, -2.25), (sx * 3.05, 1.02, -1.55),
            (sx * 3.35, 1.02, 1.25), (sx * 1.18, 1.18, 1.75),
        ]
        wing_faces = [(0,1,2,3), (4,7,6,5), (0,4,5,1), (1,5,6,2), (2,6,7,3), (3,7,4,0)]
        add_wedge(f"Arwing_Car_Wing_{label}", wing_points, wing_faces, white)
        cube(f"Arwing_Car_BlueFin_{label}", (sx * 2.82, 1.38, -0.35), (0.18, 0.75, 1.55), blue, 0.08, (0.0, 0.0, -side * 0.15))
        cube(f"Arwing_Car_LaserPod_{label}", (sx * 2.55, 1.2, 1.82), (0.28, 0.28, 0.78), dark, 0.13)
        uv(f"Arwing_Car_LaserGlow_{label}", (sx * 2.55, 1.2, 2.58), (0.22, 0.22, 0.28), red, 16, 8)
        cube(f"Arwing_Car_Stripe_{label}", (sx * 1.42, 1.48, 1.2), (0.13, 0.09, 0.8), yellow, 0.04)

    # Four exposed racing wheels with glowing anti-gravity hubs.
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        wheel(side * 2.55, 2.0, f"Arwing_Car_FrontWheel_{label}", tire, silver, cyan)
        wheel(side * 2.65, -1.75, f"Arwing_Car_RearWheel_{label}", tire, silver, cyan)

    # Twin rear engines and a central diffuser.
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        cylinder(f"Arwing_Car_Engine_{label}", (side * 0.78, 1.02, -2.95), 0.52, 1.4, dark, (math.pi / 2, 0.0, 0.0), 20)
        cylinder(f"Arwing_Car_Core_{label}", (side * 0.78, 1.02, -3.68), 0.34, 0.10, cyan, (math.pi / 2, 0.0, 0.0), 20)
    cube("Arwing_Car_Diffuser", (0.0, 0.52, -3.0), (1.05, 0.18, 0.72), dark, 0.08)
    cube("Arwing_Car_RearWing", (0.0, 2.0, -2.55), (2.35, 0.12, 0.45), blue, 0.08)
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        cube(f"Arwing_Car_WingSupport_{label}", (side * 1.55, 1.55, -2.55), (0.1, 0.5, 0.1), silver, 0.03)

    # Apply transforms so the exported OBJ bounds and normals are stable.
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    world = bpy.data.worlds.new("Arwing Car World")
    world.color = (0.008, 0.012, 0.025)
    bpy.context.scene.world = world
    OUTPUT.mkdir(parents=True, exist_ok=True)
    blend_path = OUTPUT / "arwing2_carro_detallado.blend"
    obj_path = OUTPUT / "arwing2_carro_detallado.obj"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.wm.obj_export(
        filepath=str(obj_path),
        export_materials=True,
        export_normals=True,
        export_uv=False,
        # Preserve the authored coordinates: X lateral, Y height, Z forward.
        # Blender's exporter otherwise rotates this Y-up game asset as if it
        # had been authored in Blender's native Z-up convention.
        forward_axis="Y",
        up_axis="Z",
    )
    print(f"Created {blend_path}")
    print(f"Created {obj_path}")


if __name__ == "__main__":
    build()
