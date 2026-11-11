# Security policy

## Supported versions

Only the latest release of the screen saver (`1.x`) is supported. The project
is a personal, local-only screen saver: it makes no network requests, stores no
data outside the bundle, and collects no telemetry.

## Reporting a vulnerability

Please **do not open a public issue** for a suspected vulnerability. Use
GitHub's private vulnerability reporting:

1. Open the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Describe the issue, the macOS version, and the steps to reproduce it.

If private reporting is unavailable, contact the maintainer through the address
in the git history. Expect an initial reply within a week.

## Scope

Things worth reporting include memory-safety problems in the renderer, formula
parser inputs that crash the screen saver, and any packaging or signing issue
with the published bundle. The screen saver runs with the privileges of the
logged-in user, so a crash or hang of the host process is in scope; a
compromised macOS installation or a malicious `.saver` from another source is
not.

## Release artifacts

Release bundles are ad-hoc signed and **not notarized**. macOS Gatekeeper will
warn on first install; the README explains the quarantine workaround. Verify a
download with the SHA-256 checksum published next to each release asset.
