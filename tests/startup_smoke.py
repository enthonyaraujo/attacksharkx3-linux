#!/usr/bin/env python3
"""Start a packaged executable under Xvfb, without a physical mouse or desktop."""

import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time


def main():
    executable = str(Path(sys.argv[1]).resolve())
    read_fd, write_fd = os.pipe()
    server = subprocess.Popen(
        ["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1280x900x24", "-nolisten", "tcp"],
        pass_fds=[write_fd], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    os.close(write_fd)
    try:
        assert select.select([read_fd], [], [], 15)[0], "Xvfb did not start"
        display = os.read(read_fd, 64).decode().strip()
        assert display.isdigit(), "Xvfb did not allocate a display"
        with tempfile.TemporaryDirectory(prefix="attack-shark-startup-") as config:
            env = dict(os.environ, DISPLAY=f":{display}", XDG_CONFIG_HOME=config,
                       APPIMAGE_EXTRACT_AND_RUN="1", LIBGL_ALWAYS_SOFTWARE="1")
            with tempfile.TemporaryFile(mode="w+") as log:
                app = subprocess.Popen([executable], env=env, stdout=log, stderr=log,
                                       start_new_session=True)
                try:
                    time.sleep(4)
                    if app.poll() is not None:
                        log.seek(0)
                        raise AssertionError(f"Packaged app exited ({app.returncode}):\n{log.read()}")
                    print(f"PASS: {Path(executable).name} starts with packaged dependencies")
                finally:
                    # AppImage extraction starts a child process; clean up the entire
                    # test process group, never another desktop instance.
                    import signal
                    try:
                        os.killpg(app.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    app.wait(timeout=10)
    finally:
        os.close(read_fd)
        server.terminate()
        server.wait(timeout=10)


if __name__ == "__main__":
    main()
