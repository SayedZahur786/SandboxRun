# Launch post drafts

These are drafts for the project owner to review and publish. They have not been sent. No performance or escape-test success count is asserted; measured results belong in the repository’s generated evidence.

## 1. Developer launch

I’m building SandboxRun: bounded code execution for agents that generate Python or JavaScript.

The implementation runs fixed WASI interpreters inside Rust + Wasmtime. Every request gets a fresh guest, an explicit fuel budget, a guest-memory ceiling, a wall deadline, and bounded stdout/stderr. Host filesystem directories and network access are not granted.

The useful part for an agent loop is the receipt: status, effective limits, runtime identity, duration, and fuel consumed. Guest errors stay separate from HTTP failures, and the TypeScript client does not silently retry execution.

This is an experimental preview, with its threat model and limitations documented. JavaScript is QuickJS, not Node. Python has an embedded standard library, not pip or host mounts. The repository includes adversarial checks and generated evidence rather than a claim that testing proves all escapes impossible.

Code, setup, and security model: https://github.com/SayedZahur786/SandboxRun

I’d especially like feedback from builders whose agents need small JSON transformations or calculations without a full development machine.

## 2. Engineering notes

A subtle part of building a code sandbox: an instruction budget, a wall deadline, and a memory limit are three different controls.

In SandboxRun, Wasmtime fuel bounds guest computational work. Epoch interruption supplies the execution deadline. Linear-memory limits constrain the guest’s memory, while deployment limits must cover the rest of the host process.

Another important edge: guest interruption does not automatically cancel a blocking host call. This preview denies WASI polling/sleeping instead of offering a wait capability that can outlive the intended budget.

The project runs Python and QuickJS through fixed WASI artifacts with no host directory preopens or network grants. It returns a structured execution receipt and includes a TypeScript client, browser workbench, and named adversarial checks.

It is a narrow preview, not an external security certification. The runtime, host, and deployment still belong in the threat model.

Implementation and evidence: https://github.com/SayedZahur786/SandboxRun

## 3. Short community post

Built SandboxRun for one agent-tool job: run a small generated snippet with explicit boundaries and inspect what happened.

• Python + JavaScript in Wasmtime/WASI
• Fuel, guest memory, wall-time, and output budgets
• No host filesystem mounts or network grants
• Rust library, HTTP API, TypeScript client, browser workbench
• Structured receipts and adversarial regression evidence

Preview scope: no pip/npm, no Node compatibility, no persistent sessions, and no claim of universal escape resistance. Guest output is still untrusted data.

Repository: https://github.com/SayedZahur786/SandboxRun
