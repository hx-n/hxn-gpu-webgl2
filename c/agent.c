// Starts the WebGL2 browser agent for a threaded Emscripten build, from the web page's thread,
// as Ash's page does: gpu_webgl2.mjs beside the page starts the Worker that serves the
// program's mailbox, with the program's shared memory and the page's canvas.
#include <emscripten.h>

int hxn_gpu_webgl2_agent(unsigned address) {
  return MAIN_THREAD_EM_ASM_INT({
    var canvas = (typeof Module !== "undefined" && Module.canvas) || document.getElementById("canvas");
    var fail = function (reason) {
      err("gpu_webgl2: " + reason);
      abort(reason);
    };
    import(new URL("./gpu_webgl2.mjs", location.href))
      .then(function (shim) {
        shim.start({ memory: wasmMemory, address: $0, canvas: canvas, fail: fail });
      })
      .catch(function (e) {
        fail("the browser agent did not start: " + e);
      });
    return 1;
  }, address);
}
