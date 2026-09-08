use core::hash::{BuildHasher, Hash, Hasher};

use crate::{
    AddressType, SemanticType, StorageType, Type, TypeHash, VectorSize,
    features::{AtomicUsage, Features, TypeUsage},
};
use cubecl_common::profile::TimingMethod;
use enumset::EnumSet;

/// Properties of the device related to the accelerator hardware.
///
/// # Plane size min/max
///
/// This is a range of possible values for the plane size.
///
/// For Nvidia GPUs and HIP, this is a single fixed value.
///
/// For wgpu with AMD GPUs this is a range of possible values, but the actual configured value
/// is undefined and can only be queried at runtime. Should usually be 32, but not guaranteed.
///
/// For Intel GPUs, this is variable based on the number of registers used in the kernel. No way to
/// query this at compile time is currently available. As a result, the minimum value should usually
/// be assumed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HardwareProperties {
    /// The maximum size of a single load instruction, in bits. Used for optimized vector sizes.
    pub load_width: u32,
    /// The minimum size of a plane on this device
    pub plane_size_min: u32,
    /// The maximum size of a plane on this device
    pub plane_size_max: u32,
    /// minimum number of bindings for a kernel that can be used at once.
    pub max_bindings: u32,
    /// Maximum amount of shared memory, in bytes
    pub max_shared_memory_size: usize,
    /// Maximum `CubeCount` in x, y and z dimensions
    pub max_cube_count: (u32, u32, u32),
    /// Maximum number of total units in a cube
    pub max_units_per_cube: u32,
    /// Maximum `CubeDim` in x, y, and z dimensions
    pub max_cube_dim: (u32, u32, u32),
    /// Number of streaming multiprocessors (SM), if available
    pub num_streaming_multiprocessors: Option<u32>,
    /// Number of available parallel cpu units, if the runtime is CPU.
    pub num_cpu_cores: Option<u32>,
    /// Number of tensor cores per SM, if any
    pub num_tensor_cores: Option<u32>,
    /// The minimum tiling dimension for a single axis in tensor cores.
    ///
    /// For a backend that only supports 16x16x16, the value would be 16.
    /// For a backend that also supports 32x8x16, the value would be 8.
    pub min_tensor_cores_dim: Option<u32>,
    /// Maximum vector size supported by the device
    pub max_vector_size: VectorSize,
}

/// Properties of the device related to allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryDeviceProperties {
    /// The maximum nr. of bytes that can be allocated in one go.
    pub max_page_size: u64,
    /// The required memory offset alignment in bytes.
    pub alignment: u64,
    /// Total memory the device reports, in bytes, or `None` when this backend
    /// has no way to ask.
    ///
    /// This is the device's *capacity*, not its free space and not an
    /// allocation limit — [`max_page_size`](Self::max_page_size) is the
    /// per-allocation ceiling and is typically a fraction of this. It is
    /// reported so a caller can decide **before** it builds anything whether a
    /// working set is plausible on this device at all, and pick a different
    /// implementation (a CPU path, a streaming/strip GPU path) when it is not.
    /// Answering that question by attempting the allocation and handling the
    /// failure is strictly worse: it pays the setup cost first, and on some
    /// backends the failure arrives on a thread the caller cannot observe.
    ///
    /// `None` means *unknown*, and callers must treat it as such — never as
    /// "unlimited" and never as a number worth guessing. A backend that cannot
    /// query its device says so rather than reporting a fabricated capacity.
    pub total_memory: Option<u64>,
}

