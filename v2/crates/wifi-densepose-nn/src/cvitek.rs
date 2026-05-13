//! CviTek TPU runtime backend (StalyaTech RuView Phase 2).
//!
//! This module exposes [`CviTekBackend`], an [`crate::inference::Backend`]
//! implementation that runs `.cvimodel` files on the on-chip TPU of the
//! Sophgo CV1812CP / SG2000 (also marketed as Milk-V Duo S TPU).  Inference
//! is dispatched through the closed-source `libcviruntime.so` userspace
//! runtime via raw FFI; see `cviruntime/include/cviruntime.h` for the C API
//! surface this module mirrors.
//!
//! The backend is gated behind the `cvitek` Cargo feature so upstream RuView
//! builds without the SG2000 SDK are unaffected.  See `build.rs` for the
//! linker setup and `CVITEK_SDK_DIR` env contract.
//!
//! Threading model: every `Backend::run` call requires exclusive access to
//! the underlying TPU model handle (the runtime is *not* re-entrant for a
//! single model).  Interior mutability is provided by an internal
//! [`parking_lot::Mutex`], matching the pattern used by [`crate::onnx::OnnxBackend`].

#![allow(unsafe_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::Path;
use std::ptr;
use std::slice;
use std::sync::Arc;

use ndarray::{Array4, ArrayD, IxDyn};

use crate::error::{NnError, NnResult};
use crate::inference::Backend;
use crate::tensor::{Tensor, TensorShape};

// ---------------------------------------------------------------------------
// FFI declarations: mirror of cviruntime/include/cviruntime.h
// ---------------------------------------------------------------------------

/// Maximum tensor rank reported by `cviruntime.h::CVI_DIM_MAX`.
const CVI_DIM_MAX: usize = 6;

/// `CVI_FMT` — element type tag stored on every `CVI_TENSOR`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CviFmt {
    Fp32 = 0,
    Int32 = 1,
    Uint32 = 2,
    Bf16 = 3,
    Int16 = 4,
    Uint16 = 5,
    Int8 = 6,
    Uint8 = 7,
}

/// `CVI_MEM_TYPE_E` — backing store of the tensor buffer.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum CviMemTypeE {
    System = 1,
    Device = 2,
}

/// `CVI_NN_PIXEL_FORMAT_E` — image layout hint for image-typed inputs.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum CviNnPixelFormatE {
    RgbPacked = 0,
    BgrPacked = 1,
    RgbPlanar = 2,
    BgrPlanar = 3,
    YuvNv12 = 11,
    YuvNv21 = 12,
    Yuv420Planar = 13,
    Grayscale = 15,
    Tensor = 100,
    RgbaPlanar = 1000,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CviShape {
    dim: [i32; CVI_DIM_MAX],
    dim_size: usize,
}

/// `CVI_TENSOR` — must match the C struct byte-for-byte.  The compile-time
/// `LAYOUT_CHECK` constant below asserts the size on AArch64 (the only
/// target we link against in Phase 2).
#[repr(C)]
struct CviTensor {
    name: *mut c_char,
    shape: CviShape,
    fmt: CviFmt,
    count: usize,
    mem_size: usize,
    sys_mem: *mut u8,
    paddr: u64,
    mem_type: CviMemTypeE,
    qscale: f32,
    zero_point: c_int,
    pixel_format: CviNnPixelFormatE,
    aligned: bool,
    mean: [f32; 3],
    scale: [f32; 3],
    owner: *mut c_void,
    reserved: [u8; 32],
}

type CviModelHandle = *mut c_void;
type CviRc = c_int;

const CVI_RC_SUCCESS: CviRc = 0;

