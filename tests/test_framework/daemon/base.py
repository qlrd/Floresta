import os
from subprocess import Popen
from typing import List


class BaseDaemonMetaClass(type):
    """
    Metaclass for BaseDaemon.

    Ensures that any attempt to register a subclass of `BaseDaemon` will
    adheres to a standard whereby the subclass override `create` and `add_node_settings`
    but DOES NOT override either `__init__`, `kill`.

    If any of those standards are violated, a ``TypeError`` is raised.
    """

    def __new__(mcs, clsname, bases, dct):
        if not clsname == "BaseDaemon":
            if "add_daemon_settings" not in dct and "create" not in dct:
                raise TypeError(
                    "BaseDaemon subclasses must override 'add_daemon_settings'"
                )

            if "__init__" in dct and "kill" in dct:
                raise TypeError(
                    "BaseDaemon subclasses may not override '__init__' and 'kill'"
                )

        return super().__new__(mcs, clsname, bases, dct)


class BaseDaemon(metaclass=BaseDaemonMetaClass):

    def __init__(self):
        self._target = None
        self._name = None
        self._process = None
        self._settings = []

    def log(self, message: str):
        """Log a message to the console"""
        print(f"[{self._name}] {message}")

    @property
    def target(self) -> str:
        """Getter for `target` property"""
        if self._target is None:
            raise ValueError("target is not set")
        return self._target

    @target.setter
    def target(self, value: str):
        """Setter for `target` property"""
        if not os.path.exists(value):
            raise ValueError(f"Target path {value} does not exist")
        self._target = value

    @property
    def name(self) -> str:
        """Getter for `name` property"""
        if self._name is None:
            raise ValueError("name is not set")
        return self._name

    @name.setter
    def name(self, value: str):
        """Setter for `name` property"""
        if value not in ("florestad", "utreexod"):
            raise ValueError("name should be 'floresta' or 'utreexod'")
        self._name = value

    @property
    def process(self) -> Popen:
        """Getter for `process` property"""
        if self._process is None:
            raise ValueError("process is not set")
        return self._process

    @process.setter
    def process(self, value: Popen):
        """Setter for `process` property"""
        self._process = value

    @property
    def settings(self) -> List[str]:
        """Getter for `settings` property"""
        return self._settings

    @settings.setter
    def settings(self, value: List[str]):
        """Setter for `settings` property"""
        self._settings = value

    def start(self):
        """
        Start the daemon process in regtest mode. If any extra-arg is needed,
        append it with add_daemon_settings. Not all possible arguments
        are valid for tests
        """
        daemon = os.path.normpath(os.path.join(self.target, self.name))
        if not os.path.exists(daemon):
            raise ValueError(f"Daemon path {daemon} does not exist")

        cmd = [daemon]

        # verify which daemon is running and add the correct settings
        if self.name == "utreexod":
            cmd.extend(
                [
                    "--regtest",
                    "--rpcuser=utreexo",
                    "--rpcpass=utreexo",
                    "--utreexoproofindex",
                ]
            )

        elif self.name == "florestad":
            cmd.extend(["--daemon", "--network=regtest"])

        if len(self._settings) >= 1:
            cmd.extend(self._settings)

        self.process = Popen(cmd, text=True)
        self.log(f"Starting {self.name} with command: {' '.join(cmd)}")

    # pylint: disable=unused-argument
    def create(self, target: str):
        raise NotImplementedError

    # pylint: disable=unused-argument
    def add_daemon_settings(self, settings: List[str]):
        raise NotImplementedError
