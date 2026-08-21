#[macro_use]
extern crate derive_new;
extern crate alloc;

mod compute;
mod device;
mod runtime;

pub use device::*;
pub use runtime::*;

#[cfg(feature = "ptx-wmma")]
pub(crate) type WmmaCompiler = cubecl_cpp::cuda::mma::PtxWmmaCompiler;

#[cfg(not(feature = "ptx-wmma"))]
pub(crate) type WmmaCompiler = cubecl_cpp::cuda::mma::CudaWmmaCompiler;

pub mod install {
    use std::path::PathBuf;

    /// Fallible twin of [`include_path`] — returns a diagnostic instead of
    /// panicking when the CUDA toolkit cannot be located.
    ///
    /// Prefer this anywhere the caller can report an error. A missing toolkit is
    /// an ordinary configuration problem, and panicking for it inside a
    /// server/worker thread produces an unwind that no `Result` on the calling
    /// thread can observe — the caller then sails on with a garbage result
    /// (imazen/zenforks-cubecl#4).
    ///
    /// Unlike [`cuda_path`], this also verifies the directory actually contains
    /// `include/cuda_runtime.h`. `cuda_path` accepts any directory that merely
    /// EXISTS, so a driver-only install or a partial copy yields a path whose
    /// headers are absent, and NVRTC then fails later with a far less obvious
    /// message.
    pub fn try_include_path() -> Result<PathBuf, String> {
        let base = try_cuda_path()?;
        let path = base.join("include");
        if !path.join("cuda_runtime.h").exists() {
            return Err(format!(
                "CUDA toolkit at {} has no include/cuda_runtime.h.\n\
                 cubecl compiles kernels with NVRTC, which needs the toolkit \
                 HEADERS: the driver alone (libcuda.so) is not enough, and \
                 neither is LD_LIBRARY_PATH. Point CUDA_PATH at a directory \
                 containing include/ and lib64/.",
                base.display()
            ));
        }
        Ok(path)
    }

    /// Fallible twin of [`cccl_include_path`].
    pub fn try_cccl_include_path() -> Result<PathBuf, String> {
        Ok(try_include_path()?.join("cccl"))
    }

    /// Fallible twin of [`cuda_path`], with an actionable message.
    pub fn try_cuda_path() -> Result<PathBuf, String> {
        cuda_path().ok_or_else(|| {
            "CUDA installation not found. Set CUDA_PATH to a directory that \
             contains include/ and lib64/ (the NVRTC headers live there). \
             Defaults tried: /usr/local/cuda, /opt/cuda, /usr (Linux); \
             C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/ (Windows)."
                .to_string()
        })
    }

    /// # Panics
    /// Panics when the CUDA toolkit cannot be located. Use
    /// [`try_include_path`] instead from any context that can report an error —
    /// notably anything running on a cubecl server/worker thread.
    pub fn include_path() -> PathBuf {
        let mut path = cuda_path().expect("
        CUDA installation not found.
        Please ensure that CUDA is installed and the CUDA_PATH environment variable is set correctly.
        Note: Default paths are used for Linux (/usr/local/cuda) and Windows (C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/), which may not be correct.
    ");
        path.push("include");
        path
    }

    /// # Panics
    /// Panics when the CUDA toolkit cannot be located. See
    /// [`try_cccl_include_path`].
    pub fn cccl_include_path() -> PathBuf {
        let mut path = include_path();
        path.push("cccl");
        path
    }

    pub fn cuda_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CUDA_PATH") {
            return Some(PathBuf::from(path));
        }

        #[cfg(target_os = "linux")]
        {
            // If it is installed as part of the distribution
            return if std::fs::exists("/usr/local/cuda").is_ok_and(|exists| exists) {
                Some(PathBuf::from("/usr/local/cuda"))
            } else if std::fs::exists("/opt/cuda").is_ok_and(|exists| exists) {
                Some(PathBuf::from("/opt/cuda"))
            } else if std::fs::exists("/usr/bin/nvcc").is_ok_and(|exists| exists) {
                // Maybe the compiler was installed within the user path.
                Some(PathBuf::from("/usr"))
            } else {
                None
            };
        }

        #[cfg(target_os = "windows")]
        {
            return Some(PathBuf::from(
                "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/",
            ));
        }

        #[allow(unreachable_code)]
        None
    }
}

#[cfg(test)]
#[allow(unexpected_cfgs)]
mod tests {
    pub type TestRuntime = crate::CudaRuntime;

    pub use half::{bf16, f16};

    cubecl_core::testgen_all!(f32: [f16, bf16, f32, f64], i32: [i8, i16, i32, i64], u32: [u8, u16, u32, u64]);
    cubecl_std::testgen!();
    cubecl_std::testgen_tensor_identity!([f16, bf16, f32, u32]);
    cubecl_std::testgen_quantized_view!(f16);
}
