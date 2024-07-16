use cubecl_core as cubecl;
use cubecl_core::prelude::*;

use super::block_loop::block_loop;
use super::config::CmmaConfig;

pub(crate) const USE_CMMA: bool = false;

#[cube(launch)]
#[allow(unused_mut)]
pub fn cmma_kernel<F: Float, FC: Float>(
    lhs: &Tensor<F>,
    rhs: &Tensor<F>,
    out: &mut Tensor<F>,
    config: Comptime<CmmaConfig>,
) {
    let dims = get_dims::<F>(lhs, rhs);
    let offsets = calculate_offsets::<F>(lhs, rhs, out, config);
    let shared_memories = make_shared_memories::<F, FC>(config);
    block_loop::<F, FC>(lhs, rhs, out, offsets, shared_memories, config, dims);
}

#[derive(CubeType, Copy, Clone)]
/// Information available at runtime only
/// Strides assume contiguous
pub(crate) struct Dimensions {
    pub m: UInt,
    pub k: UInt,
    pub n: UInt,
}

#[derive(CubeType, Copy, Clone)]
pub(crate) struct SharedMemories<F: Float, FC: Float> {
    pub lhs: SharedMemory<FC>,
    pub rhs: SharedMemory<FC>,
    pub accumulate: SharedMemory<F>,
}

#[derive(CubeType, Copy, Clone)]
/// Not divided by vectorization factor
///
/// Note: batch offsets take stride into account, but not the others
pub(crate) struct Offsets {
    pub batch_lhs: UInt,
    pub batch_rhs: UInt,
    pub batch_out: UInt,
    pub cube_row: UInt,
    pub cube_col: UInt,
    pub k: UInt,
}

#[cube]
fn get_dims<F: Float>(lhs: &Tensor<F>, rhs: &Tensor<F>) -> Dimensions {
    let rank = lhs.rank();
    let first_dim = rank - UInt::new(2);
    let second_dim = rank - UInt::new(1);
    let m = lhs.shape(first_dim);
    let k = lhs.shape(second_dim);
    let n = rhs.shape(second_dim);

    Dimensions { m, k, n }
}

#[cube]
fn calculate_offsets<F: Float>(
    lhs: &Tensor<F>,
    rhs: &Tensor<F>,
    out: &Tensor<F>,
    config: Comptime<CmmaConfig>,
) -> Offsets {
    let block_size_m = Comptime::map(config, |c| c.block_size_m);
    let block_size_n = Comptime::map(config, |c| c.block_size_n);

    // Cube offset
    let cube_row = CUBE_POS_X * Comptime::runtime(block_size_m);
    let cube_col = CUBE_POS_Y * Comptime::runtime(block_size_n);

    let rank = out.rank();

    let dim_m = lhs.shape(rank - UInt::new(2));
    let dim_n = rhs.shape(rank - UInt::new(1));

    // Batch offset for output
    let mut batch_out = dim_m * dim_n * CUBE_POS_Z;
    let mut batch_lhs = UInt::new(0);
    let mut batch_rhs = UInt::new(0);

    // Batch offset for lhs, rhs
    for b in range(0u32, rank - UInt::new(2), Comptime::new(false)) {
        let tmp = batch_out / out.stride(b);
        batch_lhs += tmp % lhs.shape(b) * lhs.stride(b);
        batch_rhs += tmp % rhs.shape(b) * rhs.stride(b);
    }

    Offsets {
        batch_lhs,
        batch_rhs,
        batch_out,
        cube_row,
        cube_col,
        k: UInt::new(0), // Changes during kernel
    }
}

#[cube]
fn make_shared_memories<F: Float, FC: Float>(
    config: Comptime<CmmaConfig>,
) -> SharedMemories<F, FC> {
    let sm_vec = Comptime::map(config, |c| c.sm_vec);
    let block_size_m = Comptime::map(config, |c| c.block_size_m);
    let block_size_k = Comptime::map(config, |c| c.block_size_k);
    let block_size_n = Comptime::map(config, |c| c.block_size_n);
    let unroll = Comptime::map(config, |c| c.unroll);

    let lhs = SharedMemory::<FC>::new(Comptime::get(block_size_k * block_size_m));

    let rhs = SharedMemory::<FC>::new(Comptime::get(block_size_k * block_size_n));

    let mut accumulate = SharedMemory::<F>::new(Comptime::get(block_size_m * block_size_n));

    // Init accumulation to 0
    for i in range(0u32, Comptime::get(block_size_m * block_size_n), unroll) {
        accumulate[i] = F::new(0.);
    }

    SharedMemories {
        lhs,
        rhs,
        accumulate,
    }
}
