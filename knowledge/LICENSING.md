# Offline licensing

Extraction requires an offline, Ed25519-signed license, checked with no network call. That
covers `synthpass <file>`, `synthpass batch` and `synthpass-serve`. Every other `synthpass`
command works without one — you need `fingerprint` to get one in the first place. For local
development, skip enforcement entirely:

```powershell
$env:SYNTHPASS_LICENSE_SKIP = "1"
```

> **No production license can be issued yet.** The verifying key compiled into the binary is
> still a placeholder ([`technical_debt.md`](technical_debt.md#the-licensing-public-key-is-still-a-placeholder)),
> so the flows below describe the mechanism, not an offering; distribution is source-build only
> until it is replaced ([`ROADMAP.md`](ROADMAP.md) M8). What the crate is for is in
> [`BRANDING.md` §5](BRANDING.md#5-commercial-strategy).

## Design

This is [`ARCHITECTURE.md`](ARCHITECTURE.md) §6. The code is
[`crates/synthpass-license/`](../crates/synthpass-license/). Licensing is metering and
entitlement for the official pre-built binary, not a feature gate.

- **Format.** A license file (`license.synthpass` by default, `SYNTHPASS_LICENSE_PATH` to
  override) is a small JSON envelope: `payload`, the base64 of the exact signed
  `LicensePayload` JSON bytes, and `signature`, a base64 Ed25519 signature over those same
  bytes. The verifier checks the signature over the stored bytes and only then deserializes.
  Nothing is re-serialized before verification, so field order or whitespace can never make
  a valid license fail. Verification uses `verify_strict`, which rejects non-canonical
  signatures (RFC 8032).
- **Embedded public key.** `crates/synthpass-license/pubkey.b64`, loaded with `include_str!`.
  A public key is not a secret, and rotation is a one-file swap. `SYNTHPASS_LICENSE_PUBKEY`
  overrides it at runtime for testing. The pinned key is a development placeholder: a real
  deployment runs `synthpass-license-issuer keygen` and replaces the file first.
- **Machine fingerprint (optional binding).** `machine_fingerprint()` hashes `/etc/machine-id`
  (falling back to `/var/lib/dbus/machine-id`) with the same `sha256_hex` the audit log uses.
  Hostname and CPU brand are deliberately not used: a hostname is trivially changed, and a CPU
  brand is identical across thousands of machines. On a Linux host with neither file, a random
  id is persisted on first run (`SYNTHPASS_INSTANCE_ID_PATH`). An empty `hw_fingerprint` in the
  payload skips the check (site and trial licenses).
- **Enforcement lives in the binaries, not `synthpass-pipeline`,** so the pipeline stays a
  license-agnostic library.
  - **`synthpass`** checks once at startup, on the extraction path only.
  - **`synthpass-serve`** verifies signature and fingerprint once at boot and refuses to start
    on an invalid, expired or mismatched license (`license_refusal()`, beside the
    non-loopback `startup_refusal()`). A cheap expiry-only comparison then runs on every
    `/api/extract` request, so a long-running server stops serving once its license expires.
  - **`SYNTHPASS_LICENSE_SKIP=1`** bypasses enforcement for development and CI.
- **Features are metered, not gated** (#494, [`BRANDING.md` §5](BRANDING.md#5-commercial-strategy)).
  A missing, invalid, expired or mismatched license still refuses extraction. A license that
  verifies but lacks a feature never blocks its use. `synthpass batch` and `synthpass export`
  print one stderr warning naming the feature, then run. `synthpass-serve`'s
  `POST /api/extract/batch` and `GET /metrics` serve the request, log one warning per feature
  per process, and count it in `synthpass_unentitled_requests_total{feature=...}`. That state
  is owned by `synthpass-serve`. `synthpass export` is not extraction, so it needs no valid
  license either. `multi-context` caps LLM parallelism rather than refusing.
- **Issuance is a separate binary.** `synthpass-license-issuer` (`keygen`, `issue-license`) is
  built only under the `vendor` feature and is never compiled into a customer binary. The
  shipped binaries contain no signing code, no private-key handling and no keygen RNG
  dependency.

## Threat model

The fingerprint binds to an OS *installation*, not to hardware. Root can read and copy
`machine-id`, and it survives a disk clone. Expiry trusts the system clock, which an
air-gapped operator can roll back. Most fundamentally, the source is public, so anyone who
rebuilds from source can strip the check.

**This meters the official pre-built binary, deters casual license-sharing and produces a
compliance artifact. It is not DRM and is not sold as tamper-proof.** Real hardware attestation
would need a TPM or HSM, which is out of scope.

## Customer flow

```powershell
cargo run -p synthpass-cli -- fingerprint                    # send this string to your vendor
# ...vendor emails back license.synthpass...
cargo run -p synthpass-cli -- verify-license license.synthpass     # confirm it before relying on it
```

## Vendor flow

The `synthpass-license-issuer` binary (`vendor` feature, never shipped to customers):

```powershell
cargo run -p synthpass-license --features vendor --bin synthpass-license-issuer -- keygen
# keep the private key offline; embed the printed public key in crates/synthpass-license/pubkey.b64

$env:SYNTHPASS_LICENSE_PRIVKEY = "<private key from keygen>"
cargo run -p synthpass-license --features vendor --bin synthpass-license-issuer -- `
  issue-license --customer "Acme Hospital" --tier enterprise --expires-in-days 365 `
  --hw <fingerprint from the customer> --out license.synthpass
```

An empty `--hw` issues an unbound (site/trial) license instead of a machine-locked one.

## Configuration

The licensing variables (`SYNTHPASS_LICENSE_PATH`, `SYNTHPASS_LICENSE_SKIP`,
`SYNTHPASS_LICENSE_PUBKEY`, `SYNTHPASS_INSTANCE_ID_PATH`, `SYNTHPASS_LICENSE_PRIVKEY`) are in
the [configuration reference](architecture/configuration.md#security-and-licensing). Exit code 3
is a license refusal ([exit codes](architecture/configuration.md#exit-codes)).
