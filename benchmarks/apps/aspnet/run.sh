#!/bin/sh
# Starts the published app. Run from the app directory after `dotnet publish -c Release -o bin/publish`.
set -eu

# The data directory defaults to the repository's dataset, resolved before leaving this directory.
: "${BENCH_DATA_DIR:=$(cd ../../data && pwd)}"
export BENCH_DATA_DIR

# BENCH_THREADS=N: DOTNET_PROCESSOR_COUNT=N makes the runtime size itself for N CPUs, as it does in a
# container with a CPU limit: the thread pool, the GC (workstation GC below two CPUs) and Kestrel's
# I/O queues. Outside a container nothing enforces that limit, and the thread pool's hill climbing
# would add threads on the idle cores, so the pool is also fixed at N worker threads with the
# documented thread pool settings (hexadecimal values). 0 or unset keeps the defaults for every core.
if [ "${BENCH_THREADS:-0}" != "0" ]; then
  threads_hex=$(printf '%x' "$BENCH_THREADS")
  export DOTNET_PROCESSOR_COUNT="$BENCH_THREADS"
  export DOTNET_ThreadPool_ForceMinWorkerThreads="$threads_hex"
  export DOTNET_ThreadPool_ForceMaxWorkerThreads="$threads_hex"
fi

# Like a service unit, run from the publish directory so it is the content root.
cd bin/publish
exec dotnet Harbor.dll
