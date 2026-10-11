# Security Policy

> Tesserae is currently **pre-alpha** and has no released build. We take
> security seriously at every stage of development and appreciate responsible
> disclosure of vulnerabilities.

## Reporting a Vulnerability

You can report security vulnerabilities by opening a
[GitHub issue](../../issues).

> **Note:** Tesserae is not yet released and is not hosted as a service. Public
> issue reporting is acceptable during this stage. Once there are released
> builds in users' hands, or a hosted multiplayer service, we will switch to
> private disclosure via GitHub Security Advisories.

Include as much of the following as possible:

- Description of the vulnerability
- Steps to reproduce or proof-of-concept
- Affected version(s) or commit SHA
- Potential impact assessment
- Any suggested fix (optional)

For anything involving a mod, a world save or a look pack, please attach the
artefact that triggers it rather than describing it — a reproducer file is worth
more than a paragraph.

## Response Timeline

| Stage              | Timeframe             |
| ------------------ | --------------------- |
| Acknowledgment     | Within 48 hours       |
| Initial assessment | Within 1 week         |
| Fix development    | Depends on severity   |
| Public disclosure  | After fix is released |

We will keep you informed of progress throughout the process.

## Supported Versions

| Version        | Supported                   |
| -------------- | --------------------------- |
| Latest release | Yes — full security support |
| Previous minor | Security fixes only         |
| Older versions | No                          |

> **Note:** Tesserae is pre-alpha. All users are encouraged to run the latest
> release at all times.

## Coordinated Disclosure Policy

We follow a coordinated disclosure process:

- We work directly with reporters to understand and validate the vulnerability.
- We develop and test a fix before any public disclosure.
- We credit reporters in the security advisory (unless anonymity is requested).
- We aim to release a fix within 30 days of a confirmed report.
- We ask that reporters do not publish details before a fix is available.

## Scope

A moddable engine runs untrusted code and parses untrusted files by design. That
boundary is the security product, and it is where reports matter most.

### In Scope

- **WASM sandbox escape** — any path by which a mod reads or writes outside its
  granted capabilities, reaches the host filesystem, network or process, or
  executes native code.
- **Host API capability bypass** — a mod obtaining authority it was not granted,
  including through a host function that fails to validate its arguments.
- **Malicious world saves and chunk data** — memory unsafety, panics that are
  exploitable, or unbounded allocation while deserialising an untrusted save.
  Treat every save file as attacker-controlled.
- **Malicious mod or look-pack bundles** — archive path traversal, symlink
  escape, or writing outside the mod's own directory during install.
- **Server authority bypass** — a client mutating World state that only the
  Server may mutate, or a client reading World state it should not see.
- **Network protocol vulnerabilities** — memory safety, authentication and
  authorisation flaws, once a network transport exists.
- **Privilege escalation** on a dedicated server (instance, world, or player
  level).
- **Unsoundness in `unsafe` code** where a reachable exploit path is
  demonstrated.

### Out of Scope

- **GPU hangs or device-lost resets caused by look-pack WGSL.** A shader cannot
  escape the GPU sandbox but can hang it. This is a documented limitation
  mitigated by a trust prompt, not a vulnerability.
- Denial of Service attacks, including resource exhaustion by a mod the user
  chose to install.
- Cheating, or any attack that requires the player to already control their own
  client. Anti-cheat is not a security boundary before 1.0.
- Social engineering (phishing, etc.).
- Vulnerabilities in third-party dependencies — please report these upstream.
  Our dependency advisories are checked in CI by `cargo-deny`.
- Security issues arising from self-hosted server misconfiguration.
- `unsafe` blocks reported without a demonstrated exploit path.
