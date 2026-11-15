# AGENT-THREAT-MODEL — gitui v0.2

**Status**: Draft, 2026-11-14  
**Scope**: AI Agent that can read repo context, stage, commit, and (with permission) push.  
**Stack**: Rust AgentService inside the NSH app; tool API only; no raw shell/Keychain.  
**Spec**: [../SPEC.md](../SPEC.md)  
**Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)

---

## 1. Security objectives

1. **No secret exfiltration**: tokens, SSH passphrases, Keychain data, `.env`/keys must not reach the model or logs.
2. **No unauthorized history change**: Agent cannot amend/rebase/reset/force/push without explicit permission and approval.
3. **No silent bypass**: Agent cannot use `--no-verify`, disable hooks, or auto-approve.
4. **No unintended network**: Agent cannot push to a non-allowed remote/branch.
5. **Auditability**: every plan/tool call/result is attributable and inspectable.
6. **Containment**: prompt injection from repo content cannot escalate to arbitrary commands.

---

## 2. Assets

| Asset | Sensitivity |
|---|---|
| Git credentials / tokens | critical |
| SSH keys / passphrases | critical |
| Keychain items | critical |
| Source code / diffs | high (IP) |
| Commit history / refs | high (integrity) |
| Working tree/index | high (integrity) |
| Agent plan/log | medium (may contain code) |
| Settings/audit | medium |
| Model provider account | high |

---

## 3. Actors and trust boundaries

| Actor | Trust |
|---|---|
| User | trusted for approval; may make mistakes |
| Agent (model) | **untrusted**; can be prompt-injected |
| Repo content (README, comments, issues, diffs) | **untrusted data**, never instructions |
| Git hooks | repo code; user-initiated execution only |
| Remote server | untrusted; may return malicious text |
| Model provider | external processor; data policy required |
| Local OS/Keychain | trusted platform boundary |

**Boundary 1**: model output → Agent plan → tool calls. Model output is a proposal, not permission.  
**Boundary 2**: Agent tool API → Git/services. Only allow-listed typed operations; no shell.  
**Boundary 3**: app → Keychain. Agent has no handle; AuthService only.  
**Boundary 4**: app → network. Push/fetch only through SyncService with permission checks.

---

## 4. Agent capabilities (v0.2)

Allowed, gated:

- read status/diff/log/selected files (scoped by repo and path filters)
- propose a plan
- stage paths/hunks (permission: stage)
- commit with a message (permission: commit; hooks run)
- push (permission: push; separate approval unless explicit automation)
- create branch? **No** in v0.2 unless added by ADR.
- amend/rebase/reset/force/discard: **No** in v0.2.

Denied always:

- `--no-verify`, hook disable, `core.hooksPath` change
- Keychain/credential read
- raw shell/process
- force push protected branch
- auto push without permission/approval
- reading files outside repo root or excluded sensitive paths

---

## 5. Threat table

| ID | Threat | Impact | Mitigation | Verification |
|---|---|---|---|---|
| T-01 | Prompt injection in repo content asks Agent to run shell/read secrets | RCE/exfiltration | Treat repo text as data; no shell tool; tool allow-list; instruction hierarchy; refusal | SEC-06 injection fixtures |
| T-02 | Agent sends token/.env/SSH key to model | credential leak | Path excludes; secret scanner; context redaction; no Keychain handle; user preview | SEC-04/06 |
| T-03 | Agent commits without approval | integrity | Plan approval by default; commit permission; audit | T-AGENT-01 |
| T-04 | Agent pushes without approval | remote integrity | Separate push permission + approval; protected branches; no auto push | T-AGENT-02 |
| T-05 | Agent uses `--no-verify` to bypass hooks | policy bypass | Tool API has no flag; commit path rejects; audit alert | SEC-05 |
| T-06 | Prompt injection causes force push/delete | data loss | Force/delete not in tool API; protected branch checks; confirmation | SEC-06 |
| T-07 | Malicious remote text instructs Agent | injection | Remote text untrusted; no instruction following; display as data | SEC-06 |
| T-08 | Agent log leaks secret | leak | Redaction before storage/UI; no raw env; log retention | SEC-04 |
| T-09 | Over-broad context sends entire repo | IP leak | Scope by selected files; size cap; user preview; provider policy | PRIV-01 |
| T-10 | Model provider retains/trains on code | IP/compliance | Contract/opt-out; local model option; disclosure; no secrets | PRIV-02 |
| T-11 | Tool parameter injection/path traversal | write outside repo | Canonicalize; repo-root check; `--`; reject NUL/symlink escape | SEC-02 |
| T-12 | Agent loops/retries push | rate/lock | Max steps/time; no infinite retry; backoff; user stop | T-AGENT-03 |
| T-13 | Audit log tampering | accountability | Append-only; local file permissions; no Agent write | SEC-07 |
| T-14 | Hook output contains malicious ANSI/links | UI injection | Render as plain text; strip control chars; no auto-open links | UI-SEC-01 |
| T-15 | Supply-chain compromise of model/tool dependency | RCE | Pin versions; audit; minimal deps; signature | SUP-01 |

