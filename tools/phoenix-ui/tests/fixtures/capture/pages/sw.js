// A write from inside a service worker: an install-time POST that must not
// reach the server while capture runs with --block-writes.
self.addEventListener('install', (event) => {
  event.waitUntil(fetch('/api/sw', { method: 'POST', body: 'x' }));
});
