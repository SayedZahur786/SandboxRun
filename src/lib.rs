use serde::{Deserialize, Serialize};
use std::{
    io,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context as TaskContext, Poll},
    time::{Duration, Instant},
};
use tokio::io::AsyncWrite;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap};
use wasmtime::{Result, bail, error::Context};
use wasmtime_wasi::{
    WasiCtxBuilder,
    cli::{IsTerminal, StdoutStream},
    p1::{self, WasiP1Ctx},
    p2::pipe::MemoryInputPipe,
};

pub const MAX_BODY: usize = 150_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub fuel: u64,
    pub memory_bytes: usize,
    pub wall_time_ms: u64,
    pub output_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel: 2_000_000_000,
            memory_bytes: 128 * 1024 * 1024,
            wall_time_ms: 2000,
            output_bytes: 16384,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<()> {
        if !(1..=5_000_000_000).contains(&self.fuel) {
            bail!("fuel must be 1..5000000000")
        }
        if !(16 * 1024 * 1024..=256 * 1024 * 1024).contains(&self.memory_bytes) {
            bail!("memory_bytes must be 16777216..268435456")
        }
        if !(1..=5000).contains(&self.wall_time_ms) {
            bail!("wall_time_ms must be 1..5000")
        }
        if !(1..=65536).contains(&self.output_bytes) {
            bail!("output_bytes must be 1..65536 per stream")
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Javascript,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub language: Language,
    pub code: String,
    #[serde(default)]
    pub stdin: String,
    #[serde(default)]
    pub limits: Limits,
}
impl ExecutionRequest {
    pub fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        if self.code.is_empty() || self.code.len() > 65536 {
            bail!("code must be 1..65536 UTF-8 bytes")
        };
        if self.code.contains('\0') {
            bail!("code cannot contain NUL")
        };
        if self.stdin.len() > 65536 {
            bail!("stdin exceeds 65536 bytes")
        };
        Ok(())
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub request_id: String,
    pub status: String,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: f64,
    pub fuel_consumed: u64,
    pub limits: Limits,
    pub runtime: String,
    pub output_truncated: bool,
}
#[derive(Clone)]
struct Output {
    bytes: Arc<Mutex<Vec<u8>>>,
    cap: usize,
    exceeded: Arc<AtomicBool>,
}
impl Output {
    fn new(cap: usize) -> Self {
        Self {
            bytes: Default::default(),
            cap,
            exceeded: Default::default(),
        }
    }
    fn text(&self) -> String {
        let mut text = String::from_utf8_lossy(&self.bytes.lock().unwrap()).into_owned();
        if text.len() > self.cap {
            let mut end = self.cap;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            self.exceeded.store(true, Ordering::Relaxed);
        }
        text
    }
}
impl IsTerminal for Output {
    fn is_terminal(&self) -> bool {
        false
    }
}
impl StdoutStream for Output {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}
impl AsyncWrite for Output {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut bytes = self.bytes.lock().unwrap();
        let n = buf.len().min(self.cap - bytes.len());
        bytes.extend_from_slice(&buf[..n]);
        if n < buf.len() {
            self.exceeded.store(true, Ordering::Relaxed);
            return Poll::Ready(Err(io::Error::other("output limit")));
        }
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
struct State {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}
pub struct Sandbox {
    engine: Engine,
    python: Option<Module>,
    javascript: Module,
    stopped: Arc<AtomicBool>,
    pub startup_ms: f64,
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}
impl Sandbox {
    pub fn new(dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let start = Instant::now();
        let mut config = Config::new();
        config
            .concurrency_support(false)
            .consume_fuel(true)
            .epoch_interruption(true)
            .max_wasm_stack(512 * 1024);
        let engine = Engine::new(&config)?;
        let dir = dir.as_ref();
        verify_artifact(
            &dir.join("quickjs.wasm"),
            "2c218412cb8b17e46ee91bea7556ec38f5b9b6bd6312fddd3ceeed14aa71eaaa",
        )?;
        verify_artifact(
            &dir.join("python.wasm"),
            "6adfb4b3637ea02008f1e32be1a4604465848ac12c2d6069779c389f6ceade58",
        )?;
        // SAFETY: bytes are compiled by this package's build script from SHA-256-pinned
        // interpreters for this target/configuration and embedded in the compiled executable.
        // No request or runtime filesystem content is ever deserialized as native code.
        let javascript = unsafe {
            Module::deserialize(
                &engine,
                include_bytes!(concat!(env!("OUT_DIR"), "/quickjs.cwasm")),
            )?
        };
        let python = Some(unsafe {
            Module::deserialize(
                &engine,
                include_bytes!(concat!(env!("OUT_DIR"), "/python.cwasm")),
            )?
        });
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let clock = engine.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(5));
                clock.increment_epoch();
            }
        });
        Ok(Self {
            engine,
            python,
            javascript,
            stopped,
            startup_ms: start.elapsed().as_secs_f64() * 1000.,
        })
    }
    pub fn execute(&self, req: ExecutionRequest) -> Result<Receipt> {
        req.validate()?;
        let start = Instant::now();
        let (module, args, runtime) = match req.language {
            Language::Javascript => (
                &self.javascript,
                vec![
                    "qjs".to_string(),
                    "--std".into(),
                    "-e".into(),
                    req.code.clone(),
                ],
                "quickjs-ng-0.18.0",
            ),
            Language::Python => (
                self.python.as_ref().context("Python runtime unavailable")?,
                vec![
                    "python".into(),
                    "-S".into(),
                    "-P".into(),
                    "-B".into(),
                    "-c".into(),
                    req.code.clone(),
                ],
                "cpython-3.15.2-wasi-vfs-0.6.3",
            ),
        };
        let stdout = Output::new(req.limits.output_bytes);
        let stderr = Output::new(req.limits.output_bytes);
        let mut ctx = WasiCtxBuilder::new();
        if matches!(req.language, Language::Python) {
            ctx.env("PYTHONHOME", "/");
        }
        ctx.args(&args)
            .stdin(MemoryInputPipe::new(req.stdin.into_bytes()))
            .stdout(stdout.clone())
            .stderr(stderr.clone())
            .allow_tcp(false)
            .allow_udp(false)
            .allow_ip_name_lookup(false);
        let limits = StoreLimitsBuilder::new()
            .memory_size(req.limits.memory_bytes)
            .table_elements(100_000)
            .instances(1)
            .tables(2)
            .memories(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(
            &self.engine,
            State {
                wasi: ctx.build_p1(),
                limits,
            },
        );
        store.limiter(|s| &mut s.limits);
        store.set_fuel(req.limits.fuel)?;
        store.set_epoch_deadline(req.limits.wall_time_ms.div_ceil(5) + 1);
        let linker = make_linker(&self.engine)?;
        let result = (|| -> Result<()> {
            let instance = linker.instantiate(&mut store, module)?;
            instance
                .get_typed_func::<(), ()>(&mut store, "_start")?
                .call(&mut store, ())?;
            Ok(())
        })();
        let stdout_text = stdout.text();
        let stderr_text = stderr.text();
        let output_truncated =
            stdout.exceeded.load(Ordering::Relaxed) || stderr.exceeded.load(Ordering::Relaxed);
        let status = if output_truncated {
            "output_limit"
        } else {
            match &result {
                Ok(_) => "ok",
                Err(e) => {
                    if e.downcast_ref::<CapabilityDenied>().is_some() {
                        "capability_denied"
                    } else if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                        if exit.0 == 0 { "ok" } else { "guest_error" }
                    } else if let Some(trap) = e.downcast_ref::<Trap>() {
                        match trap {
                            Trap::OutOfFuel => "fuel_limit",
                            Trap::Interrupt => "time_limit",
                            _ => "trap",
                        }
                    } else if format!("{e:#}").contains("memory") {
                        "memory_limit"
                    } else {
                        "guest_error"
                    }
                }
            }
        };
        Ok(Receipt {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: status.into(),
            stdout: stdout_text,
            stderr: stderr_text,
            duration_ms: start.elapsed().as_secs_f64() * 1000.,
            fuel_consumed: req.limits.fuel - store.get_fuel().unwrap_or(0),
            limits: req.limits,
            runtime: runtime.into(),
            output_truncated,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_reject_outside_bounds() {
        let mut l = Limits::default();
        assert!(l.validate().is_ok());
        l.fuel = 0;
        assert!(l.validate().is_err());
        l = Limits::default();
        l.memory_bytes = usize::MAX;
        assert!(l.validate().is_err());
        l = Limits::default();
        l.wall_time_ms = 0;
        assert!(l.validate().is_err());
        l = Limits::default();
        l.output_bytes = 65537;
        assert!(l.validate().is_err());
    }
    #[test]
    fn unknown_fields_fail() {
        assert!(
            serde_json::from_str::<ExecutionRequest>(
                r#"{"language":"python","code":"pass","network":true}"#
            )
            .is_err()
        );
    }
}

pub mod http;

fn verify_artifact(path: &std::path::Path, expected: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).context("required interpreter missing")?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        bail!("interpreter integrity mismatch")
    }
    Ok(())
}