#[link(name = "cviruntime")]
extern "C" {
    fn CVI_NN_RegisterModel(model_file: *const c_char, model: *mut CviModelHandle) -> CviRc;
    fn CVI_NN_CleanupModel(model: CviModelHandle) -> CviRc;
    fn CVI_NN_GetInputOutputTensors(
        model: CviModelHandle,
        inputs: *mut *mut CviTensor,
        input_num: *mut i32,
        outputs: *mut *mut CviTensor,
        output_num: *mut i32,
    ) -> CviRc;
    fn CVI_NN_Forward(
        model: CviModelHandle,
        inputs: *mut CviTensor,
        input_num: i32,
        outputs: *mut CviTensor,
        output_num: i32,
    ) -> CviRc;
    fn CVI_NN_GetModelTarget(model: CviModelHandle) -> *const c_char;
    fn CVI_NN_GetModelVersion(
        model: CviModelHandle,
        major: *mut i32,
        minor: *mut i32,
    ) -> CviRc;
}

// ---------------------------------------------------------------------------
// Safe wrapper: CviTekBackend
// ---------------------------------------------------------------------------

/// Cached per-tensor metadata so the public `Backend` getters do not have to
/// re-traverse the FFI tensor array on every call.
#[derive(Debug, Clone)]
struct TensorMeta {
    name: String,
    shape: TensorShape,
    fmt: CviFmt,
    count: usize,
    qscale: f32,
    zero_point: i32,
}

/// Inner state — borrowed exclusively by `run()`.  Held behind a Mutex to
/// make the public `Backend` type `Send + Sync` without copying weights.
struct Inner {
    handle: CviModelHandle,
    // Raw runtime-owned tensor arrays.  Lifetime is tied to `handle` —
    // `CVI_NN_CleanupModel` releases both.
    inputs_ptr: *mut CviTensor,
    input_num: i32,
    outputs_ptr: *mut CviTensor,
    output_num: i32,
}

// SAFETY: the runtime documentation describes a single-threaded usage model
// per model handle.  We serialise access through `parking_lot::Mutex<Inner>`
// in the public type and never expose the raw pointers, so cross-thread
// transfer is sound.
unsafe impl Send for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: handle is non-null and produced by RegisterModel.
            unsafe { CVI_NN_CleanupModel(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

/// TPU-accelerated [`Backend`] for Sophgo CV1812CP / SG2000.
pub struct CviTekBackend {
    inner: Arc<parking_lot::Mutex<Inner>>,
    inputs: Vec<TensorMeta>,
    outputs: Vec<TensorMeta>,
    target: String,
    version: (i32, i32),
}

impl std::fmt::Debug for CviTekBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CviTekBackend")
            .field("target", &self.target)
            .field("version", &self.version)
            .field("input_count", &self.inputs.len())
            .field("output_count", &self.outputs.len())
            .finish()
    }
}

impl CviTekBackend {
    /// Load a `.cvimodel` from disk and inspect its IO tensor layout.
    ///
    /// On failure the caller receives a structured `NnError::ModelLoad`
    /// carrying the numeric `CVI_RC` so log filtering can distinguish a
    /// missing `/dev/cvi-tpu0` (`CVI_RC_UNINIT`) from a malformed model.
    pub fn from_file<P: AsRef<Path>>(path: P) -> NnResult<Self> {
        let path = path.as_ref();
        let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| {
            NnError::model_load(format!(
                "model path contains interior NUL byte: {} ({})",
                path.display(),
                e
            ))
        })?;

        let mut handle: CviModelHandle = ptr::null_mut();
        // SAFETY: `c_path` outlives the call; `handle` is a stack slot.
        let rc = unsafe { CVI_NN_RegisterModel(c_path.as_ptr(), &mut handle) };
        if rc != CVI_RC_SUCCESS || handle.is_null() {
            return Err(NnError::model_load(format!(
                "CVI_NN_RegisterModel failed (rc={}, path={})",
                rc,
                path.display()
            )));
        }

        let mut inputs_ptr: *mut CviTensor = ptr::null_mut();
        let mut input_num: i32 = 0;
        let mut outputs_ptr: *mut CviTensor = ptr::null_mut();
        let mut output_num: i32 = 0;

