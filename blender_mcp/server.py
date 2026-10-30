from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Literal

from mcp.server.mcpserver import MCPServer

try:
    from .bridge import BlenderBridge, LockedBridge
except ImportError:  # Allows `python blender_mcp/server.py`.
    from bridge import BlenderBridge, LockedBridge


PROJECT_ROOT = Path(os.environ.get("BLENDER_MCP_PROJECT", Path.cwd())).resolve()
ASSETS_ROOT = (PROJECT_ROOT / "assets").resolve()
bridge = LockedBridge(
    BlenderBridge(
        host=os.environ.get("BLENDER_MCP_HOST", "127.0.0.1"),
        port=int(os.environ.get("BLENDER_MCP_PORT", "9876")),
        token=os.environ.get("BLENDER_MCP_TOKEN", ""),
    )
)
mcp = MCPServer("Blender Modeler")


def _vec3(value: list[float], field: str) -> list[float]:
    if len(value) != 3:
        raise ValueError(f"{field} debe contener exactamente 3 numeros")
    return [float(item) for item in value]


def _asset_path(relative_path: str, suffix: str) -> Path:
    candidate = (ASSETS_ROOT / relative_path).resolve()
    if candidate.suffix.lower() != suffix:
        raise ValueError(f"El archivo debe terminar en {suffix}")
    try:
        candidate.relative_to(ASSETS_ROOT)
    except ValueError as exc:
        raise ValueError("La exportacion debe permanecer dentro de assets/") from exc
    candidate.parent.mkdir(parents=True, exist_ok=True)
    return candidate


@mcp.tool()
def blender_status() -> dict[str, Any]:
    """Comprueba la conexion y devuelve la version de Blender."""
    return bridge.call("ping")


@mcp.tool()
def inspect_scene() -> dict[str, Any]:
    """Lista objetos, tipos, transformaciones y materiales de la escena actual."""
    return bridge.call("scene_info")


@mcp.tool()
def clear_scene(confirm: bool = False) -> dict[str, Any]:
    """Elimina todos los objetos de la escena. Requiere confirm=true."""
    if not confirm:
        raise ValueError("Vuelve a llamar con confirm=true para borrar la escena")
    return bridge.call("clear_scene")


@mcp.tool()
def create_primitive(
    primitive: Literal["cube", "sphere", "cylinder", "cone", "torus"],
    name: str,
    location: list[float] = [0.0, 0.0, 0.0],
    rotation: list[float] = [0.0, 0.0, 0.0],
    scale: list[float] = [1.0, 1.0, 1.0],
    color: list[float] = [0.8, 0.8, 0.8, 1.0],
    material_name: str = "Material",
) -> dict[str, Any]:
    """Crea una primitiva. Rotacion usa grados y color usa RGBA entre 0 y 1."""
    if not name.strip():
        raise ValueError("name no puede estar vacio")
    if len(color) != 4 or any(not 0 <= float(c) <= 1 for c in color):
        raise ValueError("color debe ser RGBA con cuatro valores entre 0 y 1")
    return bridge.call(
        "create_primitive",
        primitive=primitive,
        name=name.strip(),
        location=_vec3(location, "location"),
        rotation=_vec3(rotation, "rotation"),
        scale=_vec3(scale, "scale"),
        color=[float(c) for c in color],
        material_name=material_name,
    )


@mcp.tool()
def transform_object(
    name: str,
    location: list[float] | None = None,
    rotation: list[float] | None = None,
    scale: list[float] | None = None,
) -> dict[str, Any]:
    """Cambia la ubicacion, rotacion en grados o escala de un objeto existente."""
    if location is None and rotation is None and scale is None:
        raise ValueError("Indica al menos una transformacion")
    return bridge.call(
        "transform_object",
        name=name,
        location=_vec3(location, "location") if location is not None else None,
        rotation=_vec3(rotation, "rotation") if rotation is not None else None,
        scale=_vec3(scale, "scale") if scale is not None else None,
    )


@mcp.tool()
def create_model(spec_json: str, clear_first: bool = False) -> dict[str, Any]:
    """Crea un modelo compuesto. spec_json es una lista JSON de primitivas con primitive, name y opcionalmente location, rotation, scale, color y material_name."""
    try:
        components = json.loads(spec_json)
    except json.JSONDecodeError as exc:
        raise ValueError(f"spec_json invalido: {exc}") from exc
    if not isinstance(components, list) or not 1 <= len(components) <= 500:
        raise ValueError("La especificacion debe contener entre 1 y 500 componentes")
    allowed = {"cube", "sphere", "cylinder", "cone", "torus"}
    normalized: list[dict[str, Any]] = []
    for index, component in enumerate(components):
        if not isinstance(component, dict) or component.get("primitive") not in allowed:
            raise ValueError(f"Componente {index}: primitive no es valida")
        normalized.append(component)
    return bridge.call("create_model", components=normalized, clear_first=clear_first)


@mcp.tool()
def export_obj(relative_path: str, selected_only: bool = False) -> dict[str, Any]:
    """Exporta OBJ/MTL dentro de assets/. Ejemplo: vehicles/mi_nave.obj."""
    target = _asset_path(relative_path, ".obj")
    result = bridge.call("export_obj", path=str(target), selected_only=selected_only)
    result["project_relative_path"] = str(target.relative_to(PROJECT_ROOT))
    return result


@mcp.tool()
def save_blend(relative_path: str) -> dict[str, Any]:
    """Guarda el archivo fuente .blend dentro de assets/."""
    target = _asset_path(relative_path, ".blend")
    result = bridge.call("save_blend", path=str(target))
    result["project_relative_path"] = str(target.relative_to(PROJECT_ROOT))
    return result


if __name__ == "__main__":
    mcp.run()

