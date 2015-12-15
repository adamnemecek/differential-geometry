use coordinates::{CoordinateSystem, Point};
use std::ops::{Index, IndexMut};
use std::ops::{Add, Sub, Mul, Div, Deref, DerefMut};
use typenum::uint::Unsigned;
use typenum::consts::U1;
use typenum::{Pow};
use generic_array::{GenericArray, ArrayLength};
use super::{CovariantIndex, ContravariantIndex, TensorIndex, Variance, IndexType};
use super::variance::{Concat, Contract, Joined, Contracted, Add1, OtherIndex};

pub type Power<T, U> = <T as Pow<U>>::Output;

/// This is a struct that represents a generic tensor
pub struct Tensor<T: CoordinateSystem, U: Variance>
    where T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    p: Point<T>,
    x: GenericArray<f64, Power<T::Dimension, U::Rank>>
}

impl<T, U> Clone for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    fn clone(&self) -> Tensor<T, U> {
        Tensor {
            p: self.p.clone(),
            x: self.x.clone()
        }
    }
}

impl<T, U> Copy for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          <T::Dimension as ArrayLength<f64>>::ArrayType: Copy,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>,
          <Power<T::Dimension, U::Rank> as ArrayLength<f64>>::ArrayType: Copy 
{}

pub struct CoordIterator<U>
    where U: Variance,
          U::Rank: ArrayLength<usize>
{
    started: bool,
    dimension: usize,
    cur_coord: GenericArray<usize, U::Rank>
}

impl<U> CoordIterator<U>
    where U: Variance,
          U::Rank: ArrayLength<usize>
{
    pub fn new(dimension: usize) -> CoordIterator<U> {
        CoordIterator {
            started: false,
            dimension: dimension,
            cur_coord: GenericArray::new()
        }
    }
}

impl<U> Iterator for CoordIterator<U>
    where U: Variance,
          U::Rank: ArrayLength<usize>
{
    type Item = GenericArray<usize, U::Rank>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.started = true;
            return Some(self.cur_coord.clone())
        }

        // handle scalars
        if self.cur_coord.len() < 1 {
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
    where T: CoordinateSystem,
          V: Variance,
          T::Dimension: Pow<V::Rank>,
          Power<T::Dimension, V::Rank>: ArrayLength<f64>
{
    /// Returns the point at which the tensor is defined.
    pub fn get_point(&self) -> &Point<T> {
        &self.p
    }

    /// Converts a set of tensor indices passed as a slice into a single index for the internal array.
    /// The length of the slice (the number of indices) has to be compatible with the rank of the tensor. 
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

    /// Returns the number of coordinates of the tensor
    pub fn get_num_coords() -> usize {
        <T::Dimension as Pow<V::Rank>>::Output::to_usize()
    }

    pub fn new(point: Point<T>) -> Tensor<T, V> {
        Tensor {
            p: point,
            x: GenericArray::new()
        }
    }

    pub fn trace<Ul, Uh>(&self) -> Tensor<T, Contracted<V, Ul, Uh>>
        where Ul: Unsigned,
              Uh: Unsigned,
              V: Contract<Ul, Uh>,
              <Contracted<V, Ul, Uh> as Variance>::Rank: ArrayLength<usize>,
              T::Dimension: Pow<<Contracted<V, Ul, Uh> as Variance>::Rank>,
              Power<T::Dimension, <Contracted<V, Ul, Uh> as Variance>::Rank>: ArrayLength<f64>
    {
        let index1 = Ul::to_usize();
        let index2 = Uh::to_usize();

        let mut result = Tensor::<T, Contracted<V, Ul, Uh>>::new(self.p.clone());

        for coord in result.iter_coords() {
            let mut sum = 0.0;

            for i in 0..T::dimension() {
                let mut vec_coords = coord.to_vec();
                vec_coords.insert(index1, i);
                vec_coords.insert(index2, i);
                sum += self[&*vec_coords];
            }

            result[&*coord] = sum;
        }

        result
    }
}

impl<T, U> Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          U::Rank: ArrayLength<usize>,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    pub fn iter_coords(&self) -> CoordIterator<U> {
        CoordIterator::new(T::dimension())
    }
}

