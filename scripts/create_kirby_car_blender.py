"""Build Kirby's detailed ground-racing vehicle with Blender 4.x."""

from pathlib import Path
import math

import bpy


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "vehicles" / "kirby"


def material(name, color, metallic=0.0, roughness=0.4, emission=None):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*emission, 1.0)
        shader.inputs["Emission Strength"].default_value = 6.5
    return mat


def finish(obj, name, mat, smooth=True):
    obj.name = name
    obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = smooth
    return obj


def uv(name, location, scale, mat, segments=40, rings=20):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1.0, location=location
    )
    obj = finish(bpy.context.object, name, mat)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def cube(name, location, scale, mat, bevel=0.0, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, False)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("Soft bevel", "BEVEL")
        mod.width = bevel
        mod.segments = 3
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def cylinder(name, location, radius, depth, mat, vertices=48, rotation=(0.0, math.pi / 2.0, 0.0)):
    # Native cylinder Z axis rotated onto the lateral X wheel axle.
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=location,
        rotation=rotation,
    )
    return finish(bpy.context.object, name, mat)


def star(name, location, outer, inner, depth, mat):
    points = []
    for y in (-depth * 0.5, depth * 0.5):
        for index in range(10):
            angle = math.pi * 0.5 + index * math.pi / 5.0
            radius = outer if index % 2 == 0 else inner
            points.append((math.cos(angle) * radius, y, math.sin(angle) * radius))
    faces = []
    faces.append(tuple(range(10)))
    faces.append(tuple(range(19, 9, -1)))
    for index in range(10):
        nxt = (index + 1) % 10
        faces.append((index, nxt, 10 + nxt, 10 + index))
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(points, [], faces)
    mesh.update()
    obj = finish(bpy.data.objects.new(name, mesh), name, mat, False)
    obj.location = location
    bpy.context.collection.objects.link(obj)
    bevel = obj.modifiers.new("Star bevel", "BEVEL")
    bevel.width = 0.06
    bevel.segments = 2
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=bevel.name)
    obj.select_set(False)
    return obj


def wheel(side, z, label, tire, rim, glow):
    x = side * (2.05 if z > 0 else 2.15)
    cylinder(f"Kirby_Car_{label}_Tire", (x, 0.68, z), 0.67, 0.52, tire, 48)
    cylinder(f"Kirby_Car_{label}_Rim", (x, 0.68, z), 0.38, 0.56, rim, 48)
    cylinder(f"Kirby_Car_{label}_HubGlow", (x, 0.68, z), 0.17, 0.58, glow, 40)


