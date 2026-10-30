import json
import unittest
from unittest.mock import patch

from blender_mcp.bridge import BlenderBridge, BlenderBridgeError


class _FakeSocket:
    def __init__(self, fail=False):
        self.fail = fail
        self.request = None

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False

    def settimeout(self, timeout):
        pass

    def sendall(self, payload):
        self.request = json.loads(payload)

    def recv(self, size):
        if self.request is None:
            return b""
        response = {
            "id": self.request["id"],
            "ok": not self.fail,
            "result": self.request["params"],
            "error": "boom" if self.fail else None,
        }
        self.request = None
        return (json.dumps(response) + "\n").encode()


class BridgeTests(unittest.TestCase):
    def test_round_trip(self):
        with patch("blender_mcp.bridge.socket.create_connection", return_value=_FakeSocket()):
            self.assertEqual(BlenderBridge().call("echo", value=42), {"value": 42})

    def test_remote_error(self):
        with patch("blender_mcp.bridge.socket.create_connection", return_value=_FakeSocket(fail=True)):
            with self.assertRaisesRegex(BlenderBridgeError, "boom"):
                BlenderBridge().call("fail")


if __name__ == "__main__":
    unittest.main()

