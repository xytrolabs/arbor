#!/usr/bin/env python3
"""Measure Arbor's interactive throughput under a burst of commands.

This is the workload agentic coding produces: many commands in quick
succession, a mix of external programs, internal commands and pipelines. It
reports wall time, per-command latency and, importantly, whether every command
actually ran.

Usage:
    python3 scripts/burst_benchmark.py [--binary PATH] [--count N]

A note on methodology: an earlier version of this test read the pty with a
fixed 20 ms sleep between writes. That sleep dominated the measurement and
completely masked the effect of `$env.config.arbor.prompt_git`, making two very
different configurations look identical. This version uses a background reader
thread so the pty never throttles the shell, and waits for observable side
effects (the files the external commands create) rather than guessing at
timing.
"""
import argparse
import fcntl
import os
import pty
import re
import select  # noqa: F401  (kept for readers adapting this script)
import shutil
import struct
import termios
import threading
import time

DEFAULT_BINARY = "/home/raf/Desktop/Arbor/target/release/arbor"


def run(label, binary, config_text, n_ext, n_int, n_pipe):
    out_dir = "/tmp/arbor_bench_out"
    home = "/tmp/arbor_bench_home"
    shutil.rmtree(out_dir, ignore_errors=True)
    shutil.rmtree(home, ignore_errors=True)
    os.makedirs(out_dir, exist_ok=True)
    os.makedirs(home + "/.config/arbor", exist_ok=True)
    if config_text:
        with open(home + "/.config/arbor/config.nu", "w") as fh:
            fh.write(config_text)

    pid, fd = pty.fork()
    if pid == 0:
        env = {
            "HOME": home,
            "TERM": "xterm-256color",
            "PATH": "/usr/local/bin:/usr/bin:/bin",
            "XDG_CONFIG_HOME": home + "/.config",
            "USER": "bench",
        }
        os.execve(binary, [os.path.basename(binary)], env)

    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))

    buf = bytearray()
    stop = threading.Event()

    def reader():
        # Answer reedline's cursor-position queries or every prompt redraw stalls.
        while not stop.is_set():
            try:
                data = os.read(fd, 65536)
            except OSError:
                return
            if not data:
                return
            buf.extend(data)
            if b"\x1b[6n" in data:
                try:
                    os.write(fd, b"\x1b[1;1R")
                except OSError:
                    return

    threading.Thread(target=reader, daemon=True).start()
    time.sleep(3.0)  # banner

    cmds = [f"touch {out_dir}/t{i}.txt" for i in range(n_ext)]
    cmds += [f"1 + {i}" for i in range(n_int)]
    cmds += ["ls | length" for _ in range(n_pipe)]

    started = time.time()
    for cmd in cmds:
        os.write(fd, cmd.encode() + b"\r")

    deadline = time.time() + 180
    while time.time() < deadline:
        if len([f for f in os.listdir(out_dir) if f.endswith(".txt")]) >= n_ext:
            break
        time.sleep(0.02)
    time.sleep(1.0)
    elapsed = time.time() - started

    try:
        os.write(fd, b"exit\r")
    except OSError:
        pass
    time.sleep(0.5)
    stop.set()

    ran = len([f for f in os.listdir(out_dir) if f.endswith(".txt")])
    clean = re.sub(rb"\x1b\[[0-9;?]*[A-Za-z]", b"", bytes(buf)).decode("utf-8", "replace")
    print(
        f"{label:22} {elapsed:6.2f}s  {elapsed / len(cmds) * 1000:5.1f} ms/command  "
        f"ran={ran}/{n_ext}  errors={clean.lower().count('error:')}"
    )
    try:
        os.close(fd)
        os.waitpid(pid, 0)
    except OSError:
        pass


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=DEFAULT_BINARY)
    ap.add_argument("--count", type=int, default=100, help="number of external commands")
    args = ap.parse_args()

    print("=== Arbor interactive burst ===")
    run("prompt_git default", args.binary, None, args.count, 80, 20)
    run(
        "prompt_git = false",
        args.binary,
        "$env.config.arbor.prompt_git = false\n",
        args.count,
        80,
        20,
    )


if __name__ == "__main__":
    main()
