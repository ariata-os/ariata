/**
 * Backend origin for API + WebSocket calls.
 *
 * Two deployment shapes share this frontend:
 *  - **Desktop (box-served):** the box serves the app, so `/api` and `/ws` are
 *    same-origin and `backendOrigin` stays empty — nothing changes.
 *  - **Mobile (bundled SPA):** the app is bundled inside the Tauri binary at its
 *    own `tauri://` origin and reaches the box over the in-process iroh loopback.
 *    The mobile shell injects `window.__VIRTUES_BACKEND_ORIGIN__ =
 *    'http://127.0.0.1:7117'`, and we route `/api` + `/ws` there.
 *
 * A single global fetch interceptor (installFetchProxy) rewrites the app's
 * `/api` calls, so the ~110 existing `fetch('/api/...')` sites need no edits.
 * Only `/api` is rewritten — SvelteKit's own bundled assets/data (`/_app`,
 * route data) must keep loading from the local origin.
 */

let backendOrigin = '';

export function setBackendOrigin(origin: string): void {
  backendOrigin = origin.replace(/\/+$/, '');
}

export function getBackendOrigin(): string {
  return backendOrigin;
}

/**
 * Absolute URL for a backend path that the browser resolves from MARKUP rather
 * than through `window.fetch` — `<iframe src>`, `<img src>`, `<video src>`,
 * CSS `url()`. The fetch shim below cannot see these: it wraps `window.fetch`,
 * and an attribute-driven load never goes through it. On mobile they would
 * otherwise resolve against the bundled `tauri://` origin, which serves no
 * backend routes, and fail silently (an empty iframe, a broken image).
 *
 * No-op on desktop, where `backendOrigin` is empty and the path is already
 * same-origin.
 */
export function backendUrl(path: string): string {
  return backendOrigin ? backendOrigin + path : path;
}

/** Base WebSocket URL (y-websocket appends room/pageId). */
export function getWsUrl(path = '/ws/yjs'): string {
  if (backendOrigin) {
    return backendOrigin.replace(/^http/, 'ws') + path;
  }
  const proto = typeof window !== 'undefined' && location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = typeof window !== 'undefined' ? location.host : 'localhost:8000';
  return `${proto}//${host}${path}`;
}

/**
 * When a backend origin is set (mobile), install a global fetch shim routing
 * root-relative `/api` requests to it. No-op when unset (desktop, same-origin).
 */
export function installFetchProxy(): void {
  if (!backendOrigin || typeof window === 'undefined') return;
  const origin = backendOrigin;
  const orig = window.fetch.bind(window);

  // Backend path prefixes to route to the box. Everything else (SvelteKit's
  // /_app assets, bundled html, client route data) stays on the local origin.
  // NB: `/auth/session` is the session gate — miss it and the app thinks it's
  // unpaired and bounces to the connect screen.
  // NB: `/health` missing from this list was audit defect U16: on the phone it
  // fell through to the OTA scheme handler, which serves index.html with a 200
  // for any extension-less path — so the box-upgrade watcher never saw the box
  // go down and reported every successful phone-initiated upgrade as a
  // ten-minute failure, and the Software page's Box row rendered "—" on iOS.
  const BACKEND_PREFIXES = ['/api', '/auth', '/webhook', '/health'];
  const route = (p: string) => BACKEND_PREFIXES.some((pre) => p === pre || p.startsWith(pre + '/'));

  window.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
    if (typeof input === 'string' && route(input)) {
      return orig(origin + input, init);
    }
    if (input instanceof URL && input.origin === location.origin && route(input.pathname)) {
      return orig(origin + input.pathname + input.search, init);
    }
    if (input instanceof Request && input.url.startsWith(location.origin) && route(new URL(input.url).pathname)) {
      return orig(new Request(origin + input.url.slice(location.origin.length), input), init);
    }
    return orig(input as RequestInfo, init);
  }) as typeof window.fetch;
}

/**
 * Read the mobile shell's injected origin and wire routing. No-op on desktop
 * (the global is absent). Call once at client startup.
 */
export function initBackendFromShell(): void {
  const injected =
    typeof window !== 'undefined'
      ? (window as unknown as { __VIRTUES_BACKEND_ORIGIN__?: string }).__VIRTUES_BACKEND_ORIGIN__
      : undefined;
  if (typeof injected === 'string' && injected) {
    setBackendOrigin(injected);
    installFetchProxy();
  }
}
