import json
import socket
import time
from subprocess import Popen
from typing import Any, Dict, List, Tuple

from requests import post
from requests.exceptions import ConnectionError, HTTPError, Timeout
from requests.models import HTTPBasicAuth
from test_framework.rpc.exceptions import JSONRPCError


class BaseRpcMetaClass(type):
    """
    Metaclass for BaseDaemon.

    Ensures that any attempt to register a subclass of `BaseDaemon` will
    adheres to a standard whereby the subclass override `create` and `add_node_settings`
    but DOES NOT override either `__init__`, `kill`.

    If any of those standards are violated, a ``TypeError`` is raised.
    """

    def __new__(mcs, clsname, bases, dct):
        if not clsname == "BaseRPC":

            if (
                "__init__" in dct
                and "log" in dct
                and "kill" in dct
                and "wait_to_stop" in dct
                and "perform_request" in dct
                and "wait_for_rpc_connection" in dct
                and "get_blockchain_info" in dct
                and "stop" in dct
            ):
                raise TypeError(
                    "BaseRPC subclasses must not override '__init__', 'log', 'kill', 'wait_to_stop', 'perform_request', 'wait_for_rpc_connection', 'get_blockchain_info' and 'stop'"
                )

        return super().__new__(mcs, clsname, bases, dct)


class RPCServer:

    def __init__(self, **kwargs):
        self.host = kwargs.get("host", "127.0.0.1")
        self.ports = kwargs.get("ports", {})
        self.user = kwargs.get("user", None)
        self.password = kwargs.get("password", None)
        self.jsonrpc_version = kwargs.get("jsonrpc", "1.0")
        self.timeout = kwargs.get("timeout", 10000)


class BaseRPC(metaclass=BaseRpcMetaClass):
    """
    A class for making RPC calls to a floresta node.
    """

    # Avoid R0913: Too many arguments by defining
    # a dictionary structure for RPC server
    #
    # Avoid W0102: Dangerous default value as argument
    # See more at https://www.valentinog.com/blog/tirl-python-default-arguments/
    def __init__(self, process: Popen[str], rpcserver: Dict[str, str | Dict[str, str]]):
        """
        Initialize a FlorestaRPC object

        Args:
            process: usually, a `cargo run --features json-rpc  --bin florestad` subprocess
            rpcserver: rpc server to be called, generally a regtest (see REGTEST_RPC_SERVER)
        """

        # Avoid R0902: Too many instance attributes
        self._rpcserver = RPCServer(**rpcserver)
        self._process = process
        self._rpcconn = None

    @property
    def rpcconn(self):
        """Getter for `rpcconn` property"""
        return self._rpcconn

    @rpcconn.setter
    def rpcconn(self, value: dict):
        """Setter for `rpcconn` property"""
        self._rpcconn = value

    @property
    def process(self) -> Popen[str] | None:
        """Getter for `process` property"""
        return self._process

    @process.setter
    def process(self, value: Popen[str]):
        """Setter for `process` property"""
        self._process = value

    @property
    def rpcserver(self) -> RPCServer:
        """Getter for `rpcsserver` property"""
        return self._rpcserver

    @rpcserver.setter
    def rpcserver(self, value: RPCServer):
        """Setter for `rpcserver` property"""
        self._rpcserver = value

    def log(self, message: str):
        """Log a message to the console"""
        print(f"[{self.__class__.__name__}] {message}")

    # pylint: disable=unused-argument
    def perform_request(
        self,
        method: str,
        params: List[int | str | float | Dict[str, str | Dict[str, str]]] = [],
    ) -> Any:
        # Create basic information for the requests
        # inside the `kwargs` dictionary
        # - url
        # - headers
        # - data (payload)
        # - timeout
        host = getattr(self.rpcserver, "host")
        ports = getattr(self.rpcserver, "ports")
        rpc_port = ports["rpc"]
        user = getattr(self.rpcserver, "user")
        password = getattr(self.rpcserver, "password")
        jsonrpc_version = getattr(self.rpcserver, "jsonrpc_version")
        timeout = getattr(self.rpcserver, "timeout")
        kwargs = {
            "url": f"http://{host}:{rpc_port}/",
            "headers": {"content-type": "application/json"},
            "data": json.dumps(
                {
                    "jsonrpc": jsonrpc_version,
                    "id": "0",
                    "method": method,
                    "params": params,
                }
            ),
            "timeout": timeout,
        }

        # Check if the RPC server has a username and password
        # and set the auth accordingly to HTTPBasicAuth.
        logmsg = "GET "
        if user is not None and password is not None:
            logmsg += f"<{user}:{password}>@"
            kwargs["auth"] = HTTPBasicAuth(user, password)

        # Now make the POST request to the RPC server
        logmsg += f"{kwargs['url']}{method}?params={params}"
        response = post(**kwargs)

        # wait a little to avoid overloading the daemon.
        # maybe it not overload, but it is a good practice
        time.sleep(0.3)

        # If response isnt 200, raise an HTTPError
        if response.status_code != 200:
            raise HTTPError

        result = response.json()

        # Error could be None or a str
        # If in the future this change,
        # cast the resulted error to str
        if "error" in result and result["error"] is not None:
            raise JSONRPCError(
                rpc_id=result["id"],
                code=result["error"]["code"],
                data=result["error"]["data"],
                message=result["error"]["message"],
            )

        self.log(logmsg)
        self.log(result["result"])
        return result["result"]

    def _is_connection_open(self, host: str, port: int) -> bool:
        """Returns True if a TCP port is open (connection succeeded)."""
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.5)
            connected = sock.connect_ex((host, port))
            return connected == 0

    def _wait_for_connection(
        self, host: str, port: int, opened: bool, timeout: int = 10
    ):
        """
        Wait for the RPC connection to reach the desired state.
        If the connection does not reach the desired state in time,
        raise a TimeoutError.
        """
        start = time.time()
        while time.time() - start < timeout:
            if self._is_connection_open(host, port) == opened:
                return
            time.sleep(0.2)

        state = "open" if opened else "closed"
        raise TimeoutError(f"{host}:{port} not {state} after {timeout} seconds")

    def wait_for_connections(self, opened: bool = True):
        """Wait for all port connections in the host reach the desired state."""
        host = getattr(self.rpcserver, "host")
        for name, port in getattr(self.rpcserver, "ports").items():
            self._wait_for_connection(host, port, opened)
            self.log(f"{host}:{port} for {name} {'open' if opened else 'closed'}")

    def get_blockchain_info(self) -> dict:
        """
        Get the blockchain info by performing `perform_request('getblockchaininfo')`
        """
        return self.perform_request("getblockchaininfo")

    def stop(self):
        """
        Perform the `stop` RPC command to utreexod and some cleanup on process and files
        """
        self.perform_request("stop")
        self.wait_for_connections(opened=False)
