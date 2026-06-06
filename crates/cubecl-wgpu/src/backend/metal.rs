use cubecl_core::{
    WgpuCompilationOptions,
    ir::{AddressType, UIntKind},
};
use cubecl_cpp::{
    DialectWmmaCompiler,
    metal::{MslDialect, arch::MetalArchitecture},
    shared::register_wmma_features,
};
use cubecl_ir::{
    DeviceProperties, Type,
    features::{AtomicUsage, Plane, TypeUsage},
};
use wgpu::{
    DeviceDescriptor, Features, Limits,
    hal::{self, Adapter, metal},
};

pub async fn request_metal_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    let limits = adapter.limits();
    let features = adapter
        .features()
        .difference(Features::MAPPABLE_PRIMARY_BUFFERS);
    unsafe {
        let hal_adapter = adapter.as_hal::<hal::api::Metal>().unwrap();
        request_device(adapter, &hal_adapter, features, limits)
    }
}

fn request_device(
    wgpu_adapter: &wgpu::Adapter,
    adapter: &metal::Adapter,
    features: Features,
    limits: Limits,
) -> (wgpu::Device, wgpu::Queue) {
    // The default is MemoryHints::Performance, which tries to do some bigger
    // block allocations. However, we already batch allocations, so we
    // can use MemoryHints::MemoryUsage to lower memory usage.
    let memory_hints = wgpu::MemoryHints::MemoryUsage;
    let device = unsafe {
        adapter
            .open(features, &limits, &memory_hints)
            .expect("should create metal HAL device")
    };

    let descriptor = DeviceDescriptor {
        label: None,
        required_features: features,
        required_limits: limits,
        memory_hints,
        trace: wgpu::Trace::Off,
        // SAFETY: Enabling experimental passthrough shaders.
        experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
    };

    unsafe {
        wgpu_adapter
            .create_device_from_hal(device, &descriptor)
            .expect("Failed to create wgpu device")
    }
}

pub fn register_metal_features(
    adapter: &wgpu::Adapter,
    props: &mut DeviceProperties,
    comp_options: &mut WgpuCompilationOptions,
) {
    let features = adapter.features();
    unsafe {
        if let Some(adapter) = adapter.as_hal::<hal::api::Metal>() {
            register_features(&adapter, props, features, comp_options);
        }
    }
}

fn register_features(
    _adapter: &metal::Adapter,
    props: &mut DeviceProperties,
    _features: Features,
    comp_options: &mut WgpuCompilationOptions,
) {
    register_types(props);
    register_cmma(props);
    props.features.alignment = true;
    props.features.plane.insert(Plane::Ops);
    props.features.plane.insert(Plane::Sync);
    // Apple/Metal GPUs have no 64-bit float. By default a kernel using f64 now
    // fails to compile with a clear error (strict); the lossy f64->f32 downgrade
    // only happens when pre-authorized (CUBECL_ALLOW_F64_DOWNGRADE).
    comp_options.supports_f64 = false;
    comp_options.allow_f64_downgrade = super::wgsl::f64_downgrade_preauthorized();
}

fn register_types(props: &mut DeviceProperties) {
    use cubecl_core::ir::{ElemType, FloatKind, IntKind, StorageType};

    props.register_address_type(AddressType::U32);
    props.register_address_type(AddressType::U64);

    let types = [
        ElemType::UInt(UIntKind::U8),
        ElemType::UInt(UIntKind::U16),
        ElemType::UInt(UIntKind::U32),
        ElemType::UInt(UIntKind::U64),
        ElemType::Int(IntKind::I8),
        ElemType::Int(IntKind::I16),
        ElemType::Int(IntKind::I32),
        ElemType::Int(IntKind::I64),
        ElemType::Float(FloatKind::F16),
        ElemType::Float(FloatKind::F32),
        ElemType::Bool,
    ];

    // f32 is intentionally NOT in this list — see comment below for why
    // it ships separately with restricted usage. Integer atomics here
    // all natively support both Add and LoadStore on Metal 3.
    let atomic_types = [
        ElemType::Int(IntKind::I32),
        ElemType::UInt(UIntKind::U32),
        ElemType::UInt(UIntKind::U64),
    ];

    for ty in types {
        props.register_type_usage(ty, TypeUsage::all());
    }

    for ty in atomic_types {
        props.register_atomic_type_usage(
            Type::new(StorageType::Atomic(ty)),
            AtomicUsage::Add | AtomicUsage::LoadStore,
        )
    }

    // f32 atomic is supported only with LoadStore on Metal. AtomicUsage::Add
    // is intentionally omitted because naga's MSL backend doesn't emit
    // `atomic_fetch_add_explicit` for f32 even when the underlying Metal 3
    // device supports it: cubecl-wgpu's WGSL codegen would emit
    // `atomicAdd<f32>(...)` which naga drops silently in the MSL output,
    // causing every reduction to return its default value (~0.0) without
    // a runtime error.
    //
    // Declaring AtomicUsage::Add here would make callers happily emit
    // `Atomic<f32>::fetch_add` and ship silently-broken scores. By
    // omitting Add, callers like the cubecl-runtime construct path catch
    // the missing capability at construction time and surface
    // `unsupported atomic operation on this backend` — loud and actionable.
    //
    // Downstream (zenmetrics) has matching per-metric audits that flip
    // `fast-reduction` default to OFF on Metal-targeted builds for
    // butteraugli-gpu / dssim-gpu, and a Metal-reject path in cvvdp-gpu.
    // See zenmetrics' crates/zenmetrics-api/docs/CUBECL_METAL_ATOMIC_FIX.md
    // for the full audit.
    //
    // A future improvement: emit a u32-bitcast CAS loop in the WGSL
    // codegen for Atomic<f32>::fetch_add to get correctness on every
    // backend, then opt into native atomic_fetch_add_explicit when
    // naga grows MSL-backend support for it. That work is tracked
    // separately.
    props.register_atomic_type_usage(
        Type::new(StorageType::Atomic(ElemType::Float(FloatKind::F32))),
        AtomicUsage::LoadStore,
    );
}

fn register_cmma(props: &mut DeviceProperties) {
    let combinations = MslDialect::supported_wmma_combinations(&MetalArchitecture::Metal3);
    register_wmma_features(combinations, props);
}
