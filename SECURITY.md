# Security policy

SandboxRun is an experimental preview for bounded execution of agent-generated source. It is not an externally certified sandbox and offers no production security SLA. Review the current implementation, pinned runtime artifacts, test evidence, and known gaps before using it for a workload.

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/SayedZahur786/SandboxRun/security/advisories/new) when available. Include the affected commit/runtime version, a minimal reproducer, expected versus observed behavior, and the capability or resource boundary affected. Remove credentials and private data from attachments.

If private reporting is unavailable, do not publish actionable exploit details in public issues. Public [GitHub issues](https://github.com/SayedZahur786/SandboxRun/issues) are for non-sensitive support or a request to establish a private reporting channel. For a self-hosted deployment, contact its operator directly.

No response deadline or bounty is promised. Do not test third-party deployments without authorization. Stop if testing affects availability or exposes data beyond your own test fixtures.

## Threat model

Untrusted source and stdin run inside fixed Python/QuickJS WASI interpreter modules, hosted by Wasmtime. Fresh instances receive no host directory preopens, no inherited environment, no network access, and no process-spawning grant. Python has an embedded immutable standard library, not a host filesystem mount. Its explicitly configured `PYTHONHOME` is not inherited host configuration.

Fuel, epoch interruption, guest linear-memory limits, output ceilings, input validation, and admission bounds constrain execution. Guest linear-memory limits do not cover all server-process allocations. Provider/container/process limits remain necessary defense in depth. WASI `poll_oneoff` triggers a typed host trap, reported as `capability_denied`, so guest code cannot catch an unsupported-operation errno and continue after attempting a blocking poll. Python sleep attempts that reach this import terminate. QuickJS `os.sleep` returns `-58` (unsupported) without blocking.

Only trusted ahead-of-time compiled artifacts embedded by the build script are deserialized; user-provided or mutable external compiled artifacts are never accepted. The build script verifies the interpreter hashes before compilation.

Trusted components include Wasmtime, interpreter binaries, the Rust host, compiler/build inputs, the operating system, and hosting infrastructure. Runtime bugs, kernel bugs, side channels, and deployment misconfiguration remain risks. A static source filter is not the isolation boundary.

## Review and testing

Public hosting requires an independent review of the escape suite and runtime implementation, and resolution of release-blocking findings. Independent implementation review is not an external audit or certification. See the repository’s launch checklist and generated reports for the evidence associated with a specific revision.

The adversarial suite checks named capabilities and resource behavior, verifies that probes actually execute, and uses positive controls and host canaries. Forty test cases are finite regression coverage, not a proof of universal escape resistance. Do not inflate the count with syntax errors or label missing APIs as kernel-level containment evidence.

## Output and credentials

Treat all guest output as untrusted data. Render as text, escape terminal control sequences, and do not allow printed instructions to override an agent’s tool policy. The execution boundary does not prevent prompt injection in returned text.

Use unique deployment API keys of at least 32 characters, supply them through environment configuration, and rotate exposed keys. The workbench keeps keys in memory rather than local storage. Do not put keys in URLs, issue reports, screenshots, or committed files. The local end-to-end test key is intentionally non-production and must never be deployed publicly.

## Runtime updates

Keep artifact provenance, checksums, and upstream licenses together. Review security advisories for Wasmtime and the guest interpreters. Changing a runtime binary requires updating its pinned digest and rerunning positive controls, adversarial tests, benchmarks, and the independent release review. Do not silently fall back to executing source on the host if a runtime fails to load.