impl<'a, T, U> Index<&'a [usize]> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = f64;

    fn index(&self, idx: &'a [usize]) -> &f64 {
        &self.x[Self::get_coord(idx)]
    }
}

impl<'a, T, U> IndexMut<&'a [usize]> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    fn index_mut(&mut self, idx: &'a [usize]) -> &mut f64 {
        &mut self.x[Self::get_coord(idx)]
    }
}

impl<'a, T, U> Index<usize> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = f64;

    fn index(&self, idx: usize) -> &f64 {
        &self.x[idx]
    }
}

impl<'a, T, U> IndexMut<usize> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    fn index_mut(&mut self, idx: usize) -> &mut f64 {
        &mut self.x[idx]
    }
}

pub type Scalar<T> = Tensor<T, ()>;
pub type Vector<T> = Tensor<T, ContravariantIndex>;
pub type Covector<T> = Tensor<T, CovariantIndex>;
pub type Matrix<T> = Tensor<T, (ContravariantIndex, CovariantIndex)>;

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

impl<T, U> Add<Tensor<T, U>> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, U>;

    fn add(mut self, rhs: Tensor<T, U>) -> Tensor<T, U> {
        assert!(self.p == rhs.p);
        for i in 0..(Tensor::<T, U>::get_num_coords()) {
            self[i] = self[i] + rhs[i];
        }
        self
    }
}

impl<T, U> Sub<Tensor<T, U>> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, U>;

    fn sub(mut self, rhs: Tensor<T, U>) -> Tensor<T, U> {
        assert!(self.p == rhs.p);
        for i in 0..(Tensor::<T, U>::get_num_coords()) {
            self[i] = self[i] - rhs[i];
        }
        self
    }
}

impl<T, U> Mul<f64> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, U>;

    fn mul(mut self, rhs: f64) -> Tensor<T, U> {
        for i in 0..(Tensor::<T, U>::get_num_coords()) {
            self[i] = self[i] * rhs;
        }
        self
    }
}

impl<T, U> Mul<Tensor<T, U>> for f64
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, U>;

    fn mul(self, mut rhs: Tensor<T, U>) -> Tensor<T, U> {
        for i in 0..(Tensor::<T, U>::get_num_coords()) {
            rhs[i] = rhs[i] * self;
        }
        rhs
    }
}

impl<T, U> Div<f64> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          T::Dimension: Pow<U::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, U>;

    fn div(mut self, rhs: f64) -> Tensor<T, U> {
        for i in 0..(Tensor::<T, U>::get_num_coords()) {
            self[i] = self[i] / rhs;
        }
        self
    }
}

// Tensor multiplication

// For some reason this triggers recursion overflow when tested - to be investigated
impl<T, U, V> Mul<Tensor<T, V>> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          V: Variance,
          U::Rank: ArrayLength<usize>,
          V::Rank: ArrayLength<usize>,
          T::Dimension: Pow<U::Rank> + Pow<V::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>,
          Power<T::Dimension, V::Rank>: ArrayLength<f64>,
          U: Concat<V>,
          Joined<U, V>: Variance,
          T::Dimension: Pow<<Joined<U, V> as Variance>::Rank>,
          Power<T::Dimension, <Joined<U, V> as Variance>::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, Joined<U, V>>;

    fn mul(self, rhs: Tensor<T, V>) -> Tensor<T, Joined<U, V>> {
        assert!(self.p == rhs.p);
        let mut result = Tensor::new(self.p.clone());
        for coord1 in self.iter_coords() {
            for coord2 in rhs.iter_coords() {
                let mut vec_coord1 = coord1.to_vec();
                let mut vec_coord2 = coord2.to_vec();
                vec_coord1.append(&mut vec_coord2);
                let index: &[usize] = &vec_coord1;
                let index1: &[usize] = &coord1;
                let index2: &[usize] = &coord2;
                result[index] = self[index1] * rhs[index2];
            }
        }
        result
    }
}

