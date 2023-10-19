
let filesToCache = [
    './',
    './index.html',
    './cubedaw.js',
    './cubedaw_bg.wasm',
];

self.addEventListener('install', (e) => {
    e.waitUntil(
        caches.open('cubedaw').then((c) => c.addAll(filesToCache))
    );
});

self.addEventListener('fetch', (e) => {
    e.respondWith(
        caches.match(e.request).then((r) => r || fetch(e.request))
    );
});