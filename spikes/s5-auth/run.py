#!/usr/bin/env python3
"""S-5 local HTTP Git auth fixture.

Starts a local `git http-backend` server with basic auth, then runs:
  - no credentials -> fail
  - wrong password -> fail
  - correct password -> ls-remote succeeds

Credentials are supplied by GIT_ASKPASS, not in the URL or argv.
The SSH host-key case is not part of this script.
"""

import base64
import json
import os
import shutil
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path("/tmp/s5-auth-http")
WORK = Path("/tmp/s5-auth-work")
REMOTE = ROOT / "repo.git"
ASKPASS = Path("/tmp/s5-auth-askpass.sh")
CLONE2 = Path("/tmp/s5-auth-clone2")
USER = "demo"
PASSWORD = "secret"


def run(args, cwd=None, env=None):
    return subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)


def setup():
    shutil.rmtree(ROOT, ignore_errors=True)
    shutil.rmtree(WORK, ignore_errors=True)
    ROOT.mkdir(parents=True)
    WORK.mkdir(parents=True)
    subprocess.run(["git", "init", "--bare", "-q", str(REMOTE)], check=True)
    subprocess.run(["git", "init", "-q", "-b", "main"], cwd=WORK, check=True)
    subprocess.run(["git", "config", "user.name", "S5"], cwd=WORK, check=True)
    subprocess.run(["git", "config", "user.email", "s5@example.com"], cwd=WORK, check=True)
    (WORK / "a.txt").write_text("hello\n")
    subprocess.run(["git", "add", "a.txt"], cwd=WORK, check=True)
    subprocess.run(["git", "commit", "-qm", "base"], cwd=WORK, check=True)
    subprocess.run(["git", "remote", "add", "origin", str(REMOTE)], cwd=WORK, check=True)
    subprocess.run(["git", "push", "-q", "-u", "origin", "main"], cwd=WORK, check=True)
    subprocess.run(
        ["git", "--git-dir", str(REMOTE), "config", "http.receivepack", "true"],
        check=True,
    )


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.git()

    def do_POST(self):
        self.git()

    def git(self):
        expected = "Basic " + base64.b64encode(f"{USER}:{PASSWORD}".encode()).decode()
        if self.headers.get("Authorization", "") != expected:
            self.send_response(401)
            self.send_header("WWW-Authenticate", 'Basic realm="s5"')
            self.end_headers()
            return

        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if self.command == "POST" else b""
        path, _, query = self.path.partition("?")
        if "slow=1" in query:
            time.sleep(4)
        env = os.environ.copy()
        env.update(
            {
                "GIT_PROJECT_ROOT": str(ROOT),
                "GIT_HTTP_EXPORT_ALL": "1",
                "PATH_INFO": path,
                "QUERY_STRING": query,
                "REQUEST_METHOD": self.command,
                "CONTENT_TYPE": self.headers.get("Content-Type", ""),
                "CONTENT_LENGTH": str(length),
                "REMOTE_USER": USER,
                "REMOTE_ADDR": self.client_address[0],
                "SERVER_NAME": "127.0.0.1",
                "SERVER_PORT": str(self.server.server_address[1]),
                "SERVER_PROTOCOL": self.protocol_version,
            }
        )
        backend = os.path.join(
            subprocess.check_output(["git", "--exec-path"], text=True).strip(),
            "git-http-backend",
        )
        proc = subprocess.run([backend], input=body, env=env, capture_output=True)
        head, _, response_body = proc.stdout.partition(b"\r\n\r\n")
        status = 200
        headers = []
        for line in head.split(b"\r\n"):
            if not line:
                continue
            if line.lower().startswith(b"status:"):
                status = int(line.split(b" ")[1])
            else:
                key, _, value = line.partition(b":")
                headers.append((key.decode().strip(), value.decode().strip()))

        self.send_response(status)
        for key, value in headers:
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(response_body)


