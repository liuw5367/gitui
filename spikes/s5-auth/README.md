# S-5 — local HTTP Git auth fixture

```sh
python3 spikes/s5-auth/run.py
```

Starts a local `git http-backend` with basic auth (`demo` / `secret`) and tests:

- no credentials → fail
- wrong password → fail
- correct password → `git ls-remote` succeeds
- slow server → timeout
- non-fast-forward push → rejected
- server stopped → connection refused

Credentials are passed with `GIT_ASKPASS`, never in the URL or argv.

Not covered: SSH host-key changes, proxy/TLS, timeout/cancel, non-fast-forward, Keychain.
