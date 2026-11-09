import { createResearchApi } from './app.ts';
import { createBoundedRequestObserver } from './request-observability.ts';

const host = process.env.LOCAL_RESEARCH_API_HOST ?? '127.0.0.1';
const port = Number(process.env.LOCAL_RESEARCH_API_PORT ?? '8787');
const token = process.env.LOCAL_RESEARCH_API_TOKEN ?? '';

if (host !== '127.0.0.1' && host !== '::1') {
  throw new Error('Local research API must bind to a literal loopback IP (127.0.0.1 or ::1); hostnames and remote binding are forbidden.');
}
if (!Number.isInteger(port) || port < 1024 || port > 65535) {
  throw new Error('LOCAL_RESEARCH_API_PORT must be an integer between 1024 and 65535.');
}
if (token.length < 32) {
  throw new Error('Set LOCAL_RESEARCH_API_TOKEN to a random secret with at least 32 characters.');
}

const observeRequest = createBoundedRequestObserver(
  (event) => console.info(JSON.stringify(event)),
  { maxEvents: 120, windowMs: 60_000 },
);
const app = createResearchApi({ token, expectedPort: port, observe: observeRequest });
const server = Bun.serve({
  hostname: host,
  port,
  fetch: app.fetch,
  maxRequestBodySize: 2 * 1024 * 1024,
  idleTimeout: 10,
  error(error) {
    // Do not emit secrets, request bodies, or provider payloads into logs.
    console.error(`[local-research-api] request failed: ${error instanceof Error ? error.name : 'UnknownError'}`);
    return Response.json({ error: { code: 'internal_error', message: 'Request failed safely.' } }, { status: 500 });
  },
});

console.info(`[local-research-api] listening on ${server.hostname}:${server.port}; research-only; order routes disabled`);
