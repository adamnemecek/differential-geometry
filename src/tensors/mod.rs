//! This is a module containing definitions of different tensors
mod tensor;
mod variance;

pub use self::variance::{IndexType, ContravariantIndex, CovariantIndex, TensorIndex,
                       Variance, Concat, Contract};
pub use self::tensor::{Tensor, Vector, Covector, Matrix};
