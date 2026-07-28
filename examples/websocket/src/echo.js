const log = (text) => {
  const item = document.createElement("li");
  item.textContent = text;
  document.getElementById("log").append(item);
};

// Connect to the WebSocket endpoint served by the same host.
const socket = new WebSocket(`ws://${location.host}/echo`);

socket.addEventListener("open", () => log("connected"));

socket.addEventListener("message", (event) => {
  log(`received: ${event.data}`);
});

socket.addEventListener("close", () => log("disconnected"));

document.getElementById("form").addEventListener("submit", (event) => {
  event.preventDefault();

  const input = document.getElementById("input");

  // Send only non-empty messages while the connection is open.
  if (input.value && socket.readyState === WebSocket.OPEN) {
    log(`sent: ${input.value}`);
    socket.send(input.value);
    input.value = "";
  }
});