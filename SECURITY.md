# Security policy

Shadow handles sensitive data by design, so we take vulnerabilities seriously.

## Reporting a vulnerability

**Please do not open a public issue, discussion or pull request for security problems.**

Report privately through GitHub's private vulnerability reporting: open the repository's **Security** tab and choose **Report a vulnerability**. If that is not available to you, contact the maintainers through the Tiny Bubble Company GitHub organisation and ask for a private channel.

Please include:

- the affected version or commit (`shadow --version`);
- your OS and how Shadow was run (proxy, gateway, library);
- a description of the issue and its impact;
- steps to reproduce, or a proof of concept, **using fake data only**.

We aim to acknowledge reports within 3 business days and to agree on a disclosure timeline with you. We credit reporters who wish to be credited.

## Scope

In scope, for example:

- sensitive values reaching a provider when Shadow's configuration says they should be masked;
- failures to restore that could cause data to be sent to the wrong recipient;
- weaknesses in the vault encryption, key handling or file permissions;
- issues with the local CA or TLS interception, including interception of hosts that should be tunnelled;
- the proxy or gateway being usable from outside the machine with default settings;
- leaks of real values into logs.

Out of scope:

- detection misses for values no detector claims to cover (please open a regular issue or pull request);
- attacks that require control of the user account running Shadow;
- the limitations listed in the [security model](https://tiny-bubble-company.github.io/shadow/reference/security/).

## Supported versions

Until 1.0, security fixes are made on the latest release and `main`.
