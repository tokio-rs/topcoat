import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Off in every benchmarked framework: the benchmark measures the
  // frameworks, not a compression codec (see the fairness notes).
  compress: false,
};

export default nextConfig;
