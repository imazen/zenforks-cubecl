// We cannot put this struct in cubecl-wgpu crate due to circular dependencies.
#[derive(Clone, Copy, Debug, Default)]
pub struct WgpuCompilationOptions {
    pub supports_u64: bool,
    /// Whether the device supports 64-bit floats (`SHADER_F64`). Metal/Apple GPUs
    /// do not. When false and a kernel uses `f64`, behavior depends on
    /// [`Self::allow_f64_downgrade`]: by default (strict) compilation ERRORS;
    /// when pre-authorized, the WGSL compiler downgrades `f64`→`f32` (registers,
    /// locals, and buffer element types) so the kernel still validates and runs.
    pub supports_f64: bool,
    /// Pre-authorization for the lossy `f64`→`f32` downgrade on devices without
    /// `SHADER_F64`. Default `false` = **strict**: a kernel using `f64` on such a
    /// device fails to compile with a clear error rather than silently losing
    /// precision (or, unpatched, being rejected by the driver and producing
    /// uninitialized garbage). Set it (e.g. env `CUBECL_ALLOW_F64_DOWNGRADE=1`)
    /// only when f32 precision is acceptable for every f64 in the kernel.
    pub allow_f64_downgrade: bool,
    /// Whether the Vulkan compiler is supported or we need to fall back to WGSL
    pub supports_vulkan: bool,

    pub vulkan: VulkanCompilationOptions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VulkanCompilationOptions {
    pub supports_fp_fast_math: bool,
    pub supports_explicit_smem: bool,
    pub supports_arbitrary_bitwise: bool,
    pub supports_uniform_standard_layout: bool,
    pub supports_uniform_unsized_array: bool,

    pub max_spirv_version: (u8, u8),
}
