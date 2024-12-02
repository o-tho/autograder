var cacheName = 'autograder';

self.addEventListener('message', event => {
  if (event.data.type === 'WASM_FILENAME') {
    self.addEventListener('install', function (e) {
      e.waitUntil(
        caches.open(cacheName).then(function (cache) {
          return cache.add('./' + event.data.filename);
        })
      );
    });
  }
});

self.addEventListener('fetch', function (e) {
  e.respondWith(
    caches.match(e.request).then(function (response) {
      return response || fetch(e.request);
    })
  );
});
