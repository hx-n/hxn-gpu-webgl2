//! Generates the WebGL2 wire between the Ash backend and the browser agent. Run from the
//! repository: `cargo run --manifest-path tools/wire/Cargo.toml`.
//!
//! `idl/webgl.idl` and `idl/webgl2.idl` are Khronos's, from
//! https://registry.khronos.org/webgl/specs/latest/ (1.0/webgl.idl and 2.0/webgl2.idl);
//! `idl/canvas.idl` is the HTML standard's `OffscreenCanvas`; `idl/agent.idl` is the
//! browser agent's own.

const IDL: [&str; 4] = [
    "tools/wire/idl/webgl.idl",
    "tools/wire/idl/webgl2.idl",
    "tools/wire/idl/canvas.idl",
    "tools/wire/idl/agent.idl",
];

fn main() {
    let idl: String = IDL
        .iter()
        .map(|path| std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}")) + "\n")
        .collect();
    let wire = x_idl::wire::wire(&idl).unwrap_or_else(|e| panic!("generating the wire: {e}"));
    std::fs::write("src/agent/wire.rs", wire.rust).unwrap();
    std::fs::write("page/gpu_webgl2_service.mjs", wire.js).unwrap();
}