---

## 6. Permissions model

Per repo:

```text
read: bool
stage: bool
commit: bool
push: bool
force_push: bool        // default false; not exposed v0.2
discard: bool           // default false
automation: off | commit | commit-and-push
protected_branches: [String]
allowed_remotes: [String]
```

Rules:

- Default: `read=true`, others false; automation off.
- `commit` requires hooks; Agent cannot bypass.
- `push` requires separate approval unless automation explicitly commit-and-push; protected branch always approval.
- Permission changes audited; Agent cannot change its own permissions.
- Agent cannot read Keychain or credential helper output.

---

## 7. Approval UX

- Plan shows: files to stage, commit message, branch, push target, hook expectation.
- `Approve & Run`, `Edit`, `Cancel`.
- Push approval is separate by default.
- Any permission escalation requires user action outside Agent.
- No “approve all future” for force/destructive; commit/push automation is explicit.

---

## 8. Data handling

- Send only: selected file paths, diffs, status, commit messages; no secrets/binaries/large blobs.
- Exclude by default: `.env*`, `*.pem`, `*.key`, `id_*`, `.ssh/`, credential files, `.git/config` secrets, files > size cap.
- Redact: URLs with userinfo, tokens, Authorization, SSH fingerprints? (fingerprints okay), emails? configurable.
- Retention: provider policy documented; local logs redacted; user can delete.
- No training on user code without explicit opt-in; provider contract required.

---

## 9. Audit

Record:

- plan, approval decision, permission snapshot
- tool name, arguments (redacted), result status, duration
- commit SHA, push ref/remote, hook outcome
- errors/refusals
- user stop/cancel

Audit is append-only, local, viewable; no secrets. Agent cannot modify audit.

---

## 10. Incident response

- **Suspected secret exposure**: revoke token/SSH key, rotate, inspect audit/logs, notify if required.
- **Unauthorized push**: disable Agent, force-with-lease? Do not rewrite shared history without owner; revert commit if appropriate; audit.
- **Malicious hook**: hooks are repo code; disable by removing hook (user action), run commit with review; do not auto `--no-verify`.
- **Prompt injection**: stop Agent, preserve logs, add fixture, update filters.

---

## 11. v0.2 security gates

- [ ] Tool API has no shell/Keychain/`--no-verify`.
- [ ] Prompt-injection fixtures pass (T-01/T-06/T-07).
- [ ] Secret scan/redaction tests pass (T-02/T-08/T-09).
- [ ] Push requires permission + approval; protected branches blocked.
- [ ] Audit append-only; no secrets.
- [ ] Provider data policy documented; user disclosure.
- [ ] Pen-test/abuse review of plan approval and tool args.
- [ ] Kill switch: disable Agent per repo/globally; cancel in-flight.

---

## 12. Residual risks

- Model may still infer sensitive code from context; scope/redaction reduces, not eliminates.
- Provider retention/training depends on contract; disclose.
- Hooks are arbitrary code; user-initiated execution only; no sandbox guarantee.
- User may approve a bad plan; UI must show exact files/message/push target, not a summary only.
- No security model can prevent a user with full repo access from harming themselves; goal is informed consent and recoverability.
