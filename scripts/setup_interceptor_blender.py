"""Import the interceptor and build its Blender PBR materials.

Run with Blender's Scripting workspace or:
blender --background --python scripts/setup_interceptor_blender.py
"""

from __future__ import annotations

from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]
OBJ_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2.obj"
BLEND_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2.blend"


LOOKS = {
    "hull_white": {
        "base": (0.82, 0.88, 0.93, 1), "metallic": 0.58, "roughness": 0.19,
        "coat": 0.38, "coat_roughness": 0.12,
    },
    "wing_metal": {
        "base": (0.24, 0.29, 0.34, 1), "metallic": 0.88, "roughness": 0.25,
    },
    "fin_blue": {
        "base": (0.025, 0.09, 0.42, 1), "metallic": 0.52, "roughness": 0.18,
        "coat": 0.45, "coat_roughness": 0.10,
    },
    "cockpit_blue": {
        "base": (0.012, 0.07, 0.16, 1), "metallic": 0.22, "roughness": 0.08,
        "transmission": 0.48, "ior": 1.45, "alpha": 0.78, "coat": 0.55,
    },
    "dark_metal": {
        "base": (0.018, 0.025, 0.04, 1), "metallic": 0.92, "roughness": 0.20,
    },
    "engine_cyan": {
        "base": (0.01, 0.34, 0.52, 1), "metallic": 0.20, "roughness": 0.12,
        "emission": (0.01, 0.65, 1.0, 1), "emission_strength": 8.0,
    },
    "warning_yellow": {
        "base": (1.0, 0.55, 0.015, 1), "metallic": 0.42, "roughness": 0.22,
        "coat": 0.25,
    },
}


def set_input(node, names, value) -> None:
    for name in names:
        socket = node.inputs.get(name)
        if socket is not None:
            socket.default_value = value
            return


def configure_material(name: str, look: dict) -> bpy.types.Material:
    material = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    nodes.clear()
    output = nodes.new("ShaderNodeOutputMaterial")
    shader = nodes.new("ShaderNodeBsdfPrincipled")
    shader.location = (-260, 0)
    material.node_tree.links.new(shader.outputs["BSDF"], output.inputs["Surface"])

    set_input(shader, ("Base Color",), look["base"])
    set_input(shader, ("Metallic",), look.get("metallic", 0.0))
    set_input(shader, ("Roughness",), look.get("roughness", 0.35))
    set_input(shader, ("Coat Weight", "Clearcoat"), look.get("coat", 0.0))
    set_input(shader, ("Coat Roughness", "Clearcoat Roughness"), look.get("coat_roughness", 0.15))
    set_input(shader, ("Transmission Weight", "Transmission"), look.get("transmission", 0.0))
    set_input(shader, ("IOR",), look.get("ior", 1.5))
    set_input(shader, ("Alpha",), look.get("alpha", 1.0))
    set_input(shader, ("Emission Color", "Emission"), look.get("emission", (0, 0, 0, 1)))
    set_input(shader, ("Emission Strength",), look.get("emission_strength", 0.0))

    if look.get("alpha", 1.0) < 1.0:
        material.diffuse_color = (*look["base"][:3], look["alpha"])
        if hasattr(material, "surface_render_method"):
            material.surface_render_method = "DITHERED"
        elif hasattr(material, "blend_method"):
            material.blend_method = "BLEND"
        if hasattr(material, "use_transparency_overlap"):
            material.use_transparency_overlap = False
    return material


def import_model() -> list[bpy.types.Object]:
    before = set(bpy.data.objects)
    if hasattr(bpy.ops.wm, "obj_import"):
        bpy.ops.wm.obj_import(filepath=str(OBJ_PATH), forward_axis="NEGATIVE_Z", up_axis="Y")
    else:
        bpy.ops.import_scene.obj(filepath=str(OBJ_PATH), axis_forward="-Z", axis_up="Y")
    return [obj for obj in bpy.data.objects if obj not in before]


def finish_meshes(objects: list[bpy.types.Object]) -> None:
    for obj in objects:
        if obj.type != "MESH":
            continue
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
        bevel = obj.modifiers.new("Micro bevel", "BEVEL")
        bevel.width = 0.018
        bevel.segments = 3
        bevel.limit_method = "ANGLE"


def add_render_environment() -> None:
    world = bpy.context.scene.world or bpy.data.worlds.new("Interceptor World")
    bpy.context.scene.world = world
    world.use_nodes = True
    background = world.node_tree.nodes.get("Background")
    background.inputs["Color"].default_value = (0.008, 0.012, 0.025, 1)
    background.inputs["Strength"].default_value = 0.22


def main() -> None:
    if not OBJ_PATH.exists():
        raise FileNotFoundError(f"Primero genera el OBJ: {OBJ_PATH}")
    materials = {name: configure_material(name, look) for name, look in LOOKS.items()}
    objects = import_model()
    for obj in objects:
        if obj.type != "MESH":
            continue
        for slot in obj.material_slots:
            if not slot.material:
                continue
            imported_name = slot.material.name
            base_name, dot, suffix = imported_name.rpartition(".")
            lookup_name = base_name if dot and suffix.isdigit() else imported_name
            if lookup_name in materials:
                slot.material = materials[lookup_name]
    finish_meshes(objects)
    add_render_environment()
    engines = {item.identifier for item in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items}
    bpy.context.scene.render.engine = (
        "BLENDER_EEVEE_NEXT" if "BLENDER_EEVEE_NEXT" in engines else "BLENDER_EEVEE"
    )
    bpy.ops.wm.save_as_mainfile(filepath=str(BLEND_PATH))
    print(f"Saved {BLEND_PATH}")


if __name__ == "__main__":
    main()
