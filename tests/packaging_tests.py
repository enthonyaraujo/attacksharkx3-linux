#!/usr/bin/env python3
"""Build and inspect packages in temporary workspaces, without root or installs."""

import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="attack-shark-packaging-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        shutil.copytree(ROOT / "packaging", self.root / "packaging")
        shutil.copy(ROOT / "Cargo.toml", self.root)
        (self.root / "assets/icons").mkdir(parents=True)
        shutil.copy(ROOT / "assets/icons/icon.png", self.root / "assets/icons/icon.png")
        for name in ("README.md", "LICENSE"):
            shutil.copy(ROOT / name, self.root)
        shutil.copytree(ROOT / "docs", self.root / "docs")
        self.binary = self.root / "target/release/attack-shark-x3"
        self.binary.parent.mkdir(parents=True)
        shutil.copy("/bin/true", self.binary)
        self.bin = self.root / "tools"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text("#!/bin/sh\nexit 0\n")
        cargo.chmod(0o755)
        self.env = dict(os.environ, ATTACK_SHARK_BINARY=str(self.binary),
                        ATTACK_SHARK_PACKAGE_DIR=str(self.root / "out"),
                        APPIMAGE_GLIBC_MAX="2.99", APPIMAGE_STAGE_ONLY="1",
                        PATH=f"{self.bin}:{os.environ['PATH']}")

    def build(self, kind, check=True):
        return subprocess.run(["bash", f"packaging/{kind}/build-{kind}.sh"],
                              cwd=self.root, env=self.env, text=True,
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              check=check, timeout=60)

    def deb_archives(self):
        self.build("deb")
        packages = list((self.root / "out/deb").glob("*.deb")) or list((self.root / "target/deb").glob("*.deb"))
        self.assertTrue(packages, "The builder did not produce a Debian package")
        package = packages[0]
        members = subprocess.check_output(["ar", "t", package], text=True).splitlines()
        self.assertEqual(members, ["debian-binary", "control.tar.gz", "data.tar.gz"])
        result = []
        for name in members[1:]:
            content = subprocess.check_output(["ar", "p", package, name])
            result.append(tarfile.open(fileobj=io.BytesIO(content), mode="r:gz"))
        return result

    def test_deb_declares_graphics_and_abi_dependencies(self):
        control, _ = self.deb_archives()
        text = control.extractfile("./control").read().decode()
        self.assertRegex(text, r"libc6 \(>= 2\.\d+\)")
        self.assertNotIn("libhidapi", text)
        for dependency in ("libx11-6", "libxkbcommon-x11-0", "libgl1", "libegl1", "xwayland", "udev"):
            self.assertIn(dependency, text)

    def test_deb_installs_seat_scoped_rules_before_logind_acl_processing(self):
        _, data = self.deb_archives()
        rules = [x for x in data.getnames() if "/udev/rules.d/" in x and x.endswith(".rules")]
        self.assertEqual(len(rules), 1)
        self.assertLess(int(Path(rules[0]).name[:2]), 73)
        contents = data.extractfile(rules[0]).read().decode()
        self.assertIn('TAG+="uaccess"', contents)
        self.assertNotIn('MODE="0666"', contents)
        self.assertIn('"fa60"', contents)
        self.assertIn('"fa61"', contents)

    def test_deb_postinstall_refreshes_devices_without_manual_commands(self):
        control, _ = self.deb_archives()
        hook = self.root / "postinst"
        hook.write_bytes(control.extractfile("./postinst").read())
        log = self.root / "udev-calls"
        udev = self.bin / "udevadm"
        udev.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$TEST_UDEV_LOG"\n')
        udev.chmod(0o755)
        subprocess.run(["sh", hook, "configure"], env=dict(self.env, TEST_UDEV_LOG=str(log)), check=True)
        calls = log.read_text()
        self.assertIn("control --reload-rules", calls)
        self.assertIn("trigger --subsystem-match=hidraw", calls)

    def test_appimage_stages_runtime_libraries_and_permission_helper(self):
        self.build("appimage")
        appdir = self.root / "out/appimage/AppDir"
        self.assertTrue((appdir / "usr/lib/libX11.so.6").is_file())
        self.assertTrue((appdir / "usr/lib/libxkbcommon-x11.so.0").is_file())
        self.assertTrue((appdir / "usr/libexec/attack-shark-x3-install-udev").is_file())
        subprocess.run([appdir / "AppRun"], check=True)

    def test_appimage_rejects_a_newer_glibc_than_its_declared_baseline(self):
        self.env["APPIMAGE_GLIBC_MAX"] = "2.0"
        result = self.build("appimage", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("glibc", result.stdout)


if __name__ == "__main__":
    unittest.main()
