#!/usr/bin/env python3
"""Exercise the real X11/XWayland window and SNI service; no mouse writes.

Run in a desktop session with a tray host, or under dbus-run-session with
--no-tray to verify that close still exits when the desktop has no tray.
"""

import argparse
import ctypes as c
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import time


def wait_for(check, message):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.1)
    raise AssertionError(message)


def dbus(service, path, method, *args):
    return subprocess.check_output(
        ["gdbus", "call", "--session", "--dest", service, "--object-path", path,
         "--method", method, "--", *map(str, args)], text=True, timeout=5,
    )


def window_for(pid):
    clients = subprocess.check_output(
        ["xprop", "-root", "_NET_CLIENT_LIST"], text=True,
    )
    for window in re.findall(r"0x[0-9a-f]+", clients):
        value = subprocess.check_output(
            ["xprop", "-id", window, "_NET_WM_PID"], text=True,
        )
        match = re.search(r"= (\d+)\s*$", value)
        if match:
            candidate = int(match.group(1))
            try:
                if candidate == pid or os.getpgid(candidate) == pid:
                    return int(window, 16)
            except ProcessLookupError:
                pass
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/attack-shark-x3"))
    parser.add_argument("--no-tray", action="store_true")
    parser.add_argument("--screenshot", type=Path)
    args = parser.parse_args()
    x11 = c.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = c.c_void_p
    x11.XOpenDisplay.argtypes = [c.c_char_p]
    display = x11.XOpenDisplay(None)
    assert display, "An accessible X11/XWayland desktop is required"
    x11.XDefaultRootWindow.argtypes = [c.c_void_p]
    x11.XDefaultRootWindow.restype = c.c_ulong
    x11.XInternAtom.argtypes = [c.c_void_p, c.c_char_p, c.c_int]
    x11.XInternAtom.restype = c.c_ulong
    x11.XSendEvent.argtypes = [c.c_void_p, c.c_ulong, c.c_int, c.c_long, c.c_void_p]
    x11.XFlush.argtypes = [c.c_void_p]
    x11.XCloseDisplay.argtypes = [c.c_void_p]

    class Data(c.Union):
        _fields_ = [("l", c.c_long * 5), ("b", c.c_char * 20)]

    class ClientMessage(c.Structure):
        _fields_ = [("type", c.c_int), ("serial", c.c_ulong), ("send_event", c.c_int),
                    ("display", c.c_void_p), ("window", c.c_ulong),
                    ("message_type", c.c_ulong), ("format", c.c_int), ("data", Data)]

    class Event(c.Union):
        _fields_ = [("client", ClientMessage), ("padding", c.c_long * 24)]

    def close(window):
        event = Event()
        event.client = ClientMessage(
            33, 0, 1, display, window,
            x11.XInternAtom(display, b"_NET_CLOSE_WINDOW", 0), 32, Data(),
        )
        event.client.data.l[1] = 1  # EWMH source: application
        x11.XSendEvent(display, x11.XDefaultRootWindow(display), 0,
                       (1 << 20) | (1 << 19), c.byref(event))
        x11.XFlush(display)

    with tempfile.TemporaryDirectory(prefix="attack-shark-smoke-") as config:
        env = dict(os.environ, XDG_CONFIG_HOME=config)
        with tempfile.TemporaryFile(mode="w+") as log:
            app = subprocess.Popen([str(args.binary.resolve())], env=env,
                                   stdout=log, stderr=log, start_new_session=True)
            try:
                window = wait_for(lambda: window_for(app.pid), "Window never appeared")
                extents = subprocess.check_output(
                    ["xprop", "-id", hex(window), "_NET_FRAME_EXTENTS"], text=True,
                )
                assert any(int(x) > 0 for x in re.findall(r"\d+", extents.split("=", 1)[-1])), extents
                print("PASS: desktop window manager supplies the window frame")
                actual_pid = int(subprocess.check_output(
                    ["xprop", "-id", hex(window), "_NET_WM_PID"], text=True,
                ).rsplit("=", 1)[1].strip())
                if args.screenshot:
                    root = c.c_ulong()
                    parent = c.c_ulong()
                    children = c.POINTER(c.c_ulong)()
                    count = c.c_uint()
                    x11.XQueryTree.argtypes = [c.c_void_p, c.c_ulong, c.c_void_p,
                                               c.c_void_p, c.c_void_p, c.c_void_p]
                    x11.XFree.argtypes = [c.c_void_p]
                    x11.XQueryTree(display, window, c.byref(root), c.byref(parent),
                                   c.byref(children), c.byref(count))
                    if children:
                        x11.XFree(children)
                    frame = parent.value if parent.value != root.value else window
                    subprocess.run(["import", "-window", hex(frame), str(args.screenshot)], check=True)
                if args.no_tray:
                    close(window)
                    assert app.wait(timeout=15) == 0
                    print("PASS: close exits normally without a tray host")
                    return

                def find_item():
                    items = dbus("org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher",
                                 "org.freedesktop.DBus.Properties.Get",
                                 "org.kde.StatusNotifierWatcher", "RegisteredStatusNotifierItems")
                    return next((item for item in re.findall(r"'([^']+)'", items)
                                 if f"-{actual_pid}-" in item), None)

                item = wait_for(find_item, "StatusNotifierItem never registered")
                service, _, path = item.partition("/")
                path = "/" + path if path else "/StatusNotifierItem"
                for cycle in range(2):
                    close(window)
                    wait_for(lambda: window_for(app.pid) is None, "Close did not hide the window")
                    if cycle == 0:
                        time.sleep(2.5)  # Cover a complete background device poll.
                        assert window_for(app.pid) is None, "A background repaint reopened the hidden window"
                    assert app.poll() is None, "Close terminated instead of hiding"
                    dbus(service, path, "org.kde.StatusNotifierItem.Activate", 0, 0)
                    window = wait_for(lambda: window_for(app.pid), "Tray did not restore the window")
                print("PASS: close hides; tray restores (two cycles)")
                menu_path = dbus(service, path, "org.freedesktop.DBus.Properties.Get",
                                 "org.kde.StatusNotifierItem", "Menu")
                menu_path = re.search(r"'(/[^']+)'", menu_path).group(1)
                layout = dbus(service, menu_path, "com.canonical.dbusmenu.GetLayout", 0, -1, "[]")
                quit_id = re.search(r"\((\d+), \{'label': <'Sair'>", layout).group(1)
                dbus(service, menu_path, "com.canonical.dbusmenu.Event", quit_id, "clicked", "<0>", 0)
                assert app.wait(timeout=15) == 0
                print("PASS: tray Quit terminates cleanly")
            except Exception:
                log.seek(0)
                print(log.read())
                raise
            finally:
                try:
                    os.killpg(app.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                app.wait(timeout=5)
                x11.XCloseDisplay(display)


if __name__ == "__main__":
    main()
