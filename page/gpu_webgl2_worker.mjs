// The Worker that holds the WebGL2 context for an Ash program's hxn-gpu-webgl2, and runs the
// commands the program puts in its mailbox (gpu_webgl2_service.mjs, generated from WebGL's IDL).
// The program names the context handle 1, the canvas 2, and the helpers below 3.

import { start } from "./gpu_webgl2_service.mjs";

// Why there is no context, when there is none.
let missing = null;

function context(gl) {
  if (!gl) throw new Error(missing);
  return gl;
}

// What the library asks beyond WebGL's own calls (tools/wire/idl/agent.idl).
const agent = {
  compiled: (gl, shader) => context(gl).getShaderParameter(shader, gl.COMPILE_STATUS) === true,
  linked: (gl, program) => context(gl).getProgramParameter(program, gl.LINK_STATUS) === true,
  parameter: (gl, name) => String(context(gl).getParameter(name) ?? ""),
  samplerUnit(gl, program, name, unit) {
    const location = context(gl).getUniformLocation(program, name);
    if (location === null) return false;
    gl.useProgram(program);
    gl.uniform1i(location, unit);
    return true;
  },
  completed(gl) {
    const sync = context(gl).fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0);
    gl.flush();
    return new Promise((resolve, reject) => {
      const poll = () => {
        const status = gl.clientWaitSync(sync, 0, 0);
        if (status === gl.TIMEOUT_EXPIRED) return setTimeout(poll, 1);
        gl.deleteSync(sync);
        if (status === gl.WAIT_FAILED) reject(new Error("waiting on the WebGL fence failed"));
        else resolve();
      };
      poll();
    });
  },
};

start(({ canvas: page }) => {
  const roots = new Map([[3, agent]]);
  // Without the web page's canvas, which another agent may hold, the context draws
  // offscreen only, on a canvas of its own.
  const canvas = page ?? (typeof OffscreenCanvas === "function" ? new OffscreenCanvas(1, 1) : null);
  if (!canvas) {
    missing = "the web page has no canvas for WebGL2";
    return roots;
  }
  const gl = canvas.getContext("webgl2", { antialias: false });
  if (!gl) {
    missing = "this browser has no WebGL2";
    return roots;
  }
  gl.canvas.addEventListener("webglcontextlost", () => console.error("WebGL2 context lost"));
  roots.set(1, gl);
  roots.set(2, canvas);
  return roots;
}, (wire) => {
  // A command that throws has no reply to carry the failure, so it goes to the shim,
  // which ends the web page with it.
  wire.error = (e, op) => {
    console.error(`${op}:`, e);
    self.postMessage({ fail: `${op}: ${e && e.stack !== undefined ? e.stack : e}` });
  };
});
