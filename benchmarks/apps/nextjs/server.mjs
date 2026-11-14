// Starts the standalone Next.js server once per worker process.
//
// Node.js runs JavaScript on one thread, so Next.js serves from one core. Production deployments
// scale out with one process per core (PM2 cluster mode, several containers); this does the same
// with `node:cluster`. `BENCH_THREADS` sets the number of workers: unset or 0 means one per core,
// and 1 runs the server directly in this process without a cluster.

import cluster from "node:cluster";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { availableParallelism } from "node:os";
import { fileURLToPath } from "node:url";

const standalone = new URL("./.next/standalone/", import.meta.url);

process.env.HOSTNAME = "127.0.0.1";
process.env.BENCH_DATA_DIR ??= fileURLToPath(new URL("../../data", import.meta.url));

if (cluster.isPrimary) {
  // `next.config.ts` is serialized into the standalone server at build time.
  const { config } = JSON.parse(
    readFileSync(new URL(".next/required-server-files.json", standalone), "utf8"),
  );
  const compression = process.env.BENCH_COMPRESSION === "on" ? "on" : "off";
  if (config.compress !== (compression === "on")) {
    console.error(
      `error: built with compress: ${config.compress}, but BENCH_COMPRESSION is ${compression};` +
        " rebuild with the same setting",
    );
    process.exit(1);
  }
}

const threads = Number(process.env.BENCH_THREADS || 0);
const workers = threads > 0 ? threads : availableParallelism();

if (workers === 1 || cluster.isWorker) {
  createRequire(import.meta.url)(fileURLToPath(new URL("server.js", standalone)));
} else {
  for (let i = 0; i < workers; i++) cluster.fork();
  cluster.on("exit", (worker, code, signal) => {
    console.error(`worker ${worker.process.pid} exited (${signal ?? code})`);
  });
}