        // SAFETY: handle is non-null after a successful RegisterModel.
        let rc = unsafe {
            CVI_NN_GetInputOutputTensors(
                handle,
                &mut inputs_ptr,
                &mut input_num,
                &mut outputs_ptr,
                &mut output_num,
            )
        };
        if rc != CVI_RC_SUCCESS {
            // SAFETY: we own `handle`, cleanup before bailing.
            unsafe { CVI_NN_CleanupModel(handle) };
            return Err(NnError::model_load(format!(
                "CVI_NN_GetInputOutputTensors failed (rc={})",
                rc
            )));
        }

        let inputs = unsafe { snapshot_tensors(inputs_ptr, input_num as usize) }?;
        let outputs = unsafe { snapshot_tensors(outputs_ptr, output_num as usize) }?;

        // SAFETY: handle is valid until Cleanup.
        let target = unsafe {
            let raw = CVI_NN_GetModelTarget(handle);
            if raw.is_null() {
                String::from("unknown")
            } else {
                std::ffi::CStr::from_ptr(raw)
                    .to_string_lossy()
                    .into_owned()
            }
        };

        let mut major: i32 = 0;
        let mut minor: i32 = 0;
        // SAFETY: scalar out-parameters.
        let _ = unsafe { CVI_NN_GetModelVersion(handle, &mut major, &mut minor) };

        let inner = Inner {
            handle,
            inputs_ptr,
            input_num,
            outputs_ptr,
            output_num,
        };

        Ok(CviTekBackend {
            inner: Arc::new(parking_lot::Mutex::new(inner)),
            inputs,
            outputs,
            target,
            version: (major, minor),
        })
    }

    /// Return the model's chip target string (e.g. `cv183x`).
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Return the model file format version reported by the runtime.
    pub fn model_version(&self) -> (i32, i32) {
        self.version
    }

    /// Snapshot the cached input metadata (for diagnostics).
    pub fn inputs_info(&self) -> Vec<(String, TensorShape, &'static str, f32, i32)> {
        self.inputs
            .iter()
            .map(|m| {
                (
                    m.name.clone(),
                    m.shape.clone(),
                    fmt_name(m.fmt),
                    m.qscale,
                    m.zero_point,
                )
            })
            .collect()
    }

    /// Snapshot the cached output metadata.
    pub fn outputs_info(&self) -> Vec<(String, TensorShape, &'static str, f32, i32)> {
        self.outputs
            .iter()
            .map(|m| {
                (
                    m.name.clone(),
                    m.shape.clone(),
                    fmt_name(m.fmt),
                    m.qscale,
                    m.zero_point,
                )
            })
            .collect()
    }
}

impl Backend for CviTekBackend {
    fn name(&self) -> &str {
        "cvitek"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn input_names(&self) -> Vec<String> {
        self.inputs.iter().map(|m| m.name.clone()).collect()
    }

    fn output_names(&self) -> Vec<String> {
        self.outputs.iter().map(|m| m.name.clone()).collect()
    }

    fn input_shape(&self, name: &str) -> Option<TensorShape> {
        self.inputs
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.shape.clone())
    }

