//! This module defines the `Tensor` type and all sorts of operations on it.

use super::variance::{Concat, Contract, Contracted, Joined, OtherIndex};
use super::{ContravariantIndex, CovariantIndex, IndexType, TensorIndex, Variance};
use crate::coordinates::{ConversionTo, CoordinateSystem, Point};
use crate::typenum::{
    consts::{B1, U2},
    uint::Unsigned,
    Add1, Exp, Pow, Same,
};
use generic_array::{ArrayLength, GenericArray};
use std::ops::{
    Add, AddAssign, Deref, DerefMut, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub,
    SubAssign,
};

/// Struct representing a tensor.
///
/// A tensor is anchored at a given point and has coordinates
/// represented in the system defined by the generic parameter
/// `T`. The variance of the tensor (meaning its rank and types
/// of its indices) is defined by `V`. This allows Rust
/// to decide at compile time whether two tensors are legal
/// to be added / multiplied / etc.
///
/// It is only OK to perform an operation on two tensors if
/// they belong to the same coordinate system.
pub struct Tensor<T: CoordinateSystem, U: Variance>
where
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    p: Point<T>,
    x: GenericArray<f64, Exp<T::Dimension, U::Rank>>,
}

impl<T, U> Clone for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn clone(&self) -> Self {
        Self {
            p: self.p.clone(),
            x: self.x.clone(),
        }
    }
}

impl<T, U> Copy for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    <T::Dimension as ArrayLength<f64>>::ArrayType: Copy,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
    <Exp<T::Dimension, U::Rank> as ArrayLength<f64>>::ArrayType: Copy,
{
}

/// A struct for iterating over the coordinates of a tensor.
pub struct CoordIterator<U>
where
    U: Variance,
    U::Rank: ArrayLength<usize>,
{
    started: bool,
    dimension: usize,
    cur_coord: GenericArray<usize, U::Rank>,
}

impl<U> CoordIterator<U>
where
    U: Variance,
    U::Rank: ArrayLength<usize>,
{
    pub fn new(dimension: usize) -> Self {
        Self {
            started: false,
            dimension: dimension,
            cur_coord: <_>::default(),
        }
    }
}

impl<U> Iterator for CoordIterator<U>
where
    U: Variance,
    U::Rank: ArrayLength<usize>,
{
    type Item = GenericArray<usize, U::Rank>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.started = true;
            return Some(self.cur_coord.clone());
        }

        // handle scalars
        if self.cur_coord.is_empty() {
            return None;
        }

        let mut i = self.cur_coord.len() - 1;
        loop {
            self.cur_coord[i] += 1;
            if self.cur_coord[i] < self.dimension {
                break;
            }
            self.cur_coord[i] = 0;
            if i == 0 {
                return None;
            }
            i -= 1;
        }

        Some(self.cur_coord.clone())
    }
}

