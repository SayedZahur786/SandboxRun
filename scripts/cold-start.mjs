import {spawn} from 'node:child_process';import fs from 'node:fs';
const samples=[];
for(let i=0;i<3;i++){
 const start=performance.now();let logs='';
 const child=spawn('target/release/sandboxrun-server',[],{env:{...process.env,SANDBOXRUN_BIND:'127.0.0.1:18789',SANDBOXRUN_API_KEY:'sandboxrun-local-test-key-not-for-production'},stdio:['ignore','pipe','pipe']});child.stderr.on('data',b=>logs+=b);
 try {let ready=false;while(performance.now()-start<10000){try{if((await fetch('http://127.0.0.1:18789/api/health')).ok){ready=true;break}}catch{}await new Promise(r=>setTimeout(r,10));}if(!ready)throw Error('Startup failed');samples.push({sample:i+1,process_to_ready_ms:performance.now()-start,engine_init_ms:Number(logs.match(/startup_ms=([\d.]+)/)?.[1])});}
 finally{child.kill('SIGTERM');await new Promise(r=>child.on('exit',r));}
}
const report={measured_at:new Date().toISOString(),platform:process.platform+' '+process.arch,profile:'release',samples,methodology:'Three fresh OS processes; engine init from initialization timer, process readiness from spawn through successful HTTP health response, polling interval10ms. File-system caches are not flushed. Not a Vercel platform cold-start measurement.'};fs.writeFileSync('docs/cold-start.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
