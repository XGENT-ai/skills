const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const seed = JSON.parse(fs.readFileSync(path.join(__dirname, 'seed.json')));
const page = fs.readFileSync(path.join(__dirname, 'index.html'));
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  res.setHeader('Cache-Control', 'no-store');
  if (req.method === 'GET' && url.pathname === '/') {
    res.setHeader('Content-Type', 'text/html; charset=utf-8');
    return res.end(page);
  }
  if (req.method === 'GET' && url.pathname === '/api/tasks') {
    res.setHeader('Content-Type', 'application/json');
    if (url.searchParams.get('state') === 'error') {
      res.statusCode = 503;
      return res.end(JSON.stringify({ error: 'Fixture service unavailable' }));
    }
    return res.end(JSON.stringify({ ...seed, tasks: url.searchParams.get('state') === 'empty' ? [] : seed.tasks }));
  }
  res.statusCode = 404;
  res.end('Not found');
});

server.listen(Number(process.env.PORT || 0), '127.0.0.1', () => {
  process.stdout.write(JSON.stringify({ url: `http://127.0.0.1:${server.address().port}`, seed: seed.seed }) + '\n');
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close());