impl<T, V> Tensor<T, V>
where
    T: CoordinateSystem,
    V: Variance,
    T::Dimension: Pow<V::Rank>,
    Exp<T::Dimension, V::Rank>: ArrayLength<f64>,
{
    /// Returns the point at which the tensor is defined.
    pub fn get_point(&self) -> &Point<T> {
        &self.p
    }

    /// Sets the point at which the tensor is defined.
    pub fn set_point(&mut self, p: Point<T>) {
        self.p = p;
    }

    /// Returns the tensor's coordinates as an array
    pub fn coords_array(&self) -> &GenericArray<f64, Exp<T::Dimension, V::Rank>> {
        &self.x
    }

    /// Converts a set of tensor indices passed as a slice into a single index
    /// for the internal array.
    ///
    /// The length of the slice (the number of indices) has to be compatible
    /// with the rank of the tensor.
    pub fn get_coord(i: &[usize]) -> usize {
        assert_eq!(i.len(), V::rank());
        let dim = T::dimension();
        let index = i.into_iter().fold(0, |res, idx| {
            assert!(*idx < dim);
            res * dim + idx
        });
        index
    }

    /// Returns the variance of the tensor, that is, the list of the index types.
    /// A vector would return vec![Contravariant], a metric tensor: vec![Covariant, Covariant].
    pub fn get_variance() -> Vec<IndexType> {
        V::variance()
    }

    /// Returns the rank of the tensor
    pub fn get_rank() -> usize {
        V::rank()
    }

    /// Returns the number of coordinates of the tensor (equal to [Dimension]^[Rank])
    pub fn get_num_coords() -> usize {
        <T::Dimension as Pow<V::Rank>>::Output::to_usize()
    }

    /// Creates a new, zero tensor at a given point
    pub fn zero(point: Point<T>) -> Self {
        Self {
            p: point,
            x: <_>::default(),
        }
    }

    /// Creates a tensor at a given point with the coordinates defined by the array.
    ///
    /// The number of elements in the array must be equal to the number of coordinates
    /// of the tensor.
    ///
    /// One-dimensional array represents an n-dimensional tensor in such a way, that
    /// the last index is the one that is changing the most often, i.e. the sequence is
    /// as follows:
    /// (0,0,...,0), (0,0,...,1), (0,0,...,2), ..., (0,0,...,1,0), (0,0,...,1,1), ... etc.
    pub fn new(point: Point<T>, coords: GenericArray<f64, Exp<T::Dimension, V::Rank>>) -> Self {
        Self {
            p: point,
            x: coords,
        }
    }

    /// Creates a tensor at a given point with the coordinates defined by the slice.
    ///
    /// The number of elements in the slice must be equal to the number of coordinates
    /// of the tensor.
    ///
    /// One-dimensional slice represents an n-dimensional tensor in such a way, that
    /// the last index is the one that is changing the most often, i.e. the sequence is
    /// as follows:
    /// (0,0,...,0), (0,0,...,1), (0,0,...,2), ..., (0,0,...,1,0), (0,0,...,1,1), ... etc.
    pub fn from_slice(point: Point<T>, slice: &[f64]) -> Self {
        assert_eq!(Self::get_num_coords(), slice.len());
        Self {
            p: point,
            x: GenericArray::clone_from_slice(slice),
        }
    }

    /// Contracts two indices
    ///
    /// The indices must be of opposite types. This is checked at compile time.
    pub fn trace<Ul, Uh>(&self) -> Tensor<T, Contracted<V, Ul, Uh>>
    where
        Ul: Unsigned,
        Uh: Unsigned,
        V: Contract<Ul, Uh>,
        <Contracted<V, Ul, Uh> as Variance>::Rank: ArrayLength<usize>,
        T::Dimension: Pow<<Contracted<V, Ul, Uh> as Variance>::Rank>,
        Exp<T::Dimension, <Contracted<V, Ul, Uh> as Variance>::Rank>: ArrayLength<f64>,
    {
        let index1 = Ul::to_usize();
        let index2 = Uh::to_usize();
        let rank = V::Rank::to_usize();
        let dim = T::Dimension::to_usize();

        let mut result = Tensor::<T, Contracted<V, Ul, Uh>>::zero(self.p.clone());
        let num_coords_result = Tensor::<T, Contracted<V, Ul, Uh>>::get_num_coords();
        let modh = dim.pow((rank - 1 - index2) as u32);
        let modl = dim.pow((rank - 2 - index1) as u32);

        for coord in 0..num_coords_result {
            let coord1 = coord / modl;
            let coord1rest = coord % modl;
            let coord2 = coord1rest / modh;
            let coord2rest = coord1rest % modh;
            let coord_template = coord1 * modl * dim * dim + coord2 * modh * dim + coord2rest;
            let mut sum = 0.0;

            for i in 0..T::dimension() {
                sum += self[coord_template + i * modl * dim + i * modh];
            }

            result[coord] = sum;
        }

        result
    }
}

impl<T, U> Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    U::Rank: ArrayLength<usize>,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    /// Returns an iterator over the coordinates of the tensor.
    pub fn iter_coords(&self) -> CoordIterator<U> {
        CoordIterator::new(T::dimension())
    }
}

impl<'a, T, U> Index<&'a [usize]> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = f64;

    fn index(&self, idx: &'a [usize]) -> &f64 {
        &self.x[Self::get_coord(idx)]
    }
}

