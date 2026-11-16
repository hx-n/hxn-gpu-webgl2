// The web page imports this when an Ash program's hxn-gpu-webgl2 asks for its browser agent through
// `ash_host_agent("gpu_webgl2", mailbox)`. It starts the Worker that holds the WebGL2 context and
// serves the program's mailbox, and hands it the page's canvas when the program first asks for it.

let worker;

export function start({ memory, address, canvas, fail }) {
  if (worker) throw new Error("the WebGL2 browser agent is already running");
  if (!(memory.buffer instanceof SharedArrayBuffer)) {
    throw new Error("hxn-gpu-webgl2 needs the program's memory shared: a threaded Ash build");
  }
  worker = new Worker(new URL("./gpu_webgl2_worker.mjs", import.meta.url), { type: "module" });
  // A command that throws in the Worker, or the Worker failing, ends the web page with the
  // reason, where Ash passes `fail`, and stops the Worker so the commands after it add no
  // errors of their own. An older Ash only has the console, and the Worker keeps serving.
  let failed = false;
  const end = (reason) => {
    if (typeof fail !== "function") return console.error(reason);
    if (failed) return;
    failed = true;
    worker.terminate();
    fail(reason);
  };
  worker.onmessage = ({ data }) => {
    if (data && data.fail !== undefined) end(data.fail);
    else if (data && data.claim) claim(canvas);
  };
  worker.onerror = (event) => end(event.message || "the WebGL2 browser agent's Worker failed");
  worker.postMessage({ memory, address });
}

// The agent's claim on the page's canvas, answered with the canvas moved to its Worker, or
// null when the page has none or another hxn browser agent holds it. Only one agent on a
// page can hold the canvas, so it goes to the first program part that asks.
function claim(canvas) {
  let offscreen = null;
  if (canvas && (globalThis.hxnCanvasHolder === undefined || globalThis.hxnCanvasHolder === "gpu_webgl2")) {
    try {
      offscreen = canvas.transferControlToOffscreen();
      canvas.hidden = false;
      globalThis.hxnCanvasHolder = "gpu_webgl2";
    } catch {
      offscreen = null;
    }
  }
  worker.postMessage({ claimed: offscreen }, offscreen ? [offscreen] : []);
}
