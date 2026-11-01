"""Check provisioning exit behavior without invoking sudo or apt."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class InstallLinuxTests(unittest.TestCase):
    def run_install(self, update_exit=0, install_exit=0):
        with tempfile.TemporaryDirectory(prefix="linux-setup-fixture-") as directory:
            fixture = Path(directory)
            sudo = fixture / "sudo"
            sudo.write_text(
                '#!/bin/sh\n'
                'printf "%s\\n" "$*" >> "$APT_TEST_LOG"\n'
                'case "$2" in\n'
                '  update) exit "$APT_TEST_UPDATE_EXIT";;\n'
                '  install) exit "$APT_TEST_INSTALL_EXIT";;\n'
                '  *) exit 99;;\n'
                'esac\n'
            )
            sudo.chmod(0o755)
            log = fixture / "apt.log"
            result = subprocess.run(
                ["bash", str(ROOT / "script/install-linux.sh")],
                env={**os.environ, "PATH": f"{fixture}:{os.environ['PATH']}",
                     "APT_TEST_LOG": str(log),
                     "APT_TEST_UPDATE_EXIT": str(update_exit),
                     "APT_TEST_INSTALL_EXIT": str(install_exit)},
                capture_output=True, text=True, check=False,
            )
            return result.returncode, log.read_text().splitlines()

    def test_update_failure_stops_before_install(self):
        status, commands = self.run_install(update_exit=17)
        self.assertEqual(status, 17)
        self.assertEqual(commands, ["apt update"])

    def test_install_failure_is_preserved(self):
        status, commands = self.run_install(install_exit=23)
        self.assertEqual(status, 23)
        self.assertEqual(len(commands), 2)

    def test_success_keeps_existing_native_package_set(self):
        status, commands = self.run_install()
        self.assertEqual(status, 0)
        self.assertEqual(commands[0], "apt update")
        self.assertEqual(commands[1].split(), [
            "apt", "install", "-y", "gcc", "g++", "clang", "libfontconfig-dev",
            "libwayland-dev", "libwebkit2gtk-4.1-dev", "libxkbcommon-x11-dev",
            "libx11-xcb-dev", "libssl-dev", "libzstd-dev", "vulkan-validationlayers",
            "libvulkan1",
        ])


if __name__ == "__main__":
    unittest.main()
