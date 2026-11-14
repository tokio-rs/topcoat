import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // A self-contained server in `.next/standalone`, the recommended way to self-host.
  output: "standalone",
  // Next.js gzips responses unless told otherwise. The config is serialized into the standalone
  // server at build time, so `BENCH_COMPRESSION` must be set when building; `server.mjs` checks
  // that the build matches the environment it runs in.
  compress: process.env.BENCH_COMPRESSION === "on",
};

export default nextConfig;