fn make_linker(engine: &Engine) -> Result<Linker<State>> {
    let mut linker = Linker::new(engine);
    p1::add_to_linker_sync(&mut linker, |s: &mut State| &mut s.wasi)?;
    // Do not grant blocking waits: epoch interruption cannot cancel host sleeps.
    linker.allow_shadowing(true);
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "poll_oneoff",
        |_: i32, _: i32, _: i32, _: i32| -> Result<i32> {
            Err(wasmtime::Error::new(CapabilityDenied))
        },
    )?;
    Ok(linker)
}
#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn no_preopens_or_path_access_at_wasi_boundary() -> Result<()> {
        let engine = Engine::default();
        let module=Module::new(&engine,wat::parse_str(r#"(module
   (import "wasi_snapshot_preview1" "fd_prestat_get" (func $pre (param i32 i32) (result i32)))
   (import "wasi_snapshot_preview1" "path_open" (func $open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
   (import "wasi_snapshot_preview1" "poll_oneoff" (func $poll (param i32 i32 i32 i32) (result i32)))
   (memory (export "memory") 1) (data (i32.const 64) "/etc/passwd")
   (func (export "pre") (param i32) (result i32) local.get 0 i32.const 0 call $pre)
   (func (export "open") (result i32) i32.const 3 i32.const 0 i32.const 64 i32.const 11 i32.const 0 i64.const 2 i64.const 0 i32.const 0 i32.const 32 call $open)
   (func (export "poll") (result i32) i32.const 0 i32.const 0 i32.const 1 i32.const 0 call $poll)
  )"#).unwrap())?;
        let mut store = Store::new(
            &engine,
            State {
                wasi: WasiCtxBuilder::new().build_p1(),
                limits: StoreLimitsBuilder::new().build(),
            },
        );
        let instance = make_linker(&engine)?.instantiate(&mut store, &module)?;
        let pre = instance.get_typed_func::<i32, i32>(&mut store, "pre")?;
        for fd in 3..64 {
            assert_eq!(pre.call(&mut store, fd)?, 8, "unexpected preopen fd {fd}");
        }
        assert_eq!(
            instance
                .get_typed_func::<(), i32>(&mut store, "open")?
                .call(&mut store, ())?,
            8
        );
        let denied = instance
            .get_typed_func::<(), i32>(&mut store, "poll")?
            .call(&mut store, ())
            .unwrap_err();
        assert!(denied.downcast_ref::<CapabilityDenied>().is_some());
        Ok(())
    }
    #[test]
    fn invalid_utf8_cannot_expand_output_cap() {
        let out = Output::new(1);
        out.bytes.lock().unwrap().push(255);
        assert!(out.text().len() <= 1);
        assert!(out.exceeded.load(Ordering::Relaxed));
    }
    #[test]
    fn artifact_tampering_rejected() {
        let path =
            std::env::temp_dir().join(format!("sandboxrun-integrity-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"tampered").unwrap();
        assert!(
            verify_artifact(
                &path,
                "2c218412cb8b17e46ee91bea7556ec38f5b9b6bd6312fddd3ceeed14aa71eaaa"
            )
            .is_err()
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[derive(Debug)]
struct CapabilityDenied;
impl std::fmt::Display for CapabilityDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "blocking WASI waits are disabled")
    }
}
impl std::error::Error for CapabilityDenied {}