impl<'a, T, U> IndexMut<&'a [usize]> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn index_mut(&mut self, idx: &'a [usize]) -> &mut f64 {
        &mut self.x[Self::get_coord(idx)]
    }
}

impl<'a, T, U> Index<usize> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = f64;

    fn index(&self, idx: usize) -> &f64 {
        &self.x[idx]
    }
}

impl<'a, T, U> IndexMut<usize> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn index_mut(&mut self, idx: usize) -> &mut f64 {
        &mut self.x[idx]
    }
}

/// A scalar type, which is a tensor with rank 0.
///
/// This is de facto just a number, so it implements `Deref` and `DerefMut` into `f64`.
pub type Scalar<T> = Tensor<T, ()>;

/// A vector type (rank 1 contravariant tensor)
pub type Vector<T> = Tensor<T, ContravariantIndex>;

/// A covector type (rank 1 covariant tensor)
pub type Covector<T> = Tensor<T, CovariantIndex>;

/// A matrix type (rank 2 contravariant-covariant tensor)
pub type Matrix<T> = Tensor<T, (ContravariantIndex, CovariantIndex)>;

/// A bilinear form type (rank 2 doubly covariant tensor)
pub type TwoForm<T> = Tensor<T, (CovariantIndex, CovariantIndex)>;

/// A rank 2 doubly contravariant tensor
pub type InvTwoForm<T> = Tensor<T, (ContravariantIndex, ContravariantIndex)>;

impl<T: CoordinateSystem> Deref for Scalar<T> {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.x[0]
    }
}

impl<T: CoordinateSystem> DerefMut for Scalar<T> {
    fn deref_mut(&mut self) -> &mut f64 {
        &mut self.x[0]
    }
}

// Arithmetic operations

impl<T, U> AddAssign for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn add_assign(&mut self, rhs: Self) {
        assert!(self.p == rhs.p);
        for i in 0..(Self::get_num_coords()) {
            self[i] += rhs[i];
        }
    }
}

impl<T, U> Add for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl<T, U> SubAssign for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn sub_assign(&mut self, rhs: Self) {
        assert!(self.p == rhs.p);
        for i in 0..(Self::get_num_coords()) {
            self[i] -= rhs[i];
        }
    }
}

impl<T, U> Sub for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl<T, U> MulAssign<f64> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn mul_assign(&mut self, rhs: f64) {
        for i in 0..(Self::get_num_coords()) {
            self[i] *= rhs;
        }
    }
}

impl<T, U> Mul<f64> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self {
        self *= rhs;
        self
    }
}

impl<T, U> Mul<Tensor<T, U>> for f64
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = Tensor<T, U>;

    fn mul(self, mut rhs: Tensor<T, U>) -> Tensor<T, U> {
        rhs *= self;
        rhs
    }
}

impl<T, U> DivAssign<f64> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    fn div_assign(&mut self, rhs: f64) {
        for i in 0..(Self::get_num_coords()) {
            self[i] /= rhs;
        }
    }
}

impl<T, U> Div<f64> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    type Output = Self;

    fn div(mut self, rhs: f64) -> Self {
        self /= rhs;
        self
    }
}

// Tensor multiplication

// For some reason this triggers recursion overflow when tested - to be investigated
impl<T, U, V> Mul<Tensor<T, V>> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    V: Variance,
    U::Rank: ArrayLength<usize>,
    V::Rank: ArrayLength<usize>,
    T::Dimension: Pow<U::Rank> + Pow<V::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
    Exp<T::Dimension, V::Rank>: ArrayLength<f64>,
    U: Concat<V>,
    Joined<U, V>: Variance,
    T::Dimension: Pow<<Joined<U, V> as Variance>::Rank>,
    Exp<T::Dimension, <Joined<U, V> as Variance>::Rank>: ArrayLength<f64>,
{
    type Output = Tensor<T, Joined<U, V>>;

    fn mul(self, rhs: Tensor<T, V>) -> Self::Output {
        assert!(self.p == rhs.p);
        let mut result = Tensor::zero(self.p.clone());
        let num_coords2 = Tensor::<T, V>::get_num_coords();
        let num_coords_result = Tensor::<T, Joined<U, V>>::get_num_coords();
        for coord in 0..num_coords_result {
            let coord1 = coord / num_coords2;
            let coord2 = coord % num_coords2;
            result[coord] = self[coord1] * rhs[coord2];
        }
        result
    }
}

