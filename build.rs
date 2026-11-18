use sha2::{Digest, Sha256};
fn main() {
    let mut config = wasmtime::Config::new();
    config
        .consume_fuel(true)
        .epoch_interruption(true)
        .max_wasm_stack(512 * 1024);
    config.target(&std::env::var("TARGET").unwrap()).unwrap();
    let engine = wasmtime::Engine::new(&config).unwrap();
    for (name, hash) in [
        (
            "quickjs",
            "2c218412cb8b17e46ee91bea7556ec38f5b9b6bd6312fddd3ceeed14aa71eaaa",
        ),
        (
            "python",
            "6adfb4b3637ea02008f1e32be1a4604465848ac12c2d6069779c389f6ceade58",
        ),
    ] {
        let path = format!("runtimes/{name}.wasm");
        println!("cargo:rerun-if-changed={path}");
        let bytes = std::fs::read(&path).expect("required pinned runtime missing");
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            hash,
            "runtime SHA-256 mismatch"
        );
        let compiled = engine
            .precompile_module(&bytes)
            .expect("compile trusted interpreter");
        std::fs::write(
            std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join(format!("{name}.cwasm")),
            compiled,
        )
        .unwrap();
    }
}
