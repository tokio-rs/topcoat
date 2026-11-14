/** Runs once when each server process starts, before it handles requests. */
export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    // Load the dataset and build its indexes now rather than on the first request.
    const { getStore } = await import("@/lib/store");
    getStore();
  }
}
