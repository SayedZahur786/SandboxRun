# SandboxRun TypeScript client

A small fetch-based client for the SandboxRun HTTP API. Requires Node.js 20 or later, or a browser with Fetch. It does not retry execution requests.

From the repository root:

```sh
npm ci
npm run build:sdk
npm run test:sdk
```

Use the built client in a local script:

```ts
import { SandboxRun, SandboxRunError } from './sdk/dist/index.js';

const client = new SandboxRun({
  baseUrl: process.env.SANDBOXRUN_BASE_URL!,
  apiKey: process.env.SANDBOXRUN_API_KEY!,
});

try {
  const result = await client.run({
    language: 'python',
    code: 'print(6 * 7)',
    limits: { wall_time_ms: 2000 },
  }, { signal: AbortSignal.timeout(30_000) });
  // An HTTP 200 response can contain a guest error or limit termination.
  if (result.status === 'ok') console.log(result.stdout);
  else console.log(result.status);
} catch (error) {
  if (error instanceof SandboxRunError) console.error(error.status);
  else throw error;
}
```

`ExecutionStatus` is an exported union of `ok`, `guest_error`, `fuel_limit`, `time_limit`, `memory_limit`, `output_limit`, `capability_denied`, and `trap`. A disallowed blocking WASI poll terminates the guest with `capability_denied`.

`run` returns the receipt without interpreting guest output. Treat output as untrusted data. The client uses `redirect: 'error'` to avoid forwarding authenticated execution requests through redirects. Capability discovery sends no API key.

Run the end-to-end test against a running local server with `npm run test:e2e`. It defaults to `http://127.0.0.1:18787` and the repository's non-production local test key. Set `SANDBOXRUN_BASE_URL` and `SANDBOXRUN_API_KEY` to test a different authorized deployment. The test executes Python and asserts the actual receipt, parsed output, and fuel accounting.

The package is a local workspace package; these instructions do not imply that it has been published to npm.
