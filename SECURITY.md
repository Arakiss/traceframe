# Security Policy

Traceframe records local evidence about AI agent runs. It is not a sandbox,
permission gateway, or secret manager.

## Supported Versions

Traceframe is pre-1.0. Security fixes target the current `main` branch until a
published release channel exists.

## In Scope

Please report privately if you find:

- trace parsing behavior that can cause unsafe file reads or writes outside the
  requested path;
- HTML report rendering that permits script execution from trace payloads;
- ledger or report behavior that silently treats malformed trace evidence as
  valid;
- command wrapper behavior that misreports a failed command as successful;
- a vulnerability in release, CI, or packaging configuration.

## Out of Scope

- Traceframe allowing a command to run when the user explicitly invoked
  `traceframe exec` or `traceframe run`.
- Missing sandboxing or permission enforcement. That belongs in the runtime,
  host agent, OS confinement, or a policy layer such as Gommage.
- Secrets already present in a trace payload supplied by the caller.

## Reporting

Use GitHub's private security advisory flow when available:

<https://github.com/Arakiss/traceframe/security/advisories/new>

If that is unavailable, email `petruarakiss@gmail.com` with the subject prefix
`[traceframe-security]`.

Include:

- affected commit or version;
- operating system;
- exact command or trace file needed to reproduce;
- expected and actual behavior;
- whether the issue affects trace integrity, report rendering, or release
  trust.
