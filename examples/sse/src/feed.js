const log = (text) => {
  const item = document.createElement("li");
  item.textContent = text;
  document.getElementById("log").append(item);
};

// The browser reconnects on its own when the connection drops, sending the id
// of the last event it saw, so the server resumes the stream without gaps.
const ticks = new EventSource("/ticks");
ticks.addEventListener("open", () => log("connected"));
ticks.addEventListener("tick", (event) => log(`tick: ${event.data}`));

// A stream that ends looks like a dropped connection to an EventSource, which
// would reconnect and start the job over. The client closes the source itself
// once the server reports the job is done.
document.getElementById("start").addEventListener("click", () => {
  const job = new EventSource("/job");
  job.addEventListener("progress", (event) => {
    log(`job: ${JSON.parse(event.data).percent}%`);
  });
  job.addEventListener("done", (event) => {
    log(`job: ${event.data}`);
    job.close();
  });
});
