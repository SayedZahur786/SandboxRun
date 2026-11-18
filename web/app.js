'use strict';
const $ = (id) => document.getElementById(id);
const snippets = {
  python: {
    transform: `import json\n\n# A small transformation for an agent tool loop.\norders = [\n    {"item": "keyboard", "price": 84, "quantity": 2},\n    {"item": "cable", "price": 12, "quantity": 3},\n]\n\nresult = {\n    "total": sum(o["price"] * o["quantity"] for o in orders),\n    "items": sum(o["quantity"] for o in orders),\n}\nprint(json.dumps(result, indent=2))`,
    calculate: `import math\n\n# No packages or network needed.\nvalues = [12, 19, 24, 31, 42]\nmean = sum(values) / len(values)\nvariance = sum((x - mean) ** 2 for x in values) / len(values)\nprint("Mean:", mean)\nprint("Standard deviation:", round(math.sqrt(variance), 3))`,
    loop: `# This run should terminate when a resource budget is reached.\n# The receipt reports the actual limit reached.\nwhile True:\n    pass`,
    filesystem: `# The host filesystem is not granted to the guest.\n# A denied read is an expected guest error.\nwith open("/etc/passwd") as host_file:\n    print(host_file.read())`
  },
  javascript: {
    transform: `// A small transformation for an agent tool loop.\nconst orders = [\n  { item: "keyboard", price: 84, quantity: 2 },\n  { item: "cable", price: 12, quantity: 3 },\n];\n\nconst result = {\n  total: orders.reduce((sum, o) => sum + o.price * o.quantity, 0),\n  items: orders.reduce((sum, o) => sum + o.quantity, 0),\n};\n\nconsole.log(JSON.stringify(result, null, 2));`,
    calculate: `const values = [12, 19, 24, 31, 42];\nconst mean = values.reduce((sum, x) => sum + x, 0) / values.length;\nconst variance = values.reduce((sum, x) => sum + (x - mean) ** 2, 0) / values.length;\nconsole.log("Mean:", mean);\nconsole.log("Standard deviation:", Math.sqrt(variance).toFixed(3));`,
    loop: `// This run should terminate when a resource budget is reached.\n// The receipt reports the actual limit reached.\nwhile (true) {}`,
    filesystem: `// QuickJS exposes std helpers, but no host directories are granted.\nconst file = std.open("/etc/passwd", "r");\nif (file === null) {\n  console.log("Denied: host file is not accessible.");\n} else {\n  console.log(file.readAsString());\n  file.close();\n}`
  }
};
const statusLabels = {
  ok: 'Completed',
  guest_error: 'Guest error',
  fuel_limit: 'Fuel limit',
  time_limit: 'Time limit',
  memory_limit: 'Memory limit',
  output_limit: 'Output limit',
  capability_denied: 'Capability denied',
  trap: 'Guest trap'
};
let running = false;
let lastReceipt = null;
const fields = { fuel: 'fuel', memory_bytes: 'memory', wall_time_ms: 'wall', output_bytes: 'output' };
function analytics(event) {
  fetch('/api/events', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ event }), keepalive: true }).catch(() => {});
}
function loadExample() {
  const language = $('language').value;
  $('code').value = snippets[language][$('example').value];
  $('filename').textContent = `${$('example').value}.${language === 'python' ? 'py' : 'js'}`;
}
$('language').addEventListener('change', loadExample);
$('example').addEventListener('change', loadExample);
loadExample();
const curl = `curl ${location.origin}/api/run \\\n  -H "Authorization: Bearer $SANDBOXRUN_API_KEY" \\\n  -H "Content-Type: application/json" \\\n  -d '{\n    "language": "python",\n    "code": "print(6 * 7)",\n    "limits": { "wall_time_ms": 2000 }\n  }'`;
$('curl-example').textContent = curl;
async function copy(text, button) {
  const original = button.textContent;
  try { await navigator.clipboard.writeText(text); button.textContent = 'Copied'; }
  catch { button.textContent = 'Copy unavailable'; }
  setTimeout(() => { button.textContent = original; }, 1800);
}
$('copy-curl').addEventListener('click', () => copy(curl, $('copy-curl')));
$('copy-result').addEventListener('click', () => { if (lastReceipt) copy(JSON.stringify(lastReceipt, null, 2), $('copy-result')); });
function setMessage(message, error = false) {
  $('run-message').textContent = message;
  $('run-message').className = `run-message${error ? ' error' : ''}`;
}
async function execute(event) {
  event.preventDefault();
  if (running) return;
  const apiKey = $('api-key').value.trim();
  if (!apiKey) { setMessage('Enter your preview API key to execute code. The key stays in this tab only.', true); $('api-key').focus(); return; }
  if (!$('code').value.trim()) { setMessage('Add source code before executing.', true); $('code').focus(); return; }
  const limits = {};
  for (const [key, id] of Object.entries(fields)) {
    if ($(id).value !== '') {
      const value = Number($(id).value);
      if (!Number.isSafeInteger(value) || value < 1) { setMessage('Resource limits must be positive, safe integers.', true); $(id).focus(); return; }
      limits[key] = value;
    }
  }
  const request = { language: $('language').value, code: $('code').value, stdin: $('stdin').value };
  if (Object.keys(limits).length) request.limits = limits;
  running = true;
  $('run-button').disabled = true;
  $('run-button').textContent = 'Executing…';
  $('result-panel').setAttribute('aria-busy', 'true');
  $('result-state').textContent = 'RUNNING';
  $('result-state').className = 'result-state';
  for (const id of ['receipt-metrics', 'streams', 'raw-receipt']) $(id).hidden = true;
  $('copy-result').disabled = true;
  lastReceipt = null;
  setMessage('Executing in a fresh guest instance…');
  analytics('run_clicked');
  try {
    const response = await fetch('/api/run', { method: 'POST', headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` }, body: JSON.stringify(request), redirect: 'error' });
    const data = await response.json().catch(() => ({ error: 'The server returned a non-JSON response.' }));
    if (!response.ok) {
      const detail = typeof data.error === 'string' ? data.error : JSON.stringify(data);
      throw new Error(`HTTP ${response.status}: ${detail}`);
    }
    lastReceipt = data;
    $('result-state').textContent = (statusLabels[data.status] || 'Unknown status').toUpperCase();
    $('result-state').className = `result-state ${data.status === 'ok' ? 'success' : 'error'}`;
    $('metric-status').textContent = statusLabels[data.status] || String(data.status);
    $('metric-duration').textContent = `${Number(data.duration_ms).toLocaleString(undefined, { maximumFractionDigits: 2 })} ms`;
    $('metric-fuel').textContent = Number(data.fuel_consumed).toLocaleString();
    $('metric-truncated').textContent = data.output_truncated ? 'Yes' : 'No';
    $('stdout').textContent = data.stdout || '(no output)';
    $('stderr').textContent = data.stderr || '(no output)';
    $('receipt-json').textContent = JSON.stringify(data, null, 2);
    for (const id of ['receipt-metrics', 'streams', 'raw-receipt']) $(id).hidden = false;
    $('copy-result').disabled = false;
    setMessage(`Request ${data.request_id}. ${data.status === 'capability_denied' ? 'A disallowed blocking WASI operation terminated the guest. ' : ''}Output is untrusted guest data.`, data.status !== 'ok');
  } catch (error) {
    $('result-state').textContent = 'REQUEST FAILED';
    $('result-state').className = 'result-state error';
    setMessage(error instanceof Error ? error.message : 'Execution request failed.', true);
  } finally {
    running = false;
    $('run-button').disabled = false;
    $('run-button').textContent = '▷  Execute code  ↵';
    $('result-panel').setAttribute('aria-busy', 'false');
  }
}
$('run-form').addEventListener('submit', execute);
$('code').addEventListener('keydown', (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); $('run-form').requestSubmit(); }
});
async function capabilities() {
  try {
    const response = await fetch('/api/capabilities');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    $('capabilities').textContent = JSON.stringify(data, null, 2);
    for (const [key, id] of Object.entries(fields)) {
      if (data.defaults?.[key] != null) $(id).placeholder = String(data.defaults[key]);
      if (data.maximums?.[key] != null) $(id).max = String(data.maximums[key]);
    }
    $('connection').textContent = '● API reachable';
    $('connection').className = 'connection ready';
  } catch {
    $('connection').textContent = '○ API unavailable';
    $('capabilities').textContent = 'Capabilities could not be loaded. Check that the API service is running.';
  }
}
analytics('page_view');
capabilities();