/// Trait representing the inner product of two tensors.
///
/// The inner product is just a multiplication followed by a contraction.
/// The contraction is defined by type parameters `Ul` and `Uh`. `Ul` has to
/// be less than `Uh` and the indices at those positions must be of opposite types
/// (checked at compile time)
pub trait InnerProduct<Rhs, Ul: Unsigned, Uh: Unsigned> {
    type Output;

    fn inner_product(self, rhs: Rhs) -> Self::Output;
}

impl<T, U, V, Ul, Uh> InnerProduct<Tensor<T, V>, Ul, Uh> for Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    V: Variance,
    Ul: Unsigned,
    Uh: Unsigned,
    T::Dimension: Pow<U::Rank> + Pow<V::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
    Exp<T::Dimension, V::Rank>: ArrayLength<f64>,
    U: Concat<V>,
    Joined<U, V>: Contract<Ul, Uh>,
    <Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank: ArrayLength<usize>,
    T::Dimension: Pow<<Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank>,
    Exp<T::Dimension, <Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank>: ArrayLength<f64>,
{
    type Output = Tensor<T, Contracted<Joined<U, V>, Ul, Uh>>;

    fn inner_product(self, rhs: Tensor<T, V>) -> Self::Output {
        assert_eq!(self.p, rhs.p);
        let indexl = Ul::to_usize();
        let indexh = Uh::to_usize();
        let num_coords_result = Self::Output::get_num_coords();
        let u_rank = U::Rank::to_usize();
        let v_rank = V::Rank::to_usize();
        let dim = T::Dimension::to_usize();

        let mut result = Tensor::<T, Contracted<Joined<U, V>, Ul, Uh>>::zero(self.p.clone());
        let (modl, modh, modv) = match (indexl < u_rank, indexh < u_rank) {
            (true, true) => (
                dim.pow((u_rank - 2 - indexl) as u32),
                dim.pow((u_rank - 1 - indexh) as u32),
                dim.pow(v_rank as u32),
            ),
            (true, false) => (
                dim.pow((u_rank - 1 - indexl) as u32),
                dim.pow((u_rank + v_rank - 1 - indexh) as u32),
                dim.pow((v_rank - 1) as u32),
            ),
            (false, false) => (
                dim.pow((u_rank + v_rank - 2 - indexl) as u32),
                dim.pow((u_rank + v_rank - 1 - indexh) as u32),
                dim.pow(v_rank as u32),
            ),
            _ => unreachable!(),
        };

        let to_templates_both1 = |coord| {
            let coords1 = coord / modv;
            let coords2 = coord % modv;
            let coords1part1 = coords1 / modl;
            let coords1part2 = (coords1 % modl) / modh;
            let coords1part3 = coords1 % modh;
            (
                coords1part1 * modl * dim * dim + coords1part2 * modh * dim + coords1part3,
                coords2,
                modl + modh,
                0,
            )
        };

        let to_templates_both2 = |coord| {
            let coords1 = coord / modv;
            let coords2 = coord % modv;
            let coords2part1 = coords2 / modl;
            let coords2part2 = (coords2 % modl) / modh;
            let coords2part3 = coords2 % modh;
            (
                coords1,
                coords2part1 * modl * dim * dim + coords2part2 * modh * dim + coords2part3,
                0,
                modl + modh,
            )
        };

        let to_templates = |coord| {
            let coords1 = coord / modv;
            let coords2 = coord % modv;
            let coords1part1 = coords1 / modl;
            let coords1part2 = coords1 % modl;
            let coords2part1 = coords2 / modh;
            let coords2part2 = coords2 % modh;
            (
                coords1part1 * modl * dim + coords1part2,
                coords2part1 * modh * dim + coords2part2,
                modl,
                modh,
            )
        };

        let templates: &Fn(usize) -> (usize, usize, usize, usize) =
            match (indexl < u_rank, indexh < u_rank) {
                (false, false) => &to_templates_both2,
                (true, false) => &to_templates,
                (true, true) => &to_templates_both1,
                _ => unreachable!(),
            };

        for coord in 0..num_coords_result {
            let mut sum = 0.0;
            let (mut coord1, mut coord2, step1, step2) = templates(coord);
            for _ in 0..dim {
                sum += self[coord1] * rhs[coord2];
                coord1 += step1;
                coord2 += step2;
            }
            result[coord] = sum;
        }

        result
    }
}