pub trait InnerProduct<Rhs, Ul: Unsigned, Uh: Unsigned> {
    type Output;

    fn inner_product(self, rhs: Rhs) -> Self::Output;
}

impl<T, U, V, Ul, Uh> InnerProduct<Tensor<T, V>, Ul, Uh> for Tensor<T, U>
    where T: CoordinateSystem,
          U: Variance,
          V: Variance,
          Ul: Unsigned,
          Uh: Unsigned,
          T::Dimension: Pow<U::Rank> + Pow<V::Rank>,
          Power<T::Dimension, U::Rank>: ArrayLength<f64>,
          Power<T::Dimension, V::Rank>: ArrayLength<f64>,
          U: Concat<V>,
          Joined<U,V>: Contract<Ul, Uh>,
          <Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank: ArrayLength<usize>,
          T::Dimension: Pow<<Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank>,
          Power<T::Dimension, <Contracted<Joined<U, V>, Ul, Uh> as Variance>::Rank>: ArrayLength<f64>
{
    type Output = Tensor<T, Contracted<Joined<U, V>, Ul, Uh>>;

    fn inner_product(self, rhs: Tensor<T, V>) -> Tensor<T, Contracted<Joined<U, V>, Ul, Uh>> {
        assert!(self.p == rhs.p);
        let mut result = Tensor::<T, Contracted<Joined<U, V>, Ul, Uh>>::new(self.p.clone());

        for coord_res in result.iter_coords() {
            let mut sum = 0.0;
            for i in 0..T::dimension() {
                let mut coords = coord_res.to_vec();
                coords.insert(Ul::to_usize(), i);
                coords.insert(Uh::to_usize(), i);
                let (coords1, coords2) = coords.split_at(U::Rank::to_usize());
                sum += self[coords1]*rhs[coords2];
            }
            result[&*coord_res] = sum;
        }

        result
    }
}


impl<T, Ul, Ur> Tensor<T, (Ul, Ur)>
    where T: CoordinateSystem,
          Ul: TensorIndex + OtherIndex,
          Ur: TensorIndex + OtherIndex,
          Add1<Ul::Rank>: Unsigned + Add<U1>,
          Add1<Ur::Rank>: Unsigned + Add<U1>,
          Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>: Unsigned + Add<U1>,
          Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>: Unsigned + Add<U1>,
          <(Ul, Ur) as Variance>::Rank: ArrayLength<usize>,
          T::Dimension: Pow<Add1<Ul::Rank>>,
          T::Dimension: Pow<Add1<Ur::Rank>>,
          T::Dimension: Pow<Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>>,
          T::Dimension: Pow<Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>>,
          Power<T::Dimension, Add1<Ul::Rank>>: ArrayLength<f64>,
          Power<T::Dimension, Add1<Ur::Rank>>: ArrayLength<f64>,
          Power<T::Dimension, Add1<<<Ul as OtherIndex>::Output as Variance>::Rank>>: ArrayLength<f64>,
          Power<T::Dimension, Add1<<<Ur as OtherIndex>::Output as Variance>::Rank>>: ArrayLength<f64>
{
    pub fn unit(p: Point<T>) -> Tensor<T, (Ul, Ur)> {
        let mut result = Tensor::<T, (Ul, Ur)>::new(p);

        for i in 0..T::dimension() {
            let coords: &[usize] = &[i,i];
            result[coords] = 1.0;
        }

        result
    }

    pub fn transpose(&self) -> Tensor<T, (Ur, Ul)> {
        let mut result = Tensor::<T, (Ur, Ul)>::new(self.p.clone());

        for coords in self.iter_coords() {
            let coords2: &[usize] = &[coords[1], coords[0]];
            result[coords2] = self[&*coords];
        }

        result
    }

    // TODO
    pub fn inverse(&self) -> Tensor<T, (<Ul as OtherIndex>::Output, <Ur as OtherIndex>::Output)> {
        let mut result = Tensor::<T, (<Ul as OtherIndex>::Output, <Ur as OtherIndex>::Output)>::new(self.p.clone());
        result
    }
}