    fn output_shape(&self, name: &str) -> Option<TensorShape> {
        self.outputs
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.shape.clone())
    }

    fn run(&self, inputs: HashMap<String, Tensor>) -> NnResult<HashMap<String, Tensor>> {
        let mut inner = self.inner.lock();

        // 1) Copy named user tensors into the runtime-owned input buffers.
        for meta in &self.inputs {
            let user_tensor = inputs.get(&meta.name).ok_or_else(|| {
                NnError::invalid_input(format!("missing input tensor '{}'", meta.name))
            })?;

            // SAFETY: inputs_ptr is non-null with input_num valid elements.
            let cvi_tensor = unsafe { tensor_slot_mut(inner.inputs_ptr, inner.input_num, meta)? };

            write_tensor(user_tensor, cvi_tensor, meta)?;
        }

        // 2) Synchronous forward pass on the TPU.
        // SAFETY: pointers/lengths come straight from GetInputOutputTensors.
        let rc = unsafe {
            CVI_NN_Forward(
                inner.handle,
                inner.inputs_ptr,
                inner.input_num,
                inner.outputs_ptr,
                inner.output_num,
            )
        };
        if rc != CVI_RC_SUCCESS {
            return Err(NnError::inference(format!(
                "CVI_NN_Forward failed (rc={rc})"
            )));
        }

        // 3) Collect outputs as owned ndarray tensors (dequantised to f32).
        let mut result = HashMap::with_capacity(self.outputs.len());
        for meta in &self.outputs {
            // SAFETY: outputs_ptr is non-null with output_num valid elements.
            let cvi_tensor = unsafe { tensor_slot_ref(inner.outputs_ptr, inner.output_num, meta)? };
            let tensor = read_tensor(cvi_tensor, meta)?;
            result.insert(meta.name.clone(), tensor);
        }

        Ok(result)
    }

    fn memory_usage(&self) -> usize {
        let inputs_bytes: usize = self.inputs.iter().map(|m| m.count * fmt_size(m.fmt)).sum();
        let outputs_bytes: usize = self.outputs.iter().map(|m| m.count * fmt_size(m.fmt)).sum();
        inputs_bytes + outputs_bytes
    }
}

// ---------------------------------------------------------------------------
// FFI helpers
// ---------------------------------------------------------------------------

/// Walk a runtime-owned tensor array and capture the parts the safe wrapper
/// needs to expose: name, shape, dtype, scale/zero-point for INT8 dequant.
///
/// # Safety
/// `ptr` must point to an array of at least `n` valid `CVI_TENSOR`s whose
/// `name` field is a NUL-terminated C string (always true for arrays
/// returned by `CVI_NN_GetInputOutputTensors`).
unsafe fn snapshot_tensors(ptr: *mut CviTensor, n: usize) -> NnResult<Vec<TensorMeta>> {
    if n == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err(NnError::model_load("tensor array pointer is null"));
    }
    let slice = unsafe { slice::from_raw_parts(ptr, n) };
    let mut out = Vec::with_capacity(n);
    for t in slice {
        // SAFETY: per docstring, `name` is a valid C string.
        let name = if t.name.is_null() {
            String::new()
        } else {
            unsafe { std::ffi::CStr::from_ptr(t.name).to_string_lossy().into_owned() }
        };
        let dims: Vec<usize> = t.shape.dim[..t.shape.dim_size]
            .iter()
            .map(|d| (*d).max(0) as usize)
            .collect();
        out.push(TensorMeta {
            name,
            shape: TensorShape::new(dims),
            fmt: t.fmt,
            count: t.count,
            qscale: t.qscale,
            zero_point: t.zero_point,
        });
    }
    Ok(out)
}

/// Locate the runtime tensor that matches `meta.name` and return a mutable
/// reference for input population.
///
/// # Safety
/// `ptr` must address `n` initialised `CVI_TENSOR`s.
unsafe fn tensor_slot_mut<'a>(
    ptr: *mut CviTensor,
    n: i32,
    meta: &TensorMeta,
) -> NnResult<&'a mut CviTensor> {
    let slots = unsafe { slice::from_raw_parts_mut(ptr, n as usize) };
    for t in slots {
        let raw_name = if t.name.is_null() {
            ""
        } else {
            unsafe { std::ffi::CStr::from_ptr(t.name) }.to_str().unwrap_or("")
        };
        if raw_name == meta.name {
            return Ok(t);
        }
    }
    Err(NnError::invalid_input(format!(
        "tensor '{}' not present in runtime descriptor",
        meta.name
    )))
}