def build():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    pink = material("KirbyCar_Pink", (0.98, 0.35, 0.62), 0.12, 0.28)
    light_pink = material("KirbyCar_LightPink", (1.0, 0.62, 0.78), 0.06, 0.3)
    red = material("KirbyCar_Red", (0.72, 0.025, 0.08), 0.2, 0.25)
    blue = material("KirbyCar_EyeBlue", (0.02, 0.18, 0.55), 0.32, 0.15)
    black = material("KirbyCar_Black", (0.008, 0.01, 0.018), 0.18, 0.55)
    metal = material("KirbyCar_StarMetal", (0.95, 0.58, 0.025), 0.62, 0.22)
    tire = material("KirbyCar_Tire", (0.012, 0.014, 0.022), 0.1, 0.72)
    cyan = material("KirbyCar_CyanGlow", (0.02, 0.55, 0.98), 0.25, 0.14, (0.02, 0.75, 1.0))
    white = material("KirbyCar_White", (0.93, 0.96, 1.0), 0.15, 0.2)
    glass = material("KirbyCar_Glass", (0.035, 0.12, 0.22), 0.35, 0.10)

    # A conventional compact sports coupe, with recognizable hood, cabin,
    # doors, trunk and bumpers instead of a character-shaped blob.
    cube("Kirby_Car_Chassis", (0.0, 0.62, 0.0), (3.80, 0.68, 5.44), black, 0.20)
    cube("Kirby_Car_LowerBody", (0.0, 0.92, 0.05), (3.76, 1.10, 5.16), pink, 0.34)
    cube("Kirby_Car_Hood", (0.0, 1.34, 1.82), (3.52, 0.44, 1.76), pink, 0.24, (0.05, 0.0, 0.0))
    cube("Kirby_Car_Trunk", (0.0, 1.30, -2.02), (3.44, 0.48, 1.28), pink, 0.22, (-0.04, 0.0, 0.0))
    # A dark inner shell fills any seams between glass, pillars and body panels.
    cube("Kirby_Car_InnerCabin", (0.0, 1.78, -0.34), (2.72, 1.18, 2.18), black, 0.30)
    cube("Kirby_Car_Cabin", (0.0, 1.82, -0.34), (2.96, 1.40, 2.50), pink, 0.40)
    cube("Kirby_Car_Roof", (0.0, 2.42, -0.42), (2.50, 0.28, 1.60), light_pink, 0.16)
    cube("Kirby_Car_Windshield", (0.0, 1.94, 0.73), (2.60, 0.96, 0.11), glass, 0.10, (-0.38, 0.0, 0.0))
    cube("Kirby_Car_RearWindow", (0.0, 1.91, -1.40), (2.56, 0.90, 0.11), glass, 0.10, (0.42, 0.0, 0.0))
    cube("Kirby_Car_FrontBumper", (0.0, 0.70, 2.78), (3.68, 0.40, 0.36), metal, 0.12)
    cube("Kirby_Car_RearBumper", (0.0, 0.68, -2.78), (3.50, 0.36, 0.32), metal, 0.10)
    cube("Kirby_Car_Undertray", (0.0, 0.34, -0.02), (3.46, 0.18, 4.92), black, 0.08)

    # High-resolution fenders give the normal car silhouette smooth wheel arches.
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        uv(f"Kirby_Car_FrontFender_{label}", (side * 1.68, 0.88, 1.65), (0.42, 0.78, 0.92), pink, 48, 24)
        uv(f"Kirby_Car_RearFender_{label}", (side * 1.72, 0.88, -1.62), (0.42, 0.78, 0.92), pink, 48, 24)
        uv(f"Kirby_Car_FrontWheelWell_{label}", (side * 1.82, 0.72, 1.65), (0.30, 0.69, 0.79), black, 40, 20)
        uv(f"Kirby_Car_RearWheelWell_{label}", (side * 1.84, 0.72, -1.62), (0.30, 0.69, 0.79), black, 40, 20)
        cube(f"Kirby_Car_Door_{label}", (side * 1.925, 1.18, -0.18), (0.09, 0.94, 1.64), pink, 0.045)
        cube(f"Kirby_Car_DoorInset_{label}", (side * 1.972, 1.20, -0.18), (0.036, 0.70, 1.36), light_pink, 0.025)
        cube(f"Kirby_Car_SideWindow_{label}", (side * 1.49, 1.93, -0.35), (0.09, 0.80, 1.44), glass, 0.08)
        cube(f"Kirby_Car_SideRail_{label}", (side * 1.93, 0.60, -0.10), (0.14, 0.20, 2.90), metal, 0.05)
        cube(f"Kirby_Car_Mirror_{label}", (side * 1.68, 1.73, 0.48), (0.44, 0.22, 0.32), pink, 0.08)

    # Kirby's spacecraft eyes are retained as the car's front identity.
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        uv(f"Kirby_Car_Eye_{label}", (side * 0.40, 1.39, 2.70), (0.16, 0.31, 0.052), black, 36, 18)
        uv(f"Kirby_Car_Iris_{label}", (side * 0.40, 1.31, 2.748), (0.105, 0.20, 0.030), blue, 32, 16)
        uv(f"Kirby_Car_EyeHighlight_{label}", (side * 0.37, 1.53, 2.776), (0.050, 0.090, 0.018), white, 24, 12)
        uv(f"Kirby_Car_Cheek_{label}", (side * 0.92, 1.20, 2.69), (0.22, 0.11, 0.045), red, 28, 14)
        uv(f"Kirby_Car_Headlamp_{label}", (side * 1.45, 1.18, 2.66), (0.25, 0.16, 0.07), white, 32, 16)

    # Modeled paint reflections keep the smooth body readable in motion.
    uv("Kirby_Car_Highlight_Roof", (-0.44, 2.57, -0.18), (0.55, 0.035, 0.11), white, 32, 16)
    uv("Kirby_Car_Highlight_Front", (-1.10, 1.56, 1.95), (0.34, 0.035, 0.10), white, 28, 14)
    uv("Kirby_Car_Highlight_Side", (1.975, 1.35, -0.22), (0.022, 0.11, 0.38), white, 28, 14)

    # Four wheels and luminous hubs.
    wheel(-1.0, 1.65, "FrontWheel_L", tire, metal, cyan)
    wheel(1.0, 1.65, "FrontWheel_R", tire, metal, cyan)
    wheel(-1.0, -1.62, "RearWheel_L", tire, metal, cyan)
    wheel(1.0, -1.62, "RearWheel_R", tire, metal, cyan)

    # Subtle star badge pressed into the side of the dome, like the reference.
    badge = star("Kirby_Car_HoodStar", (2.105, 2.02, -0.45), 0.42, 0.19, 0.05, light_pink)
    badge.rotation_euler[2] = math.pi / 2.0

    # Twin glowing exhaust cores used by the game's turbo renderer.
    for side in (-1.0, 1.0):
        label = "L" if side < 0 else "R"
        cylinder(
            f"Kirby_Car_Exhaust_{label}", (side * 0.68, 0.86, -2.7),
            0.33, 0.62, black, 20, (0.0, 0.0, 0.0)
        )
        cylinder(
            f"Kirby_Car_Core_{label}", (side * 0.68, 0.86, -3.04),
            0.22, 0.10, cyan, 20, (0.0, 0.0, 0.0)
        )

    exhaust_star = star(
        "Kirby_Car_ExhaustStar", (0.0, 1.02, -3.34), 0.42, 0.19, 0.07, cyan
    )
    exhaust_star.rotation_euler[0] = math.pi / 2.0
    uv("Kirby_Car_ExhaustSpark_L", (-0.48, 1.12, -3.32), (0.12, 0.12, 0.07), white, 20, 10)
    uv("Kirby_Car_ExhaustSpark_R", (0.48, 0.93, -3.31), (0.10, 0.10, 0.06), white, 20, 10)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    world = bpy.data.worlds.new("Kirby Car World")
    world.color = (0.018, 0.008, 0.028)
    bpy.context.scene.world = world
    OUTPUT.mkdir(parents=True, exist_ok=True)
    blend_path = OUTPUT / "kirby_carro_detallado.blend"
    obj_path = OUTPUT / "kirby_carro_detallado.obj"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.wm.obj_export(
        filepath=str(obj_path), export_materials=True, export_normals=True, export_uv=False,
        forward_axis="Y", up_axis="Z",
    )
    print(f"Created {blend_path}")
    print(f"Created {obj_path}")


if __name__ == "__main__":
    build()
