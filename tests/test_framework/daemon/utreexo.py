"""
test_framework.daemon.utreexo.py

A test framework for testing utreexod daemon in regtest mode.
"""

from typing import List

from test_framework.daemon.base import BaseDaemon

VALID_UTREEXOD_EXTRA_ARGS = [
    "--datadir",
    "--logdir",
    "-C," "--configfile",
    "-d," "--debuglevel",
    "--dbtype",
    "--sigcachemaxsize",
    "--utxocachemaxsize",
    "--noutreexo",
    "--prune",
    "--profile",
    "--cpuprofile",
    "--memprofile",
    "--traceprofile",
    "--testnet",
    "--regtest",
    "--notls",
    "--norpc",
    "--rpccert",
    "--rpckey",
    "--rpclimitpass",
    "--rpclimituser",
    "--rpclisten",
    "--rpcmaxclients",
    "--rpcmaxconcurrentreqs",
    "--rpcmaxwebsockets",
    "--rpcquirks",
    "--proxy",
    "--proxypass",
    "--proxyuser",
    "-a," "--addpeer",
    "--connect",
    "--listen",
    "--nolisten",
    "--maxpeers",
    "--uacomment",
    "--trickleinterval",
    "--nodnsseed",
    "--externalip",
    "--upnp",
    "--agentblacklist",
    "--agentwhitelist",
    "--whitelist",
    "--nobanning",
    "--banduration",
    "--banthreshold",
    "--addcheckpoint",
    "--nocheckpoints",
    "--noassumeutreexo",
    "--blocksonly",
    "--maxorphantx",
    "--minrelaytxfee",
    "--norelaypriority",
    "--relaynonstd",
    "--rejectnonstd",
    "--rejectreplacement",
    "--limitfreerelay",
    "--generate",
    "--miningaddr",
    "--blockmaxsize",
    "--blockminsize",
    "--blockmaxweight",
    "--blockminweight",
    "--blockprioritysize",
    "--addrindex",
    "--txindex",
    "--utreexoproofindex",
    "--flatutreexoproofindex",
    "--utreexoproofindexmaxmemory",
    "--cfilters",
    "--nopeerbloomfilters",
    "--dropaddrindex",
    "--dropcfindex",
    "--droptxindex",
    "--droputreexoproofindex",
    "--dropflatutreexoproofindex",
    "--watchonlywallet",
    "--registeraddresstowatchonlywallet",
    "--registerextendedpubkeystowatchonlywallet",
    "--registerextendedpubkeyswithaddresstypetowatchonlywallet",
    "--nobdkwallet",
    "--electrumlisteners",
    "--tlselectrumlisteners",
    "--disableelectrum",
]


class UtreexoDaemon(BaseDaemon):
    """
    Spawn a new utreexod process on background and run it on
    regtest mode for tests. You can use it to generate blocks
    and utreexo proofs for tests.
    """

    def create(self, target: str):
        self.name = "utreexod"
        self.target = target

    def add_daemon_settings(self, settings: List[str]):
        """
        Add node settings to the list of settings.

        settings are the CLI arguments to be passed to the node and
        are based on the VALID_UTREEXOD_EXTRA_ARGS.

        Not all possible arguments are valid for tests
        (for example, "--version",
        """

        if len(settings) >= 1:
            for extra in settings:
                option = extra.split("=") if "=" in extra else extra.split(" ")
                if option[0] in VALID_UTREEXOD_EXTRA_ARGS:
                    self.settings.extend(option)
                else:
                    raise ValueError(f"Invalid extra_arg '{option}'")