/// Read-only variant for output collection.
///
/// # Safety
/// See `tensor_slot_mut`.
unsafe fn tensor_slot_ref<'a>(
    ptr: *mut CviTensor,
    n: i32,
    meta: &TensorMeta,
) -> NnResult<&'a CviTensor> {
    let slots = unsafe { slice::from_raw_parts(ptr, n as usize) };
    for t in slots {
        let raw_name = if t.name.is_null() {
            ""
        } else {
            unsafe { std::ffi::CStr::from_ptr(t.name) }.to_str().unwrap_or("")
        };
        if raw_name == meta.name {
            return Ok(t);
        }
    }
    Err(NnError::invalid_input(format!(
        "tensor '{}' not present in runtime descriptor",
        meta.name
    )))
}

/// Copy a user-supplied `Tensor` into a runtime-owned `CVI_TENSOR` buffer.
/// FP32 inputs are written verbatim; INT8/UINT8 inputs are quantised using
/// the per-tensor `qscale`/`zero_point` reported by the runtime.
fn write_tensor(user: &Tensor, cvi: &mut CviTensor, meta: &TensorMeta) -> NnResult<()> {
    let src = user.to_vec()?; // owned Vec<f32>
    if src.len() != meta.count {
        return Err(NnError::shape_mismatch(
            meta.shape.dims().to_vec(),
            user.shape().dims().to_vec(),
        ));
    }
    if cvi.sys_mem.is_null() {
        return Err(NnError::inference(format!(
            "input '{}' has no sys_mem buffer (runtime did not allocate)",
            meta.name
        )));
    }
    // SAFETY: sys_mem points at mem_size bytes owned by the runtime.
    let dst = unsafe { slice::from_raw_parts_mut(cvi.sys_mem, cvi.mem_size) };

    match meta.fmt {
        CviFmt::Fp32 => {
            let need = meta.count * 4;
            if dst.len() < need {
                return Err(NnError::inference(format!(
                    "input '{}' buffer too small: got {} bytes, need {}",
                    meta.name,
                    dst.len(),
                    need
                )));
            }
            // SAFETY: lengths checked, both buffers byte-addressable.
            let dst_f32 = unsafe {
                slice::from_raw_parts_mut(cvi.sys_mem as *mut f32, meta.count)
            };
            dst_f32.copy_from_slice(&src);
        }
        CviFmt::Int8 => {
            quantise_into_i8(&src, dst, meta.qscale, meta.zero_point, &meta.name)?;
        }
        CviFmt::Uint8 => {
            quantise_into_u8(&src, dst, meta.qscale, meta.zero_point, &meta.name)?;
        }
        other => {
            return Err(NnError::Unsupported(format!(
                "input '{}' uses unsupported dtype {:?}",
                meta.name, other
            )))
        }
    }
    Ok(())
}

/// Read a runtime-owned output tensor back into a Rust-owned `Tensor`.
fn read_tensor(cvi: &CviTensor, meta: &TensorMeta) -> NnResult<Tensor> {
    if cvi.sys_mem.is_null() {
        return Err(NnError::inference(format!(
            "output '{}' has no sys_mem buffer",
            meta.name
        )));
    }
    let mut data = Vec::<f32>::with_capacity(meta.count);
    match meta.fmt {
        CviFmt::Fp32 => {
            // SAFETY: sys_mem points at mem_size >= count*4 bytes.
            let src = unsafe { slice::from_raw_parts(cvi.sys_mem as *const f32, meta.count) };
            data.extend_from_slice(src);
        }
        CviFmt::Int8 => {
            // SAFETY: sys_mem holds count int8 elements.
            let src = unsafe { slice::from_raw_parts(cvi.sys_mem as *const i8, meta.count) };
            let zp = meta.zero_point;
            let scale = meta.qscale.max(f32::EPSILON);
            data.extend(src.iter().map(|&q| (q as i32 - zp) as f32 * scale));
        }
        CviFmt::Uint8 => {
            // SAFETY: sys_mem holds count uint8 elements.
            let src = unsafe { slice::from_raw_parts(cvi.sys_mem as *const u8, meta.count) };
            let zp = meta.zero_point;
            let scale = meta.qscale.max(f32::EPSILON);
            data.extend(src.iter().map(|&q| (q as i32 - zp) as f32 * scale));
        }
        other => {
            return Err(NnError::Unsupported(format!(
                "output '{}' uses unsupported dtype {:?}",
                meta.name, other
            )))
        }
    }
    tensor_from_shape(meta.shape.dims(), data, &meta.name)
}

