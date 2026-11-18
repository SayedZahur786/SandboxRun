import test from 'node:test';
import assert from 'node:assert/strict';
import { SandboxRun, SandboxRunError } from '../dist/index.js';

test('sends one authenticated request and preserves a limit receipt', async () => {
  let calls = 0;
  const receipt = { request_id: 'test-run', status: 'fuel_exhausted', stdout: '', stderr: '', duration_ms: 4, fuel_consumed: 100, limits: { fuel: 100, memory_bytes: 65536, wall_time_ms: 100, output_bytes: 100 }, runtime: 'test', output_truncated: false };
  const client = new SandboxRun({ baseUrl: 'https://example.test/', apiKey: 'test-key', fetch: async (url, init) => {
    calls++;
    assert.equal(url, 'https://example.test/api/run');
    assert.equal(init.headers.Authorization, 'Bearer test-key');
    assert.equal(init.redirect, 'error');
    assert.deepEqual(JSON.parse(init.body), { language: 'python', code: 'while True: pass' });
    return Response.json(receipt);
  }});
  assert.deepEqual(await client.run({ language: 'python', code: 'while True: pass' }), receipt);
  assert.equal(calls, 1);
});

test('does not retry admission failure and retains HTTP error', async () => {
  let calls = 0;
  const client = new SandboxRun({ baseUrl: 'https://example.test', apiKey: 'test', fetch: async () => {
    calls++; return Response.json({ error: 'busy' }, { status: 429 });
  }});
  await assert.rejects(client.run({ language: 'javascript', code: '1' }), error => error instanceof SandboxRunError && error.status === 429 && error.body.error === 'busy');
  assert.equal(calls, 1);
});

test('forwards abort signal and does not send credentials for capability discovery', async () => {
  const controller = new AbortController();
  const client = new SandboxRun({ baseUrl: 'http://localhost:3000', apiKey: 'test', fetch: async (_, init) => {
    assert.equal(init.signal, controller.signal);
    assert.equal(init.headers, undefined);
    return Response.json({ network: false });
  }});
  assert.deepEqual(await client.capabilities({ signal: controller.signal }), { network: false });
});

test('rejects URLs that could expose credentials accidentally', () => {
  for (const baseUrl of ['file:///tmp/api', 'https://u:p@example.test', 'https://example.test?key=x']) {
    assert.throws(() => new SandboxRun({ baseUrl, apiKey: 'test' }), TypeError);
  }
});
