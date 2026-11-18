import fs from 'node:fs';
const base=process.env.SANDBOXRUN_URL||'http://127.0.0.1:18787';
const key=process.env.SANDBOXRUN_API_KEY||'sandboxrun-local-test-key-not-for-production';
const samples=[];
for (const language of ['python','javascript']){
 for(let i=0;i<11;i++) {const start=performance.now();const r=await fetch(base+'/api/run',{method:'POST',headers:{authorization:`Bearer ${key}`,'content-type':'application/json'},body:JSON.stringify({language,code:language==='python'?'import json; print(json.dumps({"answer":42}))':'console.log(JSON.stringify({answer:42}))'})});const receipt=await r.json();if(r.status!==200||receipt.status!=='ok')throw Error(JSON.stringify(receipt));samples.push({language,sequence:i,client_roundtrip_ms:performance.now()-start,execution_ms:receipt.duration_ms,fuel_consumed:receipt.fuel_consumed});}
}
const summaries=['python','javascript'].map(language=>{const x=samples.filter(s=>s.language===language);const warm=x.slice(1).map(s=>s.execution_ms).sort((a,b)=>a-b);return {language,first_execution_ms:x[0].execution_ms,warm_samples:warm.length,warm_p50_ms:warm[Math.ceil(warm.length*.5)-1],warm_p95_ms:warm[Math.ceil(warm.length*.95)-1]}});
const report={measured_at:new Date().toISOString(),base,client_platform:process.platform+' '+process.arch,methodology:'One first request followed by ten sequential warm requests per language. First request is not proof of a provider cold start. Startup-to-ready measured separately from server initialization logs. Execution includes fresh WASM instance and interpreter initialization.',summaries,samples};fs.writeFileSync(process.env.BENCHMARK_OUTPUT||'docs/benchmark-local.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(summaries));