/// Whether a prospective allocation is possible on a device, from
/// [`MemoryDeviceProperties::can_allocate`].
///
/// The three "no" answers are kept apart because they call for different
/// responses: a request past the per-allocation ceiling may still work if it is
/// split, one past the device's capacity will not work however it is split, and
/// an unknown capacity is a reason to be careful rather than a reason to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationVerdict {
    /// Within both the per-allocation ceiling and the device's capacity.
    Fits,
    /// Larger than [`MemoryDeviceProperties::max_page_size`]. A single
    /// allocation this size cannot be bound, but the work may still fit if it
    /// is split into several smaller ones.
    ExceedsMaxPageSize {
        /// The per-allocation ceiling that was exceeded, in bytes.
        max_page_size: u64,
    },
    /// Larger than the device's total memory. Splitting will not help; this
    /// device cannot hold the request at all.
    ExceedsDeviceMemory {
        /// The device's total memory, in bytes.
        total_memory: u64,
    },
    /// Within the per-allocation ceiling, but the device's capacity is unknown,
    /// so whether it actually fits could not be decided here.
    Unknown,
}

impl AllocationVerdict {
    /// Whether this verdict is [`Fits`](AllocationVerdict::Fits).
    ///
    /// Deliberately false for [`Unknown`](AllocationVerdict::Unknown): an
    /// undecidable answer is not a positive one, so a caller that treats this
    /// as a go/no-go gate errs toward caution.
    pub fn fits(&self) -> bool {
        matches!(self, Self::Fits)
    }
}

impl MemoryDeviceProperties {
    /// Whether an allocation of `bytes` is possible on this device.
    ///
    /// Checks the per-allocation ceiling first, then the device's capacity, so
    /// the returned verdict names the *first* limit the request runs into.
    /// This is a static check against what the device reports — it says nothing
    /// about how much is free right now, and a `Fits` answer can still fail
    /// against a device another process is using.
    pub fn can_allocate(&self, bytes: u64) -> AllocationVerdict {
        if bytes > self.max_page_size {
            return AllocationVerdict::ExceedsMaxPageSize {
                max_page_size: self.max_page_size,
            };
        }
        match self.total_memory {
            Some(total) if bytes > total => AllocationVerdict::ExceedsDeviceMemory {
                total_memory: total,
            },
            Some(_) => AllocationVerdict::Fits,
            None => AllocationVerdict::Unknown,
        }
    }
}

/// Properties of what the device can do, like what `Feature` are
/// supported by it and what its memory properties are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProperties {
    /// The features supported by the runtime.
    pub features: Features,
    /// The memory properties of this client.
    pub memory: MemoryDeviceProperties,
    /// The topology properties of this client.
    pub hardware: HardwareProperties,
    /// The method used for profiling on the device.
    pub timing_method: TimingMethod,
}

impl TypeHash for DeviceProperties {
    fn write_hash(_hasher: &mut impl core::hash::Hasher) {
        // ignored.
    }
}

impl DeviceProperties {
    /// Create a new feature set with the given features and memory properties.
    pub fn new(
        features: Features,
        memory_props: MemoryDeviceProperties,
        hardware: HardwareProperties,
        timing_method: TimingMethod,
    ) -> Self {
        DeviceProperties {
            features,
            memory: memory_props,
            hardware,
            timing_method,
        }
    }

    /// Get the usages for a type
    pub fn type_usage(&self, ty: StorageType) -> EnumSet<TypeUsage> {
        self.features.type_usage(ty)
    }

    /// Get the usages for an atomic type
    pub fn atomic_type_usage(&self, ty: Type) -> EnumSet<AtomicUsage> {
        self.features.atomic_type_usage(ty)
    }

    /// Whether the type is supported in any way
    pub fn supports_type(&self, ty: impl Into<Type>) -> bool {
        self.features.supports_type(ty)
    }

    /// Whether the address type is supported in any way
    pub fn supports_address(&self, ty: impl Into<AddressType>) -> bool {
        self.features.supports_address(ty)
    }

    /// Register an address type to the features
    pub fn register_address_type(&mut self, ty: impl Into<AddressType>) {
        self.features.types.address.insert(ty.into());
    }

    /// Register an address type to the features
    pub fn register_atomic_type_usage(&mut self, ty: Type, uses: impl Into<EnumSet<AtomicUsage>>) {
        *self.features.types.atomic.entry(ty).or_default() |= uses.into();
    }

