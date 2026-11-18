# SandboxRun specification

Date: 2026-11-19. Stage: implementation specification; measurements pending.

## User job and product promise
Builders of code-running agents need to execute model-generated snippets without granting the model the service's filesystem, environment, network, or an unbounded compute budget. SandboxRun runs Python and JavaScript inside fixed WebAssembly interpreters using Rust and Wasmtime and returns an inspectable execution receipt. It provides a Rust library, TypeScript client, HTTP API, and browser workbench.

The niche is small, ephemeral transformations and calculations inside agent tool loops. It is not a general development machine. A successful sandbox run does not make its output trustworthy instructions for another agent.

## Five core flows
1. Local library: construct an engine, submit source and explicit limits, inspect a typed receipt.
2. Hosted API: authenticate with a bearer key, submit POST /v1/execute, receive a bounded synchronous result. GET /v1/capabilities documents exact supported limits and interpreter identities.
3. Agent integration: use the TypeScript client and branch on success, guest error, or resource termination. Never automatically retry execution.
4. Browser workbench: choose Python or JavaScript, edit source and budgets, execute, inspect stdout/stderr and the receipt. API keys stay in memory and are never persisted in browser storage.
5. Operator verification: fetch checksum-pinned interpreters, test capability denial and limits, measure startup and execution, obtain independent implementation review, deploy, smoke test, and retain evidence.

## Runtime and threat model
Trusted: host Rust code, pinned Wasmtime dependencies, approved interpreter artifacts and compiler, provider runtime/kernel. Untrusted: source, stdin, requests, guest outputs. No user module compilation or native language execution. Fresh Store and WASI context per execution; no inherited environment, filesystem preopens, sockets, or stdio. Python's standard library must be embedded/virtualized, with no host-directory grant. Final interpreter choice and compatibility must be recorded before launch.

CPU work is bounded by Wasmtime fuel (instruction accounting, not elapsed CPU milliseconds). Epoch interruption bounds guest execution wall time. Memory limits cover guest linear memory, with table/instance/stack bounds. Input, combined or per-stream output, request concurrency and API access are independently bounded. Provider process memory and request duration are defense in depth, and their exact scope must be documented. Host initialization/compilation time is reported separately. No claim of mathematical safety or immunity to runtime vulnerabilities.

## API and data model
ExecutionRequest: language enum; UTF-8 code; stdin; optional Limits. Reject unknown fields, invalid ranges, unsupported languages, and oversized bodies before executing. Limits: fuel, memory_bytes, wall_time_ms, output_bytes. Effective defaults and ceilings live in one source of truth.

ExecutionReceipt: request_id; terminal status; stdout; stderr; output truncation; effective limits; duration; fuel consumed; runtime identity. Guest failures and limit outcomes are HTTP 200; malformed requests 400; missing/invalid credentials 401; oversized requests 413; admission saturation 429; runtime unavailable 503. No durable execution records, code logging, stdin logging or output logging.

RuntimeManifest: artifact URL, version, SHA-256, upstream license, packaging recipe. ServerConfig: auth secret, deployment identity, immutable policy bounds. Analytics: aggregate language/status/duration counters and page views without source, input, output, keys, cookies or session replay. Non-durability must be disclosed if using logs or memory only.

## UX
Technical workbench with clear status, visible policy, keyboard-accessible editor and controls, mobile layout, copyable integration examples, and readable execution receipts. Guest output renders only as text. Include real example snippets for both languages and limit behavior, compatibility notes, privacy, terms and security disclosure pages.

## Verification and launch gates
Unit-test validation and policy primitives; integration-test both interpreters; one full HTTP/SDK end-to-end flow. Forty named adversarial attempts must have meaningful expected-denial assertions, observed outcomes and positive controls. A syntax/import failure is not automatically counted as containment evidence. Test file reads/writes/traversal, environment exposure, process creation, network capabilities, state isolation, and resource abuse. Publish what was actually tested, not a promise of universal escape resistance.

Record startup-to-ready, first execution and warm execution milliseconds with sample count, environment and methodology. Independent reviewer must inspect tests and runtime implementation before any public hosting; resolve release-blocking findings. This is independent implementation review, not external security certification.

Run launch checks for dependency advisories, secrets, reproducible dependency locks, artifact integrity, auth/admission, input/output handling, policy pages, browser layout/accessibility, tests, benchmarks, deployment smoke, rollback, README screenshots, analytics, three unsent launch drafts and known gaps.

## Out of scope
pip/npm installation; Node.js compatibility; arbitrary WASM uploads; network enablement; host mounts; persistent sessions/files; background jobs; billing; user/team administration; SLA; external security certification. No unsupported security claims or fabricated test/benchmark data.

## Design discussion decisions
Research perspective prioritizes a narrow capability boundary and real denial evidence over a large execution surface. Builder perspective prioritizes a stable receipt schema, capability discovery and clear distinctions between guest failure and transport failure. Both agree on source-as-data inside preselected interpreters, no implicit retries, no secret inheritance, and transparent interpreter compatibility. Python artifact freshness versus self-contained packaging remains an explicit engineering decision to resolve experimentally before release.
