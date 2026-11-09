#!/usr/bin/env node
// Test server for capture fixtures: serves a directory over http on
// 127.0.0.1, answers every non-GET with 200 JSON and prints each one as a
// JSON line ({"method","path"}) so tests can see what reached the server.
// Usage: node serve.mjs <root> [--port N]; the first line is {"url": ...}.
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[2] || '.');
const portFlag = process.argv.indexOf('--port');
const port = portFlag > 0 ? Number(process.argv[portFlag + 1]) : 0;
const types = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml' };

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    let body = '';
    req.on('data', (chunk) => { body += chunk; });
    req.on('end', () => {
      process.stdout.write(JSON.stringify({ method: req.method, path: url.pathname }) + '\n');
      res.writeHead(200, { 'content-type': 'application/json', 'access-control-allow-origin': '*' });
      res.end('{"ok":true}');
    });
    return;
  }
  let file = path.join(root, decodeURIComponent(url.pathname));
  if (!file.startsWith(root)) { res.writeHead(403).end(); return; }
  if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, 'index.html');
  fs.readFile(file, (error, data) => {
    if (error) { res.writeHead(404, { 'content-type': 'text/plain' }).end('not found'); return; }
    res.writeHead(200, { 'content-type': types[path.extname(file)] || 'application/octet-stream', 'cache-control': 'no-store' });
    res.end(req.method === 'HEAD' ? undefined : data);
  });
});
server.listen(port, '127.0.0.1', () => {
  process.stdout.write(JSON.stringify({ url: `http://127.0.0.1:${server.address().port}/` }) + '\n');
});
