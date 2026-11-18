# SandboxRun launch gate

Date: 19 November 2026 (Asia/Kolkata). Release scope: authenticated experimental preview for ephemeral agent code execution. No production SLA or external security certification is claimed.

## Product and implementation

- [x] SPEC.md written before implementation, with research-engineer and builder persona discussion, five flows, data model, threat model and exclusions.
- [x] Rust library and Wasmtime 50.0.2 execution; pinned CPython 3.15.2 and QuickJS-NG 0.18.0 WASI interpreters.
- [x] Host filesystem/network/environment denied by default. Python's immutable stdlib is embedded in guest memory; no host directory is mounted.
- [x] Fuel, guest linear memory, table/instance/stack, deadline, input and output bounds. Blocking WASI waits trap. Two executions per service instance; excess receives 429.
- [x] Authenticated HTTP API, capability discovery, OpenAPI contract, typed TypeScript client, workbench and examples.
- [x] No automatic execution retries, code persistence or application logging of source/stdin/output/keys.

## Verification and security

- [x] Five Rust unit tests pass, including raw-WASI no-preopen/path-open/poll checks, artifact integrity and invalid-UTF8 output caps.
- [x] Four SDK tests pass and the SDK→HTTP→Python end-to-end test passes.
- [x] Forty named adversarial checks pass, with actual denial assertions, canaries, markers, timing and positive controls. These include unavailable capabilities and resource attacks; they are not 40 distinct proven sandbox exploits. [Evidence](docs/escape-results.json).
- [x] Independent implementation reviewer inspected source/tests and independently reran 40 cases before hosting. All identified blockers fixed. [Review and source hashes](docs/SECURITY_REVIEW.md).
- [x] Rust Clippy all targets with warnings denied, formatting and whitespace checks pass.
- [x] Cargo audit: zero known advisories; npm audit: zero vulnerabilities at check time. [Rust](docs/dependency-audit.json), [npm](docs/npm-audit.json).
- [x] Tracked-file deployment-key and credential-pattern scan: zero findings. Secrets excluded from Git; runtime uses a separately generated bearer key. [Scope](docs/secret-scan.json).
- [x] Interpreter hashes verified at build and initialization; dependency lockfiles and upstream licenses included. Trusted AOT code is embedded by the build, never accepted from requests.

## Experience and launch assets

- [x] Desktop browser execution tested; 390px mobile layout inspected with no horizontal overflow. Narrow-screen budget fields expanded for readability.
- [x] Guest output renders as text; labelled controls, skip link, keyboard execution shortcut and visible status/error states provided. This is a manual accessibility review, not a formal conformance certification.
- [x] README with real execution screenshots, MIT license, security disclosure instructions and compatibility/known-gap documentation.
- [x] Privacy, terms and security pages implemented; no invented customer claims, certification, statistics or testimonials.
- [x] Analytics wired through allowlisted page/run events and execution metadata logs. Unknown analytics fields rejected; no content/session replay. No durable analytics database is claimed.
- [x] Three launch-post drafts saved and not posted. [Drafts](docs/LAUNCH_POSTS.md).
- [x] Local cold-start and sequential execution benchmarks recorded with sample counts and method. [Cold start](docs/cold-start.json), [execution](docs/benchmark-local.json).

## Deployment gate

- [x] Vercel preview deployed from source commit `a95df1316f3791e8d8f51399938849e82a8e7ab3`: [open SandboxRun](https://sandboxrun-l1y3f6npl-braveeminent-gmailcoms-projects.vercel.app).
- [x] Hosted HTTP smoke: both languages; invalid-auth rejection; fuel, time, memory and output enforcement; blocking-wait denial; host-file denial; policy pages; analytics allowlist. [Evidence](docs/preview-smoke.jsonl).
- [x] TypeScript SDK end-to-end test passed against the preview; browser Python execution returned the expected JSON. Desktop and 390px viewport screenshots captured from the deployed app.
- [x] Preview benchmark recorded: ten warm samples per language. Python median 307.35ms, p95 309.57ms; JavaScript median 1.44ms, p95 1.82ms. These are the documented JSON-example execution times, not universal language benchmarks. [Raw measurements](docs/benchmark-preview.json).
- [x] Observed hosted engine initialization 647.19ms and 670.09ms in runtime logs; first observed client health request 706ms. [Method and analytics evidence](docs/hosted-startup-and-analytics.json).
- [x] Public repository pushed; private vulnerability reporting enabled. Deployment credentials delivered outside the repository. Explicit deployments prevent an unreviewed Git push from automatically publishing.

GitHub-hosted CI is **blocked by the account's billing lock**, before a runner started. GitHub's annotation says: “The job was not started because your account is locked due to a billing issue.” This is not a passing CI run and not a failing application test. All listed local and hosted smoke tests were actually executed and passed. The workflow is retained for use after the owner resolves that account restriction. [Blocked run](https://github.com/SayedZahur786/SandboxRun/actions/runs/44403292849).

## Rollback and incident procedure

If a preview regression or suspected isolation issue appears, disable execution immediately by rotating/removing SANDBOXRUN_API_KEY in the affected Vercel environment and redeploying, or disable the deployment through Vercel. Revoke the old credential; do not rely on changing only the local file. Preserve request metadata and test evidence without copying guest content into public issues. Restore a previously verified deployment through Vercel's deployment controls, then run scripts/e2e.mjs against its URL before distributing it. For the first release, there is no earlier known-good public release; disable execution while fixing it. Report vulnerabilities through the repository's private security-reporting channel.

## Known gaps accepted for this preview

Fuel is instruction accounting rather than CPU-time measurement. Guest memory bounds exclude host/runtime allocations; the Vercel function uses provider-level memory/duration limits as defense in depth. Epoch deadlines have scheduler/tick granularity, not hard real-time guarantees. There is no global multi-instance quota/rate limiter, team/billing system, package installation, persistent filesystem or production SLA. Runtime/kernel vulnerabilities and downstream prompt injection remain risks. Libraries are distributed in the repository, not published to npm/crates.io. Python's rebuild recipe currently pins macOS-arm64 build tools; its resulting Wasm is portable. Operational analytics uses provider-retained logs rather than a durable product database.
