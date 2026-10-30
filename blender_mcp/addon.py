bl_info = {
    "name": "Local MCP Bridge",
    "author": "graph-proy3",
    "version": (1, 0, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > MCP",
    "description": "Exposes safe modeling commands to a local MCP server",
    "category": "Development",
}

import json
import math
import os
import queue
import socketserver
import threading
import traceback

import bpy


_requests = queue.Queue()
_server = None
_thread = None
_timer_active = False


def _material(name, color):
    material = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    material.diffuse_color = tuple(color)
    return material


def _create_primitive(params):
    primitive = params["primitive"]
    operators = {
        "cube": bpy.ops.mesh.primitive_cube_add,
        "sphere": bpy.ops.mesh.primitive_uv_sphere_add,
        "cylinder": bpy.ops.mesh.primitive_cylinder_add,
        "cone": bpy.ops.mesh.primitive_cone_add,
        "torus": bpy.ops.mesh.primitive_torus_add,
    }
    operator = operators.get(primitive)
    if operator is None:
        raise ValueError("Primitiva no permitida: " + str(primitive))
    rotation = [math.radians(float(value)) for value in params.get("rotation", (0, 0, 0))]
    operator(
        location=params.get("location", (0, 0, 0)),
        rotation=rotation,
        scale=params.get("scale", (1, 1, 1)),
    )
    obj = bpy.context.object
    obj.name = params["name"]
    material = _material(params.get("material_name", "Material"), params.get("color", (0.8, 0.8, 0.8, 1)))
    if obj.data and hasattr(obj.data, "materials"):
        obj.data.materials.append(material)
    return {"name": obj.name, "type": obj.type}


def _clear_scene():
    count = len(bpy.data.objects)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    return {"deleted": count}


def _execute(command, params):
    if command == "ping":
        return {"connected": True, "blender_version": bpy.app.version_string}
    if command == "scene_info":
        objects = []
        for obj in bpy.context.scene.objects:
            objects.append({
                "name": obj.name,
                "type": obj.type,
                "location": list(obj.location),
                "rotation_degrees": [math.degrees(value) for value in obj.rotation_euler],
                "scale": list(obj.scale),
                "materials": [slot.material.name for slot in obj.material_slots if slot.material],
            })
        return {"object_count": len(objects), "objects": objects}
    if command == "clear_scene":
        return _clear_scene()
    if command == "create_primitive":
        return _create_primitive(params)
    if command == "create_model":
        if params.get("clear_first"):
            _clear_scene()
        created = [_create_primitive(item) for item in params["components"]]
        return {"created_count": len(created), "objects": created}
    if command == "transform_object":
        obj = bpy.data.objects.get(params["name"])
        if obj is None:
            raise ValueError("No existe el objeto: " + params["name"])
        if params.get("location") is not None:
            obj.location = params["location"]
        if params.get("rotation") is not None:
            obj.rotation_euler = [math.radians(float(value)) for value in params["rotation"]]
        if params.get("scale") is not None:
            obj.scale = params["scale"]
        return {"name": obj.name, "updated": True}
    if command == "export_obj":
        path = os.path.abspath(params["path"])
        os.makedirs(os.path.dirname(path), exist_ok=True)
        if hasattr(bpy.ops.wm, "obj_export"):
            bpy.ops.wm.obj_export(filepath=path, export_selected_objects=bool(params.get("selected_only")))
        else:
            bpy.ops.export_scene.obj(filepath=path, use_selection=bool(params.get("selected_only")))
        return {"exported": True, "path": path}
    if command == "save_blend":
        path = os.path.abspath(params["path"])
        os.makedirs(os.path.dirname(path), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=path)
        return {"saved": True, "path": path}
    raise ValueError("Comando no permitido: " + str(command))


class _RequestHandler(socketserver.StreamRequestHandler):
    def handle(self):
        raw = self.rfile.readline(8 * 1024 * 1024)
        request_id = None
        try:
            request = json.loads(raw.decode("utf-8"))
            request_id = request.get("id")
            expected = os.environ.get("BLENDER_MCP_TOKEN", "")
            if expected and request.get("token") != expected:
                raise PermissionError("Token MCP incorrecto")
            event = threading.Event()
            item = {"request": request, "event": event, "response": None}
            _requests.put(item)
            if not event.wait(60):
                raise TimeoutError("Blender no proceso la solicitud en 60 segundos")
            response = item["response"]
        except Exception as exc:
            response = {"id": request_id, "ok": False, "error": str(exc)}
        self.wfile.write((json.dumps(response) + "\n").encode("utf-8"))


class _ThreadedServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    daemon_threads = True
    allow_reuse_address = True


def _process_requests():
    global _timer_active
    if not _timer_active:
        return None
    for _ in range(20):
        try:
            item = _requests.get_nowait()
        except queue.Empty:
            break
        request = item["request"]
        try:
            result = _execute(request.get("command"), request.get("params") or {})
            item["response"] = {"id": request.get("id"), "ok": True, "result": result}
        except Exception as exc:
            traceback.print_exc()
            item["response"] = {"id": request.get("id"), "ok": False, "error": str(exc)}
        finally:
            item["event"].set()
    return 0.05


def start_server():
    global _server, _thread, _timer_active
    if _server is not None:
        return
    port = int(os.environ.get("BLENDER_MCP_PORT", "9876"))
    _server = _ThreadedServer(("127.0.0.1", port), _RequestHandler)
    _thread = threading.Thread(target=_server.serve_forever, name="BlenderMCP", daemon=True)
    _thread.start()
    _timer_active = True
    if not bpy.app.timers.is_registered(_process_requests):
        bpy.app.timers.register(_process_requests, first_interval=0.1, persistent=True)


def stop_server():
    global _server, _thread, _timer_active
    _timer_active = False
    if _server is not None:
        _server.shutdown()
        _server.server_close()
    _server = None
    _thread = None


class MCP_OT_Start(bpy.types.Operator):
    bl_idname = "mcp.start_bridge"
    bl_label = "Start MCP Bridge"

    def execute(self, context):
        try:
            start_server()
            self.report({"INFO"}, "MCP Bridge activo en 127.0.0.1")
            return {"FINISHED"}
        except OSError as exc:
            self.report({"ERROR"}, str(exc))
            return {"CANCELLED"}


class MCP_OT_Stop(bpy.types.Operator):
    bl_idname = "mcp.stop_bridge"
    bl_label = "Stop MCP Bridge"

    def execute(self, context):
        stop_server()
        return {"FINISHED"}


class MCP_PT_Panel(bpy.types.Panel):
    bl_label = "MCP Modeler"
    bl_idname = "MCP_PT_modeler"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "MCP"

    def draw(self, context):
        layout = self.layout
        layout.label(text="Activo" if _server else "Detenido")
        layout.operator("mcp.stop_bridge" if _server else "mcp.start_bridge")


_CLASSES = (MCP_OT_Start, MCP_OT_Stop, MCP_PT_Panel)


def register():
    for cls in _CLASSES:
        bpy.utils.register_class(cls)
    start_server()


def unregister():
    stop_server()
    for cls in reversed(_CLASSES):
        bpy.utils.unregister_class(cls)


if __name__ == "__main__":
    register()