impl<T, Ul, Ur> Tensor<T, (Ul, Ur)>
where
    T: CoordinateSystem,
    Ul: TensorIndex + OtherIndex,
    Ur: TensorIndex + OtherIndex,
    Add1<Ul::Rank>: Unsigned + Add<B1>,
    Add1<Ur::Rank>: Unsigned + Add<B1>,
    Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>: Unsigned + Add<B1>,
    Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>: Unsigned + Add<B1>,
    <(Ul, Ur) as Variance>::Rank: ArrayLength<usize>,
    T::Dimension: Pow<Add1<Ul::Rank>> + Pow<Add1<Ur::Rank>> + ArrayLength<usize>,
    T::Dimension: Pow<Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>>,
    T::Dimension: Pow<Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>>,
    Exp<T::Dimension, Add1<Ul::Rank>>: ArrayLength<f64>,
    Exp<T::Dimension, Add1<Ur::Rank>>: ArrayLength<f64>,
    Exp<T::Dimension, Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>>: ArrayLength<f64>,
    Exp<T::Dimension, Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>>: ArrayLength<f64>,
{
    /// Returns a unit matrix (1 on the diagonal, 0 everywhere else)
    pub fn unit(p: Point<T>) -> Tensor<T, (Ul, Ur)> {
        let mut result = Tensor::<T, (Ul, Ur)>::zero(p);

        for i in 0..T::dimension() {
            let coords: &[usize] = &[i, i];
            result[coords] = 1.0;
        }

        result
    }

    /// Transposes the matrix
    pub fn transpose(&self) -> Tensor<T, (Ur, Ul)> {
        let mut result = Tensor::<T, (Ur, Ul)>::zero(self.p.clone());

        for coords in self.iter_coords() {
            let coords2: &[usize] = &[coords[1], coords[0]];
            result[coords2] = self[&*coords];
        }

        result
    }

    // Function calculating the LU decomposition of a matrix - found in the internet
    // The decomposition is done in-place and a permutation vector is returned (or None
    // if the matrix was singular)
    fn lu_decompose(&mut self) -> Option<GenericArray<usize, T::Dimension>> {
        let n = T::dimension();
        let absmin = 1.0e-30_f64;
        let mut result = GenericArray::default();
        let mut row_norm = GenericArray::<f64, T::Dimension>::default();

        let mut max_row = 0;

        for i in 0..n {
            let mut absmax = 0.0;

            for j in 0..n {
                let coord: &[usize] = &[i, j];
                let maxtemp = self[coord].abs();
                absmax = if maxtemp > absmax { maxtemp } else { absmax };
            }

            if absmax == 0.0 {
                return None;
            }

            row_norm[i] = 1.0 / absmax;
        }

        for j in 0..n {
            for i in 0..j {
                for k in 0..i {
                    let coord1: &[usize] = &[i, j];
                    let coord2: &[usize] = &[i, k];
                    let coord3: &[usize] = &[k, j];

                    self[coord1] -= self[coord2] * self[coord3];
                }
            }

            let mut absmax = 0.0;

            for i in j..n {
                let coord1: &[usize] = &[i, j];

                for k in 0..j {
                    let coord2: &[usize] = &[i, k];
                    let coord3: &[usize] = &[k, j];

                    self[coord1] -= self[coord2] * self[coord3];
                }

                let maxtemp = self[coord1].abs() * row_norm[i];

                if maxtemp > absmax {
                    absmax = maxtemp;
                    max_row = i;
                }
            }

            if max_row != j {
                if (j == n - 2) && self[&[j, j + 1] as &[usize]] == 0.0 {
                    max_row = j;
                } else {
                    for k in 0..n {
                        let jk: &[usize] = &[j, k];
                        let maxrow_k: &[usize] = &[max_row, k];
                        let maxtemp = self[jk];
                        self[jk] = self[maxrow_k];
                        self[maxrow_k] = maxtemp;
                    }

                    row_norm[max_row] = row_norm[j];
                }
            }

            result[j] = max_row;

            let jj: &[usize] = &[j, j];

            if self[jj] == 0.0 {
                self[jj] = absmin;
            }

            if j != n - 1 {
                let maxtemp = 1.0 / self[jj];
                for i in j + 1..n {
                    self[&[i, j] as &[usize]] *= maxtemp;
                }
            }
        }

        Some(result)
    }

    // Function solving a linear system of equations (self*x = b) using the LU decomposition
    fn lu_substitution(
        &self,
        b: &GenericArray<f64, T::Dimension>,
        permute: &GenericArray<usize, T::Dimension>,
    ) -> GenericArray<f64, T::Dimension> {
        let mut result = b.clone();
        let n = T::dimension();

        for i in 0..n {
            let mut tmp = result[permute[i]];
            result[permute[i]] = result[i];
            for j in (0..i).rev() {
                tmp -= self[&[i, j] as &[usize]] * result[j];
            }
            result[i] = tmp;
        }

        for i in (0..n).rev() {
            for j in i + 1..n {
                result[i] -= self[&[i, j] as &[usize]] * result[j];
            }
            result[i] /= self[&[i, i] as &[usize]];
        }

        result
    }

    /// Function calculating the inverse of `self` using the LU ddecomposition.
    ///
    /// The return value is an `Option`, since `self` may be non-invertible -
    /// in such a case, None is returned
    pub fn inverse(
        &self,
    ) -> Option<Tensor<T, (<Ul as OtherIndex>::Output, <Ur as OtherIndex>::Output)>> {
        let mut result =
            Tensor::<T, (<Ul as OtherIndex>::Output, <Ur as OtherIndex>::Output)>::zero(
                self.p.clone(),
            );

        let mut tmp = self.clone();

        let permute = match tmp.lu_decompose() {
            Some(p) => p,
            None => return None,
        };

        for i in 0..T::dimension() {
            let mut dxm = GenericArray::<f64, T::Dimension>::default();
            dxm[i] = 1.0;

            let x = tmp.lu_substitution(&dxm, &permute);

            for k in 0..T::dimension() {
                result[&[k, i] as &[usize]] = x[k];
            }
        }

        Some(result)
    }
}

