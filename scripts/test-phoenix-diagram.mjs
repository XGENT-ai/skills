#!/usr/bin/env node
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { HELP,parseOptions, runCore } from './test-phoenix-diagram-core.mjs';

export async function main(argv = process.argv.slice(2)) {
  const stageAt=argv.findIndex(value=>value==='--stage' || value.startsWith('--stage='));
  const stage=stageAt<0?undefined:argv[stageAt]==='--stage'?argv[stageAt+1]:argv[stageAt].slice('--stage='.length);
  if(stage==='distribution' || stage==='install') {
    const rest=[...argv];rest.splice(stageAt,argv[stageAt]==='--stage'?2:1);
    return await(await import(stage==='distribution'?'./check-phoenix-diagram-candidate.mjs':'./test-phoenix-diagram-install.mjs')).main(rest);
  }
  try {
    const options = parseOptions(argv);
    if(options.help){process.stdout.write(HELP);return 0;}
    const result = await runCore(options);
    process.stdout.write(`${JSON.stringify(result)}\n`);
    return result.exitCode;
  } catch (error) {
    process.stdout.write(`${JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'test-phoenix-diagram',status:'failed',diagnostics:[{stage:error.stage || 'controller',code:error.code || 'controller/failure',message:error.message}]})}\n`);
    return error.exitCode || (typeof error.errno==='number'?4:1);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main();
}
