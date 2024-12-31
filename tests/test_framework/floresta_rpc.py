""" 
floresta_rpc.py

A test framework for testing JsonRPC calls to a floresta node.

| Method              | Floresta JsonRPC calls  | Comment                            |
| ------------------- | ----------------------- | ---------------------------------- |
| get_blockchain_info | `getblockchaininfo`     | -                                  |
"""

# commented unused modules since maybe they can be useful
# import os
# import shutil
# import tempfile
# import logging
# import traceback
import re
import time
import json
from subprocess import Popen
from requests import post, exceptions

REGTEST_RPC_SERVER = {
    "host": "127.0.0.1",
    "port": 18442,
    "user": "user",
    "password": "password",
}


class JSONRPCError(Exception):
    """A custom exception for JSONRPC calls"""

    def __init__(self, rpc_id: str, code: int, message: str):
        """Initialize with message, the error code and the caller id"""
        super().__init__(message)
        self.message = message
        self.code = code
        self.rpc_id = rpc_id

    def __repr__(self):
        """Format the exception repr"""
        return f"JSONRPC error {self.code} on request {self.rpc_id}: {self.message}"

    def __str__(self):
        """Format the exception toString"""
        return f"JSONRPC error {self.code} on request {self.rpc_id}: {self.message}"


class FlorestaRPC:
    """
    A class for making RPC calls to a floresta node.
    """

    # Avoid R0913: Too many arguments by defining
    # a dictionary structure for RPC server
    #
    # Avoid W0102: Dangerous default value as argument
    # See more at https://www.valentinog.com/blog/tirl-python-default-arguments/
    def __init__(self, process: Popen, rpcserver: dict[str, str]):
        """
        Initialize a FlorestaRPC object

        Args:
            process: usually, a `cargo run --features json-rpc  --bin florestad` subprocess
            rpcserver: rpc server to be called, generally a regtest (see REGTEST_RPC_SERVER)
        """

        # Avoid R0902: Too many instance attributes
        self.rpcserver = rpcserver
        self.process = process

    # Define `rpcconn` in a more pythonic way
    # since linter warns for security calls like `rpcconn = None`
    # Defining them with decorators stop it
    @property
    def rpcconn(self):
        """Getter for `rpcconn` property"""
        return self._rpcconn

    @rpcconn.setter
    def rpcconn(self, value: dict):
        """Setter for `rpcconn` property"""
        self._rpcconn = value

    @rpcconn.deleter
    def rpcconn(self):
        """Deleter for `rpcconn` property"""
        self._rpcconn = None

    def wait_for_rpc_connection(self):
        """
        Wait for the RPC connection to be established. This will define
        the `rpcconn` as a dictionary derived from a response performed
        by `perform_request('get')`

        Raises:
            TimeoutError: if a timeout occurs
                          if there are 10 consecutive failed attempts
        """
        timeout = 10
        while True:
            try:
                self.rpcconn = self.get_blockchain_info()
                break

            except exceptions.Timeout as exc:
                raise TimeoutError("Timeout waiting for RPC connection") from exc

            except exceptions.ConnectionError as exc:
                time.sleep(0.1)
                timeout -= 0.1
                if timeout <= 0:
                    raise TimeoutError("Timeout due to a failing connection") from exc
                continue

    def kill(self):
        """
        Kill the floresta node process.
        """
        self.process.kill()

    def wait_to_stop(self):
        """
        Wait for the floresta node process to stop.
        """
        self.process.wait()

    def perform_request(self, method, params=None) -> dict:
        """
        Perform an JsonRPC request to the floresta node.
        """
        host = self.rpcserver["host"]
        port = self.rpcserver["port"]
        url = f"http://{host}:{port}"
        headers = {"content-type": "application/json"}
        payload = json.dumps(
            {
                "method": method,
                "params": params,
                "jsonrpc": "2.0",
                "id": "0",
            }
        )
        timeout = 10000

        # Provide some timeout to request
        # to avoid W3101: Missing timeout argument for method 'requests.post'
        # can cause your program to hang indefinitely (missing-timeout)
        response = post(url, data=payload, headers=headers, timeout=timeout)
        result = response.json()
        if "error" in result:
            raise JSONRPCError(
                rpc_id=result["id"],
                code=result["error"]["code"],
                message=result["error"]["message"],
            )

        return result["result"]

    def get_blockchain_info(self) -> dict:
        """
        Get the blockchain info by performing `perform_request('getblockchaininfo')`
        """
        return self.perform_request("getblockchaininfo")

    def get_blockhash(self, height: int) -> dict:
        """
        Get the blockhash associated with a given height performing
        `perform_request('getblockhash', params=[<int>])`
        """
        return self.perform_request("getblockhash", params=[height])

    def get_block(self, blockhash: str):
        """
        Get a full block, given its hash performing
        `perform_request('getblock', params=[str])`

        Notice that this rpc will cause a actual network request to our node,
        so it may be slow, and if used too often, may cause more network usage.
        The returns for this rpc are identical to bitcoin core's getblock rpc
        as of version 27.0.
        """
        return self.perform_request("getblock", params=[blockhash])

    def get_peerinfo(self):
        """
        Get the outpoint associated with a given tx and vout performing
        `perform_request('gettxout', params=[str, int])`
        """
        return self.perform_request("getpeerinfo")

    def get_stop(self):
        """
        Gracefully stops the node performing
        `perform_request('stop')`
        """
        return self.perform_request("stop")

    def get_addnode(self, node: str):
        """
        Adds a new node to our list of peers performing
        `perform_request('addnode', params=[str])`

        This will make our node try to connect to this peer.

        Args
            node: A network address with the format ip[:port]

        Returns
            success: Whether we successfully added this node to our list of peers
        """
        # matches, IPv4, IPv6 and optional ports from 0 to 65535
        pattern = re.compile(
            r"^("
            r"(?:(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}"
            r"(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])|"
            r"\[([a-fA-F0-9:]+)\]"
            r")"
            r"(:(6553[0-5]|655[0-2][0-9]|65[0-4][0-9]{2}|6[0-4][0-9]{3}|[1-9]?[0-9]{1,4}))?$"
        )

        if not pattern.match(node):
            raise ValueError("Invalid ip[:port] format")
        return self.perform_request("addnode", params=[node])
