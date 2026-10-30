from __future__ import annotations

import json
import socket
import threading
import uuid
from dataclasses import dataclass
from typing import Any


class BlenderBridgeError(RuntimeError):
    """Raised when Blender cannot execute a bridge command."""


@dataclass(slots=True)
class BlenderBridge:
    host: str = "127.0.0.1"
    port: int = 9876
    token: str = ""
    timeout: float = 30.0

    def call(self, command: str, **params: Any) -> Any:
        request_id = str(uuid.uuid4())
        request = {
            "id": request_id,
            "token": self.token,
            "command": command,
            "params": params,
        }
        payload = (json.dumps(request, separators=(",", ":")) + "\n").encode()

        try:
            with socket.create_connection((self.host, self.port), self.timeout) as sock:
                sock.settimeout(self.timeout)
                sock.sendall(payload)
                response = self._read_line(sock)
        except OSError as exc:
            raise BlenderBridgeError(
                f"No se pudo conectar con Blender en {self.host}:{self.port}. "
                "Instala y activa el complemento blender_mcp/addon.py."
            ) from exc

        try:
            message = json.loads(response)
        except json.JSONDecodeError as exc:
            raise BlenderBridgeError("Blender devolvio una respuesta JSON invalida") from exc
        if message.get("id") != request_id:
            raise BlenderBridgeError("La respuesta de Blender no coincide con la solicitud")
        if not message.get("ok"):
            raise BlenderBridgeError(str(message.get("error", "Error desconocido de Blender")))
        return message.get("result")

    @staticmethod
    def _read_line(sock: socket.socket) -> str:
        chunks = bytearray()
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            chunks.extend(chunk)
            if b"\n" in chunk:
                break
            if len(chunks) > 8 * 1024 * 1024:
                raise BlenderBridgeError("Respuesta de Blender demasiado grande")
        if not chunks:
            raise BlenderBridgeError("Blender cerro la conexion sin responder")
        return bytes(chunks).split(b"\n", 1)[0].decode("utf-8")


class LockedBridge:
    """Serializes calls because Blender processes scene commands on its UI thread."""

    def __init__(self, bridge: BlenderBridge):
        self._bridge = bridge
        self._lock = threading.Lock()

    def call(self, command: str, **params: Any) -> Any:
        with self._lock:
            return self._bridge.call(command, **params)

