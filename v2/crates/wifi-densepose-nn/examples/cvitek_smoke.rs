//! CviTek TPU smoke test — StalyaTech RuView Phase 2.
//!
//! Loads a `.cvimodel` file, prints the runtime-reported IO descriptors,
//! pushes a zero-filled input tensor through `Backend::run`, and reports
//! output statistics + inference latency.  No accuracy assertions: this
//! example exists to validate that the FFI binding, linker setup, and
//! `/dev/cvi-tpu0` device path are working end-to-end on the SG2000.
//!
//! Usage on the SG2000:
//!
//! ```text
//! cvitek_smoke /opt/ruview/models/yolov8n_pose_384_640.cvimodel
//! ```
//!
//! Build only when the `cvitek` feature is enabled (otherwise the
//! `cvitek` module is gated out and this example will not compile).

#[cfg(feature = "cvitek")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::time::Instant;
    use wifi_densepose_nn::{Backend, CviTekBackend, Tensor};

    // tracing-subscriber not in this crate's deps; keep output plain.
    let model_path = std::env::args().nth(1).ok_or_else(|| {
        "usage: cvitek_smoke <path/to/model.cvimodel>".to_string()
    })?;

    println!("==> Loading {model_path}");
    let t0 = Instant::now();
    let backend = CviTekBackend::from_file(&model_path)?;
    let load_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("    loaded in {load_ms:.1} ms");
    println!("    target  : {}", backend.target());
    let (major, minor) = backend.model_version();
    println!("    version : {major}.{minor}");
    println!();

    println!("==> Inputs ({}):", backend.input_names().len());
    for (name, shape, dtype, qscale, zp) in backend.inputs_info() {
        println!(
            "    - {name}  shape={:?}  dtype={dtype}  qscale={qscale}  zp={zp}",
            shape.dims()
        );
    }
    println!();

    println!("==> Outputs ({}):", backend.output_names().len());
    for (name, shape, dtype, qscale, zp) in backend.outputs_info() {
        println!(
            "    - {name}  shape={:?}  dtype={dtype}  qscale={qscale}  zp={zp}",
            shape.dims()
        );
    }
    println!();

    // Build zero-filled inputs matching every reported input shape.
    let mut inputs = HashMap::new();
    for name in backend.input_names() {
        let shape = backend
            .input_shape(&name)
            .ok_or_else(|| format!("missing shape metadata for input '{name}'"))?;
        let dims = shape.dims();
        let tensor = if dims.len() == 4 {
            Tensor::zeros_4d([dims[0], dims[1], dims[2], dims[3]])
        } else {
            return Err(format!(
                "smoke test currently only supports 4D inputs (got {} for '{name}')",
                dims.len()
            )
            .into());
        };
        inputs.insert(name, tensor);
    }

    println!("==> Forward pass (zero input) …");
    let t1 = Instant::now();
    let outputs = backend.run(inputs)?;
    let infer_ms = t1.elapsed().as_secs_f64() * 1000.0;
    println!("    forward in {infer_ms:.2} ms");
    println!();

    println!("==> Output summary:");
    for (name, tensor) in outputs.iter() {
        let dims = tensor.shape().dims().to_vec();
        let data = tensor.to_vec()?;
        let n = data.len();
        let (mut mn, mut mx, mut sum, mut nonzero) = (f32::INFINITY, f32::NEG_INFINITY, 0.0_f64, 0usize);
        for &v in &data {
            if v < mn {
                mn = v;
            }
            if v > mx {
                mx = v;
            }
            sum += v as f64;
            if v != 0.0 {
                nonzero += 1;
            }
        }
        let mean = if n > 0 { sum / n as f64 } else { 0.0 };
        println!(
            "    - {name}  shape={dims:?}  count={n}  min={mn:.4}  max={mx:.4}  mean={mean:.4}  nonzero={nonzero}"
        );
    }

    println!();
    println!("OK — TPU FFI roundtrip succeeded.");
    println!("Phase 2.0 acceptance: load + forward + dequantise wired.");
    Ok(())
}

#[cfg(not(feature = "cvitek"))]
fn main() {
    eprintln!(
        "This example requires `--features cvitek`.\n\
         Try: cargo run -p wifi-densepose-nn --example cvitek_smoke --features cvitek -- <model.cvimodel>"
    );
    std::process::exit(2);
}
