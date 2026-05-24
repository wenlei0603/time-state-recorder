import { createReadStream } from "node:fs";
import { access, stat } from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const distDir = path.join(root, "dist");

const options = parseArgs(process.argv.slice(2));
const host = options.host ?? "127.0.0.1";
const port = Number(options.port ?? "5173");
const apiBase = new URL(options.api ?? "http://127.0.0.1:4317");

const mimeTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".ico", "image/x-icon"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".map", "application/json; charset=utf-8"],
  [".png", "image/png"],
  [".svg", "image/svg+xml"],
  [".webp", "image/webp"],
]);

const server = http.createServer(async (request, response) => {
  try {
    const requestUrl = new URL(request.url ?? "/", `http://${host}:${port}`);

    if (requestUrl.pathname.startsWith("/api/") || requestUrl.pathname === "/api") {
      proxyToCollector(request, response, requestUrl);
      return;
    }

    if (
      requestUrl.pathname.startsWith("/screenshots/") ||
      requestUrl.pathname === "/screenshots"
    ) {
      proxyToCollector(request, response, requestUrl);
      return;
    }

    await serveStatic(requestUrl, response);
  } catch (error) {
    response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
    response.end(error instanceof Error ? error.message : String(error));
  }
});

server.listen(port, host, () => {
  console.log(`Time State Recorder WebUI listening at http://${host}:${port}`);
  console.log(`Proxying collector requests to ${apiBase.origin}`);
});

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (!arg.startsWith("--")) continue;
    const key = arg.slice(2);
    const next = args[index + 1];
    if (next && !next.startsWith("--")) {
      parsed[key] = next;
      index += 1;
    } else {
      parsed[key] = "true";
    }
  }
  return parsed;
}

async function serveStatic(requestUrl, response) {
  const decodedPath = decodeURIComponent(requestUrl.pathname);
  const relativePath = decodedPath === "/" ? "index.html" : decodedPath.slice(1);
  let filePath = path.resolve(distDir, relativePath);

  if (!filePath.startsWith(distDir)) {
    response.writeHead(403, { "content-type": "text/plain; charset=utf-8" });
    response.end("Forbidden");
    return;
  }

  if (!(await isFile(filePath))) {
    filePath = path.join(distDir, "index.html");
  }

  if (!(await isFile(filePath))) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("WebUI build not found. Run npm run build or use the packaged release zip.");
    return;
  }

  const contentType = mimeTypes.get(path.extname(filePath).toLowerCase()) ?? "application/octet-stream";
  response.writeHead(200, { "content-type": contentType });
  createReadStream(filePath).pipe(response);
}

async function isFile(filePath) {
  try {
    await access(filePath);
    return (await stat(filePath)).isFile();
  } catch {
    return false;
  }
}

function proxyToCollector(request, response, requestUrl) {
  const upstreamPath = `${requestUrl.pathname}${requestUrl.search}`;
  const proxyRequest = http.request(
    {
      hostname: apiBase.hostname,
      port: apiBase.port || 80,
      path: upstreamPath,
      method: request.method,
      headers: {
        ...request.headers,
        host: apiBase.host,
      },
    },
    (proxyResponse) => {
      response.writeHead(proxyResponse.statusCode ?? 502, proxyResponse.headers);
      proxyResponse.pipe(response);
    },
  );

  proxyRequest.on("error", (error) => {
    response.writeHead(502, { "content-type": "text/plain; charset=utf-8" });
    response.end(`Collector proxy failed: ${error.message}`);
  });

  request.pipe(proxyRequest);
}
