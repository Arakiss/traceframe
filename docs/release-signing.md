# Release Signing and Provenance

Every tagged Slod release ships signed, attested binaries plus a software
bill of materials (SBOM). This document explains how they are produced and how
to verify a binary you downloaded.

## What ships with a release

For each platform (`x86_64`/`aarch64` × Linux/macOS) a release attaches:

- `slod-<platform>.tar.gz` — the release binary.
- `slod-<platform>.tar.gz.sha256` — its SHA-256 checksum.
- `slod-<platform>.tar.gz.sigstore.json` — the Sigstore signing bundle.

The release also attaches one SBOM for the whole source tree:

- `slod-<tag>.cdx.json` — a CycloneDX SBOM.

## How signing works

Signing uses [Sigstore](https://www.sigstore.dev) in **keyless** mode. There is
no long-lived private key to leak. During the release workflow GitHub issues a
short-lived OIDC token; `cosign` uses it to obtain an ephemeral certificate from
Sigstore's Fulcio CA, signs the artifact, and records the signature in the
Rekor transparency log. The signing identity is bound to the release workflow at
the release tag:

```
https://github.com/Arakiss/slod/.github/workflows/release.yml@refs/tags/<tag>
```

The workflow aborts if its run identity is not the tag ref, so a signature can
only be produced from the exact tag it claims to come from.

In addition to the Sigstore signature, the workflow publishes a GitHub
[artifact attestation](https://docs.github.com/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds)
for each binary and for the SBOM, recording how and where they were built.

## Verifying a downloaded binary

Install [`cosign`](https://docs.sigstore.dev/cosign/installation/), then verify
the checksum and the signature. Replace `<platform>` and `<tag>` with the values
you downloaded (for example `x86_64-linux` and `v0.2.0`).

```bash
# 1) Checksum.
shasum -a 256 -c slod-<platform>.tar.gz.sha256

# 2) Signature — identity is pinned to the release workflow at the tag.
cosign verify-blob slod-<platform>.tar.gz \
  --bundle slod-<platform>.tar.gz.sigstore.json \
  --certificate-identity "https://github.com/Arakiss/slod/.github/workflows/release.yml@refs/tags/<tag>" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
```

A successful run prints `Verified OK`. If verification fails, do not run the
binary.

You can also verify the GitHub attestation with the `gh` CLI:

```bash
gh attestation verify slod-<platform>.tar.gz --repo Arakiss/slod
```

## Inspecting the SBOM

The CycloneDX SBOM lists the dependency graph used to build the release. Any
tool that reads CycloneDX JSON works; for example:

```bash
# List components with cyclonedx-cli, jq, or your SCA tool of choice.
jq '.components[].name' slod-<tag>.cdx.json
```

The SBOM also carries its own attestation, verifiable the same way as the
binaries.
