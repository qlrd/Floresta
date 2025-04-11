"""
test_framework.daemon.floresta.py

A test framework for testing florestad daemon in regtest mode.
"""

from typing import List

from test_framework.daemon.base import BaseDaemon

VALID_FLORESTAD_EXTRA_ARGS = [
    "-c",
    "--config-file",
    "-d",
    "--debug",
    "--log-to-file",
    "--data-dir",
    "--cfilters",
    "-p",
    "--proxy",
    "--wallet-xpub",
    "--wallet-descriptor",
    "--assume-valid",
    "-z",
    "--zmq-address",
    "--connect",
    "--rpc-address",
    "--electrum-address",
    "--filters-start-height",
    "--assume-utreexo",
    "--pid-file",
    "--gen-selfsigned-cert" "--ssl-electrum-address",
    "--ssl-cert-path",
    "--ssl-key-path",
    "--no-ssl",
]


class FlorestaDaemon(BaseDaemon):
    """
    Spawn a new Florestad process on background and run it on
    regtest mode for tests.
    """

    def create(self, target: str):
        """
        Create a new instance of Florestad.
        Args:
            target: The path to the executable.
        """
        self.name = "florestad"
        self.target = target

    def add_daemon_settings(self, settings: List[str]):
        """
        Add node settings to the list of settings.

        settings are the CLI arguments to be passed to the node and
        are based on the VALID_FLORESTAD_EXTRA_ARGS.

        Not all possible arguments are valid for tests
        (for example, --version, --help, and others).
        """

        if len(settings) >= 1:
            for extra in settings:
                option = extra.split("=") if "=" in extra else extra.split(" ")
                if option[0] in VALID_FLORESTAD_EXTRA_ARGS:
                    self.settings.extend(option)
                else:
                    raise ValueError(f"Invalid extra_arg '{option}'")