impl<T, U> Tensor<T, U>
where
    T: CoordinateSystem,
    U: Variance,
    U::Rank: ArrayLength<usize>,
    T::Dimension: Pow<U::Rank>,
    Exp<T::Dimension, U::Rank>: ArrayLength<f64>,
{
    pub fn convert<T2>(&self) -> Tensor<T2, U>
    where
        T2: CoordinateSystem + 'static,
        T2::Dimension: Pow<U::Rank> + Pow<U2> + Same<T::Dimension>,
        Exp<T2::Dimension, U::Rank>: ArrayLength<f64>,
        Exp<T2::Dimension, U2>: ArrayLength<f64>,
        T: ConversionTo<T2>,
    {
        let mut result = Tensor::<T2, U>::zero(<T as ConversionTo<T2>>::convert_point(&self.p));

        let jacobian = <T as ConversionTo<T2>>::jacobian(&self.p);
        let inv_jacobian = <T as ConversionTo<T2>>::inv_jacobian(&self.p);
        let variance = <U as Variance>::variance();

        for i in result.iter_coords() {
            let mut temp = 0.0;
            for j in self.iter_coords() {
                let mut temp2 = self[&*j];
                for (k, v) in variance.iter().enumerate() {
                    let coords = [i[k], j[k]];
                    temp2 *= match *v {
                        IndexType::Covariant => inv_jacobian[&coords[..]],
                        IndexType::Contravariant => jacobian[&coords[..]],
                    };
                }
                temp += temp2;
            }
            result[&*i] = temp;
        }

        result
    }
}