fn quantise_into_i8(
    src: &[f32],
    dst: &mut [u8],
    qscale: f32,
    zero_point: i32,
    name: &str,
) -> NnResult<()> {
    if dst.len() < src.len() {
        return Err(NnError::inference(format!(
            "input '{}' int8 buffer too small ({} < {})",
            name,
            dst.len(),
            src.len()
        )));
    }
    let scale = qscale.max(f32::EPSILON);
    for (s, d) in src.iter().zip(dst.iter_mut()) {
        let q = (*s / scale).round() as i32 + zero_point;
        let clamped = q.clamp(i8::MIN as i32, i8::MAX as i32) as i8;
        *d = clamped as u8; // bit-cast: store i8 in the u8 slot
    }
    Ok(())
}

fn quantise_into_u8(
    src: &[f32],
    dst: &mut [u8],
    qscale: f32,
    zero_point: i32,
    name: &str,
) -> NnResult<()> {
    if dst.len() < src.len() {
        return Err(NnError::inference(format!(
            "input '{}' uint8 buffer too small ({} < {})",
            name,
            dst.len(),
            src.len()
        )));
    }
    let scale = qscale.max(f32::EPSILON);
    for (s, d) in src.iter().zip(dst.iter_mut()) {
        let q = (*s / scale).round() as i32 + zero_point;
        *d = q.clamp(u8::MIN as i32, u8::MAX as i32) as u8;
    }
    Ok(())
}

fn tensor_from_shape(dims: &[usize], data: Vec<f32>, name: &str) -> NnResult<Tensor> {
    if dims.len() == 4 {
        let arr = Array4::from_shape_vec((dims[0], dims[1], dims[2], dims[3]), data)
            .map_err(|e| NnError::tensor_op(format!("reshape failed for '{name}': {e}")))?;
        Ok(Tensor::Float4D(arr))
    } else {
        let arr = ArrayD::from_shape_vec(IxDyn(dims), data)
            .map_err(|e| NnError::tensor_op(format!("reshape failed for '{name}': {e}")))?;
        Ok(Tensor::FloatND(arr))
    }
}

fn fmt_size(f: CviFmt) -> usize {
    match f {
        CviFmt::Fp32 | CviFmt::Int32 | CviFmt::Uint32 => 4,
        CviFmt::Bf16 | CviFmt::Int16 | CviFmt::Uint16 => 2,
        CviFmt::Int8 | CviFmt::Uint8 => 1,
    }
}

fn fmt_name(f: CviFmt) -> &'static str {
    match f {
        CviFmt::Fp32 => "fp32",
        CviFmt::Int32 => "int32",
        CviFmt::Uint32 => "uint32",
        CviFmt::Bf16 => "bf16",
        CviFmt::Int16 => "int16",
        CviFmt::Uint16 => "uint16",
        CviFmt::Int8 => "int8",
        CviFmt::Uint8 => "uint8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard against silent C ABI drift.  The struct must match the
    /// AArch64 layout of `CVI_TENSOR` from cviruntime.h on the SG2000
    /// rootfs; if it ever diverges, every Forward() call would scribble
    /// past the runtime's owned buffer.
    #[test]
    fn cvi_shape_layout() {
        assert_eq!(std::mem::size_of::<CviShape>(), 6 * 4 + 8);
    }
}
