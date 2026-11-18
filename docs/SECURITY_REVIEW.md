# Independent implementation review

Date: 19 November 2026. Reviewed source commit: `4b15f5aed6c7c60121f4359ab46f70fc79bb51d2`, including runtime/API hardening commit `57aed3e`.

**Disposition: no unresolved release-blocking finding in the reviewed scope for an authenticated experimental preview.** This satisfies the project's independent implementation-review gate before public hosting. Deployment and post-deployment smoke checks remain separate launch gates. This was a separate reviewing agent's code/test review, not an external security audit, penetration-test certification, or proof against all escapes. The reviewer did not author the runtime or adversarial suite.

## Scope and evidence

Reviewed `src/lib.rs`, `src/http.rs`, `build.rs`, the runtime provenance and packaging recipe, `tests/escape_suite.rs`, raw-WASI unit tests, TypeScript client, browser output handling, security/privacy statements, and deployment policy. The reviewed guest artifacts are CPython 3.15.2 with wasi-vfs 0.6.3 and QuickJS-NG 0.18.0 under Wasmtime 50.0.2.

The reviewer independently reran `cargo test --release --locked --test escape_suite` on macOS arm64. It passed in 1.90 seconds after compilation was cached. Its wrapper explicitly invokes the isolated helper; the helper's ignored entry in the outer Rust test listing does not indicate skipped adversarial coverage. The regenerated report records **40/40 passing named capability/resource checks**, positive controls before and after, and the additional invalid-UTF-8 output regression. See [results](escape-results.json) and [methodology](ESCAPE-METHODOLOGY.md).

Source hashes in that observed report match the reviewed files:

| File | SHA-256 |
| --- | --- |
| `src/lib.rs` | `349ae0f01aaff924782ee9f5bea7bc607a3710bd70bcf0c724ba25331966459d` |
| `tests/escape_suite.rs` | `5a1710a732f98fbbda1c859ea461b14bb9212c873f465e12ee79544c8125e0db` |
| `src/http.rs` | `84e405261563bd7b4b787f2dc261b380a413ccd5929e31f52721d6f40cc91f3e` |
| `build.rs` | `05e1eceffe855edb2e2724ee17326defda10ceed9ae6e870982bff093adf9b4c` |

Independent local HTTP probes also observed: wrong bearer key → 401; unknown request field → 400; negative fuel → 400; oversized body → 413; a 256 MiB allocation under a 96 MiB guest cap → `memory_limit`. These supplement rather than replace the final suite and deployment smoke tests.

## Findings resolved before hosting

1. **Blocking host calls could outlive guest epoch interruption.** The final linker shadows WASI `poll_oneoff` with a typed, non-recoverable policy denial. Python and JavaScript sleep attempts now terminate with `capability_denied`. Raw-WASI and interpreter-level checks exercise this behavior. Returning an unsupported-operation errno was rejected as the final design because interpreter/libc fallback behavior could spin or obscure the denial.
2. **Artifact loading did not enforce provenance or mandatory Python availability.** Both interpreter files are now mandatory and SHA-256 checked. The build script verifies the same pinned bytes before precompiling; native-code deserialization consumes only build-generated, executable-embedded artifacts. No request data or arbitrary filesystem cache is deserialized as compiled native code.
3. **Invalid UTF-8 could inflate returned output past its byte cap.** The reviewer reproduced a one-byte invalid stream becoming a three-byte replacement character without truncation. Final conversion caps valid UTF-8 output at a character boundary and sets the overflow flag before receipt classification. Unit and interpreter regressions now pass.
4. **Some initial tests did not establish the intended operation.** A buffered Python write was replaced with an actual `os.write` denial; wall-limit cases were separated from fuel exhaustion; explicit start markers and timing assertions were retained. Python `subprocess` import itself reaches an unsupported poll, so it is documented as a compatibility limitation and is not counted as a subprocess escape attempt. No unrelated initialization failure is accepted as a successful escape check.

## Security properties assessed

Fresh Stores/WASI contexts, no host-directory preopens, no inherited environment, explicit network denial, bounded source/stdin/output, validated limit ceilings, fuel and epoch enforcement, guest memory/table limits, authentication and per-instance admission were inspected. Python's embedded read-only standard library is not a host mount. Browser output uses text rendering; client execution calls do not automatically retry or follow redirects. Analytics omit guest contents and credentials.

The 40 cases include actual filesystem/descriptor/network denials, specifically absent capabilities, resource exhaustion and cross-run isolation. They are not 40 distinct exploit vulnerabilities. Every counted case must reach its marker, satisfy its specific expected outcome, keep host canaries unchanged, disclose no canary, and remain within output/timing checks. The scheduling allowance in the test is disclosed; it is not a real-time execution guarantee.

## Residual risks and release boundary

- Wasmtime, interpreter, compiler, kernel and hosting vulnerabilities and side channels remain in the trusted computing base. This review does not exhaust those surfaces.
- Fuel measures Wasm work, not CPU milliseconds. Guest linear memory excludes some host allocations. Provider process memory/request-duration limits are distinct controls; the local library alone is not a total-process resource jail.
- There is no global multi-instance rate limiter, per-user quota system, or external security certification. Keep the execution API authenticated and configure provider usage controls before broader access.
- Blocking waits, some Python standard-library modules, native extensions, Node APIs and arbitrary packages are unsupported. Returned text remains untrusted for downstream agents and terminal displays.
- Production deployment behavior, availability and cold-start performance require the separate smoke/benchmark evidence recorded in the launch checklist.

Changes to runtime imports, artifact pins, resource limits, auth, output handling or adversarial assertions require a fresh review of the affected boundary and rerun of relevant checks.
