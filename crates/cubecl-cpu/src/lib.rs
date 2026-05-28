#[macro_use]
extern crate derive_new;

extern crate alloc;

#[cfg(test)]
#[allow(unexpected_cfgs)]
mod tests {
    pub type TestRuntime = crate::CpuRuntime;

    pub use half::f16;

    use cubecl_core as cubecl;
    use cubecl_core::prelude::*;

    cubecl_core::testgen_all!(f32: [f16, f32, f64], i32: [i8, i16, i32, i64], u32: [u8, u16, u32, u64]);
    cubecl_std::testgen!();
    cubecl_std::testgen_tensor_identity!([f16, f32, u32]);
    cubecl_std::testgen_quantized_view!(f32);

    #[cube(launch)]
    fn barrier_smoke(out: &mut Array<f32>) {
        let barrier = barrier::Barrier::local();
        barrier.arrive_and_wait();
        if UNIT_POS == 0 {
            out[0] = 1.0;
        }
    }

    #[cube(launch)]
    fn sync_cube_magic(out: &mut Array<u32>) {
        let mut mem = SharedMemory::<u32>::new(1usize);
        if UNIT_POS == 0 {
            mem[0] = 0xDEADBEEFu32;
        }
        sync_cube();
        out[UNIT_POS as usize] = mem[0];
    }

    #[cube(launch)]
    fn sync_cube_two_phase(out: &mut Array<u32>) {
        let mut mem = SharedMemory::<u32>::new(4usize);
        let idx = UNIT_POS as usize;
        mem[idx] = (idx as u32) + 1;
        sync_cube();

        if UNIT_POS == 0 {
            let mut sum = 0u32;
            for i in 0..4 {
                sum += mem[i];
            }
            mem[0] = sum;
        }
        sync_cube();

        out[idx] = mem[0];
    }

    #[cube(launch)]
    fn sync_cube_all_reduce(out: &mut Array<u32>) {
        let mut mem = SharedMemory::<u32>::new(8usize);
        let idx = UNIT_POS as usize;
        mem[idx] = idx as u32;
        sync_cube();

        let mut sum = 0u32;
        for i in 0..8 {
            sum += mem[i];
        }
        out[idx] = sum;
    }

    #[test]
    fn test_barrier_smoke_cpu() {
        let client = TestRuntime::client(&Default::default());
        let out = client.empty(core::mem::size_of::<f32>());

        unsafe {
            barrier_smoke::launch::<TestRuntime>(
                &client,
                CubeCount::new_single(),
                CubeDim::new_1d(1),
                ArrayArg::from_raw_parts(out.clone(), 1),
            )
        }

        let bytes = client.read_one_unchecked(out);
        let actual = f32::from_bytes(&bytes);
        assert_eq!(actual[0], 1.0);
    }

    #[test]
    fn test_sync_cube_magic_cpu() {
        let client = TestRuntime::client(&Default::default());
        let out = client.empty(4 * core::mem::size_of::<u32>());

        unsafe {
            sync_cube_magic::launch::<TestRuntime>(
                &client,
                CubeCount::new_single(),
                CubeDim::new_1d(4),
                ArrayArg::from_raw_parts(out.clone(), 4),
            )
        }

        let bytes = client.read_one_unchecked(out);
        let actual = u32::from_bytes(&bytes);
        assert_eq!(actual, &[0xDEADBEEF; 4]);
    }

    #[test]
    fn test_sync_cube_two_phase_cpu() {
        let client = TestRuntime::client(&Default::default());
        let out = client.empty(4 * core::mem::size_of::<u32>());

        unsafe {
            sync_cube_two_phase::launch::<TestRuntime>(
                &client,
                CubeCount::new_single(),
                CubeDim::new_1d(4),
                ArrayArg::from_raw_parts(out.clone(), 4),
            )
        }

        let bytes = client.read_one_unchecked(out);
        let actual = u32::from_bytes(&bytes);
        assert_eq!(actual, &[10u32; 4]);
    }

    #[test]
    fn test_sync_cube_all_reduce_cpu() {
        let client = TestRuntime::client(&Default::default());
        let out = client.empty(8 * core::mem::size_of::<u32>());

        unsafe {
            sync_cube_all_reduce::launch::<TestRuntime>(
                &client,
                CubeCount::new_single(),
                CubeDim::new_1d(8),
                ArrayArg::from_raw_parts(out.clone(), 8),
            )
        }

        let bytes = client.read_one_unchecked(out);
        let actual = u32::from_bytes(&bytes);
        assert_eq!(actual, &[28u32; 8]);
    }

    // Regression test for the multi-cube + SharedMemory + sync_cube
    // bug surfaced by cvvdp-gpu's downscale_tiled_kernel on cubecl-cpu.
    // Each cube writes its CUBE_POS_X into shared memory at slot 0,
    // sync_cubes, then every unit in the cube reads it back to its
    // per-cube output slice. If multi-cube dispatch correctly
    // isolates shared memory between cubes (each cube must see its
    // own CUBE_POS_X), output[cube_x * cube_dim_size + unit] ==
    // cube_x. If shared memory leaks across cubes (the bug), output
    // would show whatever the last-running cube wrote.
    #[cube(launch)]
    fn sync_cube_multi_cube_writes_pos(out: &mut Array<u32>) {
        let mut mem = SharedMemory::<u32>::new(1usize);
        if UNIT_POS == 0 {
            mem[0] = CUBE_POS_X;
        }
        sync_cube();
        let idx = CUBE_POS_X * CUBE_DIM_X + UNIT_POS_X;
        out[idx as usize] = mem[0];
    }

    #[test]
    fn test_sync_cube_multi_cube_writes_pos_cpu() {
        let client = TestRuntime::client(&Default::default());
        // 3 cubes × 4 units = 12 outputs.
        let n_cubes: u32 = 3;
        let cube_dim_size: u32 = 4;
        let n = (n_cubes * cube_dim_size) as usize;
        let out = client.empty(n * core::mem::size_of::<u32>());

        unsafe {
            sync_cube_multi_cube_writes_pos::launch::<TestRuntime>(
                &client,
                CubeCount::Static(n_cubes, 1, 1),
                CubeDim::new_1d(cube_dim_size),
                ArrayArg::from_raw_parts(out.clone(), n),
            )
        }

        let bytes = client.read_one_unchecked(out);
        let actual = u32::from_bytes(&bytes);
        // Each cube_x writes its own index into shared memory; all
        // units in that cube should read it back. With the bug, the
        // global barrier means units may read shared memory written
        // by a different cube — output would have wrong values.
        let expected: Vec<u32> = (0..n_cubes)
            .flat_map(|c| std::iter::repeat(c).take(cube_dim_size as usize))
            .collect();
        assert_eq!(
            actual,
            expected.as_slice(),
            "multi-cube SharedMemory/sync_cube isolation bug: got {actual:?}, expected {expected:?}"
        );
    }
}

pub mod compiler;
pub mod compute;
pub mod device;
pub mod frontend;
pub mod runtime;

pub use device::CpuDevice;
pub use runtime::*;
