import assert from 'node:assert/strict';
const base=process.env.SANDBOXRUN_URL||'http://127.0.0.1:18787';
const key=process.env.SANDBOXRUN_API_KEY||'sandboxrun-local-test-key-not-for-production';
async function run(language,code,limits){const r=await fetch(base+'/api/run',{method:'POST',headers:{authorization:`Bearer ${key}`,'content-type':'application/json'},body:JSON.stringify({language,code,limits})});assert.equal(r.status,200,await r.clone().text());return r.json()}
assert.equal((await fetch(base+'/api/health')).status,200);
assert.equal((await fetch(base+'/api/run',{method:'POST',body:'{}'})).status,401);
for(const language of ['python','javascript']){const r=await run(language,language==='python'?'import json, math\nprint(json.dumps({"answer": int(math.sqrt(1764))}))':'console.log(JSON.stringify({answer:Math.sqrt(1764)}))');assert.equal(r.status,'ok',JSON.stringify(r));assert.equal(JSON.parse(r.stdout).answer,42);assert.ok(r.request_id);assert.ok(r.fuel_consumed>0);console.log(JSON.stringify({check:'main_flow',language,status:r.status,duration_ms:r.duration_ms}));}
const f=await run('javascript','while(true){}',{fuel:100000});assert.equal(f.status,'fuel_limit',JSON.stringify(f));
const t=await run('javascript','while(true){}',{fuel:2000000000,wall_time_ms:1});assert.equal(t.status,'time_limit',JSON.stringify(t));
const o=await run('javascript','for(let i=0;i<1000;i++) console.log("x".repeat(100))',{output_bytes:100});assert.equal(o.status,'output_limit',JSON.stringify(o));assert.ok(Buffer.byteLength(o.stdout)<=100);
console.log(JSON.stringify({check:'limit_enforcement',fuel:f.status,time:t.status,output:o.status}));
