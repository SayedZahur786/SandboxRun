import test from 'node:test';
import assert from 'node:assert/strict';
import { SandboxRun } from '../dist/index.js';

// The default key is for a local test server only; production keys come from env.
const client = new SandboxRun({
  baseUrl: process.env.SANDBOXRUN_BASE_URL || 'http://127.0.0.1:18787',
  apiKey: process.env.SANDBOXRUN_API_KEY || 'sandboxrun-local-test-key-not-for-production'
});
test('SDK → HTTP → Python WASI guest → typed execution receipt', async () => {
  const receipt = await client.run({ language: 'python', code: 'import json\nprint(json.dumps({"answer": 6 * 7}))' });
  assert.equal(receipt.status, 'ok');
  assert.deepEqual(JSON.parse(receipt.stdout.trim()), { answer: 42 });
  assert.equal(receipt.stderr, '');
  assert.equal(typeof receipt.request_id, 'string');
  assert.ok(receipt.request_id.length > 0);
  assert.ok(receipt.duration_ms >= 0);
  assert.ok(receipt.fuel_consumed > 0);
  assert.ok(receipt.fuel_consumed <= receipt.limits.fuel);
  assert.equal(receipt.output_truncated, false);
});
