"""
floresta_cli_getblock.py

This functional test cli utility to interact with a Floresta node with `getblock`
"""

import time
from test_framework import FlorestaTestFramework
from test_framework.rpc.exceptions import JSONRPCError

DATA_DIR = FlorestaTestFramework.get_integration_test_dir()

# The fields that are expected to be present in the genesis block
# and mined blocks when using `getblock` with verbose level 1.
# These fields are based on the Bitcoin Core implementation.
# Note that some fields are commented out because they are not implemented
# or differ from the Bitcoin Core implementation.
# For example, `difficulty` is not implemented in rust-bitcoin, and `nTx` is
# used in Bitcoin Core but `n_tx` is used in Floresta.
GENESIS_BLOCK_FIELDS = [
    "bits",
    "chainwork",
    "confirmations",
    # "difficulty",  # at the moment, the difficulty implemented in rust-bitcoin differs from the one in bitcoin-core
    "hash",
    "height",
    "mediantime",
    "merkleroot",
    # "nTx",  # core uses nTx, floresta uses n_tx
    "nonce",
    # "previousblockhash",
    "size",
    "strippedsize",
    "time",
    "tx",
    "version",
    "versionHex",
    "weight",
]

# The fields that are expected to be present in the mined blocks
# when using `getblock` with verbose level 1.
# These fields are based on the Bitcoin Core implementation.
# Note that some fields are commented out because they are not implemented
# or because they differ from the Bitcoin Core implementation and need to
# be fixed. The most problematic field is `chainwork`, where we return a
# different value
# than bitcoin-core
MINED_BLOCK_FIELDS = [
    "bits",
    # "chainwork",
    "confirmations",
    # "difficulty",
    "hash",
    "height",
    # "mediantime",
    "merkleroot",
    # "nTx",  # core uses nTx, floresta uses n_tx
    "nonce",
    "previousblockhash",
    "size",
    "strippedsize",
    # "time",
    "tx",
    "version",
    "versionHex",
    "weight",
]


class Genesis:
    """
    Genesis block data for the first block in the blockchain.
    """

    block = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
    bits = "207fffff"
    serialized_data = "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff7f20020000000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000"
    chainwork = "0000000000000000000000000000000000000000000000000000000000000002"
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


class GetBlockTest(FlorestaTestFramework):
    """
    Test `getblock` with a fresh node and the first block with verbose levels 0 and 1
    """

    # Thelast char is different from the one in
    # the genesis block for test purposes.
    invalid_block = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2207"

    # pylint: disable=line-too-long

    def set_test_params(self):
        """
        Setup a single node
        """
        self.data_dirs = GetBlockTest.create_data_dirs(
            DATA_DIR, self.__class__.__name__.lower(), 3
        )
        self.bitcoind = self.add_node(
            variant="bitcoind",
            extra_args=[f"-datadir={self.data_dirs[0]}"],
        )
        self.utreexod = self.add_node(
            variant="utreexod",
            extra_args=[
                f"--datadir={self.data_dirs[1]}",
                "--miningaddr=bcrt1q4gfcga7jfjmm02zpvrh4ttc5k7lmnq2re52z2y",
                "--prune=0",
            ],
        )

        self.florestad = self.add_node(
            variant="florestad",
            extra_args=[f"--data-dir={self.data_dirs[2]}"],
        )

    def test_check_serialized_data(self):
        """
        Check that the serialized data of the genesis block matches the expected value.
        """
        bitcoin = self.bitcoind.rpc.get_block(Genesis.block, 0)
        floresta = self.florestad.rpc.get_block(Genesis.block, 0)
        self.assertEqual(bitcoin, floresta)
        self.assertEqual(bitcoin, Genesis.serialized_data)
        self.assertEqual(floresta, Genesis.serialized_data)

    def test_check_fields(self, bitcoin, floresta, fields=GENESIS_BLOCK_FIELDS):
        """
        Check that the fields in the response match the expected values.
        """
        for field in fields:
            self.log(f"Checking field: {field}")
            self.assertEqual(floresta[field], bitcoin[field])

    def test_wrong_block(self):
        """
        Check that an exception is raised when an invalid block hash is provided.
        """
        # Change last character to make it invalid
        wrong_block = Genesis.block[:-1] + "7"

        # Test with verbose level 0
        with self.assertRaises(JSONRPCError) as exc:
            self.bitcoind.rpc.get_block(wrong_block, 0)
            print(exc.exception)

        with self.assertRaises(JSONRPCError) as exc:
            self.florestad.rpc.get_block(wrong_block, 0)
            print(exc.exception)

        # Test with verbose level 1
        with self.assertRaises(JSONRPCError):
            self.bitcoind.rpc.get_block(wrong_block, 1)

        with self.assertRaises(JSONRPCError):
            self.florestad.rpc.get_block(wrong_block, 1)

    def test_getblock_after_mining(self):
        """Mine some blocks with utreexod and connect
        floresta and bitcoin-core to it. After that, check
        that `getblock` returns the expected data for the
        mined blocks.
        """
        self.run_node(self.utreexod)
        self.utreexod.rpc.generate(10)

        utreexod_host = self.utreexod.get_host()
        utreexod_port = self.utreexod.get_port("p2p")

        self.florestad.rpc.addnode(
            f"{utreexod_host}:{utreexod_port}", command="onetry", v2transport=False
        )
        self.bitcoind.rpc.addnode(
            f"{utreexod_host}:{utreexod_port}", command="onetry", v2transport=False
        )

        time.sleep(10)

        # Get the blocks mined by utreexod
        block_count_bitcoind = self.bitcoind.rpc.get_block_count()
        blockchaininfo = self.florestad.rpc.get_blockchain_info()
        self.assertEqual(blockchaininfo["validated"], block_count_bitcoind)

        # get best block hash and check if the raw block data is the same
        bitcoind_block = self.bitcoind.rpc.get_block(blockchaininfo["best_block"], 0)
        floresta_block = self.florestad.rpc.get_block(blockchaininfo["best_block"], 0)
        self.assertEqual(floresta_block, bitcoind_block)

        # Now test verbose level 1
        bitcoind_block = self.bitcoind.rpc.get_block(blockchaininfo["best_block"], 1)
        floresta_block = self.florestad.rpc.get_block(blockchaininfo["best_block"], 1)

        self.test_check_fields(bitcoind_block, floresta_block, MINED_BLOCK_FIELDS)

    def run_test(self):
        """
        Run JSONRPC server and get some data about first block
        """
        self.run_node(self.bitcoind)
        self.run_node(self.florestad)

        # Test verbose level 0
        self.test_check_serialized_data()

        # Test verbose level 1
        response_bitcoind = self.bitcoind.rpc.get_block(Genesis.block, 1)
        response_florestad = self.florestad.rpc.get_block(Genesis.block, 1)
        self.test_check_fields(
            response_bitcoind, response_florestad, GENESIS_BLOCK_FIELDS
        )

        # Mine some blocks and check `getblock` for them
        self.test_getblock_after_mining()

        # Test wrong block hash
        self.test_wrong_block()

        # Shutdown node
        self.stop()


if __name__ == "__main__":
    GetBlockTest().main()