def main():
    setup()
    ASKPASS.write_text(
        """#!/bin/sh
if [ "${S5_FAIL:-0}" = "1" ]; then
  exit 1
fi
case "$1" in
  *sername*) printf '%s\\n' "$S5_USER" ;;
  *) printf '%s\\n' "$S5_PASS" ;;
esac
"""
    )
    ASKPASS.chmod(0o755)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    url = f"http://127.0.0.1:{port}/repo.git"
    base_env = os.environ.copy()
    base_env.update(
        {
            "GIT_ASKPASS": str(ASKPASS),
            "GIT_TERMINAL_PROMPT": "0",
            "GIT_PAGER": "cat",
        }
    )

    cases = [
        ("no_auth", {"S5_FAIL": "1"}, False),
        ("bad_password", {"S5_USER": USER, "S5_PASS": "wrong"}, False),
        ("good_password", {"S5_USER": USER, "S5_PASS": PASSWORD}, True),
    ]

    results = []
    for name, extra, expect_ok in cases:
        env = base_env.copy()
        env.update(extra)
        proc = run(["git", "ls-remote", url], env=env)
        ok = proc.returncode == 0
        results.append(
            {
                "case": name,
                "expected_ok": expect_ok,
                "ok": ok,
                "exit": proc.returncode,
                "stderr": proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "",
                "refs": len(proc.stdout.strip().splitlines()),
            }
        )

    good_env = base_env.copy()
    good_env.update({"S5_USER": USER, "S5_PASS": PASSWORD})

    slow = run(
        [
            "git",
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=1",
            "ls-remote",
            url + "?slow=1",
        ],
        env=good_env,
    )
    results.append(
        {
            "case": "timeout",
            "expected_ok": False,
            "ok": slow.returncode == 0,
            "exit": slow.returncode,
            "stderr": slow.stderr.strip().splitlines()[-1] if slow.stderr.strip() else "",
            "refs": 0,
        }
    )

    # non-fast-forward: clone, push ahead, then push from the original worktree.
    shutil.rmtree(CLONE2, ignore_errors=True)
    subprocess.run(
        ["git", "clone", url, str(CLONE2)],
        env=good_env,
        check=True,
        capture_output=True,
    )
    subprocess.run(["git", "config", "user.name", "S5"], cwd=CLONE2, check=True)
    subprocess.run(["git", "config", "user.email", "s5@example.com"], cwd=CLONE2, check=True)
    (CLONE2 / "b.txt").write_text("remote ahead\n")
    subprocess.run(["git", "add", "b.txt"], cwd=CLONE2, check=True)
    subprocess.run(["git", "commit", "-qm", "remote ahead"], cwd=CLONE2, check=True)
    subprocess.run(["git", "push", "-q", "origin", "main"], cwd=CLONE2, env=good_env, check=True)

    (WORK / "c.txt").write_text("local ahead\n")
    subprocess.run(["git", "add", "c.txt"], cwd=WORK, check=True)
    subprocess.run(["git", "commit", "-qm", "local ahead"], cwd=WORK, check=True)
    non_ff = run(["git", "push", url, "main"], cwd=WORK, env=good_env)
    results.append(
        {
            "case": "non_fast_forward",
            "expected_ok": False,
            "ok": non_ff.returncode == 0,
            "exit": non_ff.returncode,
            "stderr": non_ff.stderr.strip().splitlines()[-1] if non_ff.stderr.strip() else "",
            "refs": 0,
        }
    )

    thread.join(timeout=0.1)
    server.shutdown()
    server.server_close()

    network = run(["git", "ls-remote", url], env=good_env)
    results.append(
        {
            "case": "network_down",
            "expected_ok": False,
            "ok": network.returncode == 0,
            "exit": network.returncode,
            "stderr": network.stderr.strip().splitlines()[-1] if network.stderr.strip() else "",
            "refs": 0,
        }
    )

    print(json.dumps({"port": port, "url": url, "results": results}, indent=2))
    if any(result["ok"] != result["expected_ok"] for result in results):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
