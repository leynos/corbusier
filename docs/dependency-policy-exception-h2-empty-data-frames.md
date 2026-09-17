# Dependency policy exception: `h2` empty DATA frames (RUSTSEC-2026-0258)

Corbusier's dependency audit gate (`make rust-audit`, which wraps
`cargo audit`) fails the build for any known vulnerability. This document
records an **explicit, reviewer-approved** exception that suppresses one
advisory for which no fixed release is reachable from the current HTTP server
stack.

Recorded on 2026-09-17.

## Advisory

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| Identifier       | [RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258) |
| Crate            | `h2`                                                                  |
| Resolved version | 0.3.27                                                                |
| Title            | `h2` unbounded empty DATA frames                                      |
| Class            | Denial of service                                                     |
| Fixed in         | 0.4.16 and later                                                      |

Table 1: advisory covered by this exception.

A peer that holds an open HTTP/2 connection can send an unbounded stream of
empty DATA frames. The frames consume server processing time without advancing
flow control, so a single connection can occupy server resources indefinitely.

## Why the advisory cannot be fixed

The vulnerable crate is a transitive dependency of the Actix HTTP server, and
no release in its dependency chain accepts a patched `h2`:

- `h2` 0.3.27 is the newest release on the 0.3 series, so no patch-level
  upgrade exists. The fix lands only in the 0.4 series, from 0.4.16.
- The only path to `h2` is `actix-web` 4, through `actix-http` 3. Every
  `actix-http` 3.x release, including the newest (3.13.6), declares
  `h2 = "0.3.27"`. Moving `actix-http` forward within its major series
  therefore changes nothing.
- The `http2` feature that pulls in `h2` cannot be switched off locally. The
  `actix_v2a` dependency, recorded under its own
  [dependency-policy exception](dependency-policy-exception-actix-v2a.md),
  declares `actix-web = "4"` with default features, so the feature is
  re-enabled through that crate regardless of the features Corbusier selects.

Migrating off `actix-web` 4 onto a major version that consumes `h2` 0.4 is the
only genuine remedy, and it is out of scope for a dependency-audit fix.

## Exposure

The exposure is limited by how the server listens rather than by network
placement:

- `src/main.rs` starts the server with `HttpServer::bind`, which serves
  HTTP/1.1 only. Neither `bind_auto_h2c` (plaintext HTTP/2) nor a Transport
  Layer Security (TLS) binding such as `bind_rustls` or `bind_openssl` is used,
  and TLS is what would otherwise negotiate HTTP/2 through Application Layer
  Protocol Negotiation (ALPN).
- The `h2` code is therefore compiled into the binary but is not reachable
  through any listener Corbusier configures.
- The Helm chart supports this reading of the deployment. The default
  `values.yaml` publishes a `ClusterIP` service on port 8080 and leaves
  `ingress.enabled` set to `false`, so the plaintext listener is not exposed
  outside the cluster by default. Where an ingress is enabled, its TLS
  configuration terminates at the ingress and the backend connection remains
  the plaintext HTTP/1.1 service port.

No claim is made about the trust level of clients that reach the service port,
because the chart leaves network policy and ingress configuration to the
deployment. The mitigating factor recorded here is the absence of an HTTP/2
listener, not the presence of a trusted boundary.

## Review trigger

Remove the ignore entry from `.cargo/audit.toml`, along with this document,
when any of the following becomes true:

- `actix-http` releases a version that depends on `h2` 0.4.16 or later, or
  Corbusier moves to an `actix-web` major version that does.
- The [`actix_v2a` exception](dependency-policy-exception-actix-v2a.md) is
  retired, if its replacement no longer forces `actix-web` default features and
  the `http2` feature can then be disabled.
- An HTTP/2 listener is introduced, whether through `bind_auto_h2c` or a TLS
  binding. This raises the exposure from unreachable to live, and the exception
  must be reassessed before that change ships.

## Suppression record

The advisory is suppressed in `.cargo/audit.toml`:

```toml
[advisories]
ignore = [
    # h2 0.3.27 unbounded empty DATA frames. No fixed release is reachable
    # while the server runs on actix-web 4; see
    # docs/dependency-policy-exception-h2-empty-data-frames.md.
    "RUSTSEC-2026-0258",
]
```