    /// Register a storage type to the features
    pub fn register_type_usage(
        &mut self,
        ty: impl Into<StorageType>,
        uses: impl Into<EnumSet<TypeUsage>>,
    ) {
        *self.features.types.storage.entry(ty.into()).or_default() |= uses.into();
    }

    /// Register a semantic type to the features
    pub fn register_semantic_type(&mut self, ty: SemanticType) {
        self.features.types.semantic.insert(ty);
    }

    /// Create a stable hash of all device properties relevant to kernel compilation. Can be used
    /// as a stable checksum for a compilation cache.
    pub fn checksum(&self) -> u64 {
        let state = foldhash::fast::FixedState::default();
        let mut hasher = state.build_hasher();
        self.features.hash(&mut hasher);
        self.hardware.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod allocation_verdict_tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    /// A 2 GiB card, reporting its ceiling the way the CUDA and HIP backends do.
    fn small_card() -> MemoryDeviceProperties {
        MemoryDeviceProperties {
            max_page_size: 2 * GIB / 4,
            alignment: 256,
            total_memory: Some(2 * GIB),
        }
    }

    /// The case this API exists for: a caller sizing a 10 GiB working set against
    /// a 2 GiB device gets a definite "no" from what the device reports, without
    /// allocating anything, and can pick a CPU or streaming path instead.
    #[test]
    fn ten_gib_on_a_two_gib_card_is_refused_before_allocating() {
        let verdict = small_card().can_allocate(10 * GIB);
        assert!(!verdict.fits());
        // The per-allocation ceiling is hit first, and it is the more specific
        // limit: it is what a caller would try to split under.
        assert_eq!(
            verdict,
            AllocationVerdict::ExceedsMaxPageSize {
                max_page_size: 2 * GIB / 4
            }
        );
    }

    /// Past the device's capacity entirely, splitting cannot rescue it. Reported
    /// distinctly from the page-size ceiling so a caller can tell "split this"
    /// apart from "not on this device".
    #[test]
    fn beyond_total_memory_is_distinct_from_beyond_page_size() {
        let props = MemoryDeviceProperties {
            // A device permitting one huge binding but holding little.
            max_page_size: 64 * GIB,
            alignment: 256,
            total_memory: Some(2 * GIB),
        };
        assert_eq!(
            props.can_allocate(10 * GIB),
            AllocationVerdict::ExceedsDeviceMemory {
                total_memory: 2 * GIB
            }
        );
    }

    #[test]
    fn a_request_within_both_limits_fits() {
        assert_eq!(
            small_card().can_allocate(256 * 1024 * 1024),
            AllocationVerdict::Fits
        );
        assert!(small_card().can_allocate(256 * 1024 * 1024).fits());
    }

    /// An unknown capacity must never read as permission. `fits()` is false so a
    /// caller gating on it errs toward the safe path rather than the fast one.
    #[test]
    fn unknown_capacity_is_not_a_yes() {
        let props = MemoryDeviceProperties {
            max_page_size: 64 * GIB,
            alignment: 256,
            total_memory: None,
        };
        let verdict = props.can_allocate(10 * GIB);
        assert_eq!(verdict, AllocationVerdict::Unknown);
        assert!(!verdict.fits(), "unknown must not read as fitting");
        // Still refused when it breaches the one limit that IS known.
        assert_eq!(
            props.can_allocate(65 * GIB),
            AllocationVerdict::ExceedsMaxPageSize {
                max_page_size: 64 * GIB
            }
        );
    }

    /// Exact-fit boundaries are inclusive on both limits -- an allocation equal to
    /// a limit is allowed, so a caller sizing exactly to the reported ceiling is
    /// not turned away by an off-by-one.
    #[test]
    fn limits_are_inclusive() {
        let props = MemoryDeviceProperties {
            max_page_size: 4 * GIB,
            alignment: 256,
            total_memory: Some(4 * GIB),
        };
        assert_eq!(props.can_allocate(4 * GIB), AllocationVerdict::Fits);
        assert!(!props.can_allocate(4 * GIB + 1).fits());
    }
}
