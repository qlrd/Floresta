"""
floresta_cli_getblock.py

This functional test cli utility to interact with a Floresta node with `getblock`
"""

from test_framework.test_framework import FlorestaTestFramework
from test_framework.floresta_rpc import REGTEST_RPC_SERVER


class GetBlockTest(FlorestaTestFramework):
    """
    Test `getblock` with a fresh node and the first block
    """

    nodes = [-1]
    bits = "ffff7f20"
    chainwork = "2"
    confirmations = 1
    difficulty = 1
    hash = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
    height = 0
    mediantime = 1296688602
    merkleroot = "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"
    n_tx = 1
    nonce = 2
    prev_blockhash = "0000000000000000000000000000000000000000000000000000000000000000"
    size = 285
    strippedsize = 285
    time = 1296688602
    tx = ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"]
    version = 1
    version_hex = "01000000"
    weight = 1140

    def set_test_params(self):
        """
        Setup a single node
        """
        GetBlockTest.nodes[0] = self.add_node_settings(
            chain="regtest", extra_args=[], rpcserver=REGTEST_RPC_SERVER
        )

    def run_test(self):
        """
        Run JSONRPC server and get some data about first block
        """
        self.run_rpc()
        self.run_node(GetBlockTest.nodes[0])
        self.wait_for_rpc_connection(GetBlockTest.nodes[0])

        # Test assertions
        node = self.get_node(GetBlockTest.nodes[0])
        response = node.get_block(
            "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
        )
        assert response["bits"] == GetBlockTest.bits
        assert response["chainwork"] == GetBlockTest.chainwork
        assert response["confirmations"] == GetBlockTest.confirmations
        assert response["difficulty"] == GetBlockTest.difficulty
        assert response["hash"] == GetBlockTest.hash
        assert response["height"] == GetBlockTest.height
        assert response["mediantime"] == GetBlockTest.mediantime
        assert response["merkleroot"] == GetBlockTest.merkleroot
        assert response["n_tx"] == GetBlockTest.n_tx
        assert response["nonce"] == GetBlockTest.nonce
        assert response["previousblockhash"] == GetBlockTest.prev_blockhash
        assert response["size"] == GetBlockTest.size
        assert response["strippedsize"] == GetBlockTest.strippedsize
        assert response["time"] == GetBlockTest.time
        assert len(response["tx"]) == len(GetBlockTest.tx)
        assert response["version"] == GetBlockTest.version
        assert response["versionHex"] == GetBlockTest.version_hex
        assert response["weight"] == GetBlockTest.weight

        # Shutdown node
        self.stop_node(GetBlockTest.nodes[0])


if __name__ == "__main__":
    GetBlockTest().main()
