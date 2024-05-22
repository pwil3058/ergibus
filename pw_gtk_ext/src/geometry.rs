// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::convert::From;
use std::ops::*;

use gdk_pixbuf;
use gtk;

use num::Num;

use normalised_angles::Angle;

// POINT
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Point(pub f64, pub f64);

use std::f64::consts::SQRT_2;
pub const SIN_45_DEG: f64 = 1.0 / SQRT_2;
pub const COS_45_DEG: f64 = SIN_45_DEG;

impl Point {
    pub fn hypot(&self) -> f64 {
        self.0.hypot(self.1)
    }

    pub fn x(&self) -> f64 {
        self.0
    }

    pub fn y(&self) -> f64 {
        self.1
    }

    pub fn xy(&self) -> (f64, f64) {
        (self.0, self.1)
    }

    pub fn rotate_45_deg(&self) -> Point {
        Point(self.0 - self.1, self.0 + self.1) * SIN_45_DEG
    }
}

impl From<Point> for [f64; 2] {
    fn from(point: Point) -> [f64; 2] {
        [point.0, point.1]
    }
}

impl From<(f64, f64)> for Point {
    fn from(tuple: (f64, f64)) -> Point {
        Point(tuple.0, tuple.1)
    }
}

impl From<(Angle<f64>, f64)> for Point {
    fn from(polar: (Angle<f64>, f64)) -> Point {
        // NB: cairo coordinates are upside down to normal people
        let (angle, radius) = polar;
        Point(radius * angle.cos(), -radius * angle.sin())
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Point {
        Point(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Point) {
        *self = *self + rhs
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, rhs: Point) -> Point {
        Point(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Point) {
        *self = *self - rhs
    }
}

impl<Scalar: Into<f64> + Copy> Mul<Scalar> for Point {
    type Output = Point;

    fn mul(self, rhs: Scalar) -> Point {
        let frhs: f64 = rhs.into();
        Point(self.0 * frhs, self.1 * frhs)
    }
}

impl<Scalar: Into<f64> + Copy> MulAssign<Scalar> for Point {
    fn mul_assign(&mut self, rhs: Scalar) {
        *self = *self * rhs
    }
}

impl<Scalar: Into<f64> + Copy> Div<Scalar> for Point {
    type Output = Point;

    fn div(self, rhs: Scalar) -> Point {
        let frhs: f64 = rhs.into();
        Point(self.0 / frhs, self.1 / frhs)
    }
}

impl<Scalar: Into<f64> + Copy> DivAssign<Scalar> for Point {
    fn div_assign(&mut self, rhs: Scalar) {
        *self = *self / rhs
    }
}

pub type Points = Vec<Point>;

// SIZE
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Size<T: Num + PartialOrd + Copy> {
    pub width: T,
    pub height: T,
}

impl<T: Num + PartialOrd + Copy> Sub for Size<T> {
    type Output = Size<T>;

    fn sub(self, rhs: Size<T>) -> Size<T> {
        Size::<T> {
            width: self.width - rhs.width,
            height: self.height - rhs.width,
        }
    }
}

impl<T: Num + PartialOrd + Copy> Add for Size<T> {
    type Output = Size<T>;

    fn add(self, rhs: Size<T>) -> Size<T> {
        Size::<T> {
            width: self.width + rhs.width,
            height: self.height + rhs.width,
        }
    }
}

impl<T: Num + PartialOrd + Copy> Size<T> {
    pub fn length_longest_side(&self) -> T {
        if self.width > self.height {
            self.width
        } else {
            self.height
        }
    }
}

impl From<Size<f64>> for [f64; 2] {
    fn from(size: Size<f64>) -> [f64; 2] {
        [size.width, size.height]
    }
}

impl From<Size<i32>> for [i32; 2] {
    fn from(size: Size<i32>) -> [i32; 2] {
        [size.width, size.height]
    }
}

impl From<Size<i32>> for Size<f64> {
    fn from(size_i32: Size<i32>) -> Size<f64> {
        Size::<f64> {
            width: size_i32.width as f64,
            height: size_i32.height as f64,
        }
    }
}

impl From<Size<f64>> for Size<i32> {
    fn from(size_f64: Size<f64>) -> Size<i32> {
        Size::<i32> {
            width: size_f64.width.round() as i32,
            height: size_f64.height.round() as i32,
        }
    }
}

impl Mul<f64> for Size<f64> {
    type Output = Size<f64>;

    fn mul(self, rhs: f64) -> Size<f64> {
        Size::<f64> {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl Mul<f64> for Size<i32> {
    type Output = Size<i32>;

    fn mul(self, rhs: f64) -> Size<i32> {
        (Size::<f64>::from(self) * rhs).into()
    }
}

impl Size<f64> {
    pub fn scales_versus(&self, other: Size<f64>) -> Size<f64> {
        Size::<f64> {
            width: self.width / other.width,
            height: self.height / other.height,
        }
    }

    pub fn scale_versus(&self, other: Size<f64>) -> f64 {
        let scales = self.scales_versus(other);
        if self.aspect_ratio_matches_size(other) {
            (scales.width + scales.height) / 2.0
        } else {
            scales.width.max(scales.height)
        }
    }
}

impl Size<i32> {
    pub fn scales_versus(&self, other: Size<f64>) -> Size<f64> {
        Size::<f64> {
            width: self.width as f64 / other.width,
            height: self.height as f64 / other.height,
        }
    }

    pub fn scale_versus(&self, other: Size<f64>) -> f64 {
        assert!(self.aspect_ratio_matches_size(other));
        let scales = self.scales_versus(other);
        (scales.width + scales.height) / 2.0
    }
}

pub trait SizeExt {
    fn size(&self) -> Size<i32>;

    fn length_longest_side(&self) -> i32 {
        self.size().length_longest_side()
    }

    fn scales_versus<S: SizeExt>(&self, other: &S) -> Size<f64> {
        let other_size: Size<f64> = other.size().into();
        let my_size: Size<f64> = self.size().into();
        my_size.scales_versus(other_size)
    }

    fn scale_versus<S: SizeExt>(&self, other: &S) -> f64 {
        let other_size: Size<f64> = other.size().into();
        let my_size: Size<f64> = self.size().into();
        my_size.scale_versus(other_size)
    }
}

impl SizeExt for gdk_pixbuf::Pixbuf {
    fn size(&self) -> Size<i32> {
        Size::<i32> {
            width: self.get_width(),
            height: self.get_height(),
        }
    }
}

impl SizeExt for gtk::Rectangle {
    fn size(&self) -> Size<i32> {
        Size::<i32> {
            width: self.width,
            height: self.height,
        }
    }
}

pub trait AspectRatio {
    fn aspect_ratio(&self) -> f64;

    fn aspect_ratio_matches_size(&self, size: Size<f64>) -> bool {
        if size.width < size.height {
            (size.height * self.aspect_ratio()).round() == size.width
        } else {
            (size.width / self.aspect_ratio()).round() == size.height
        }
    }

    fn aspect_ratio_matches<S: SizeExt>(&self, other: &S) -> bool {
        let size: Size<f64> = other.size().into();
        self.aspect_ratio_matches_size(size)
    }
}

impl AspectRatio for gdk_pixbuf::Pixbuf {
    fn aspect_ratio(&self) -> f64 {
        self.get_width() as f64 / self.get_height() as f64
    }
}

impl AspectRatio for gtk::Rectangle {
    fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
}

impl AspectRatio for Size<i32> {
    fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
}

impl AspectRatio for Size<f64> {
    fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }
}

// RECTANGLE
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Rectangle<T: Num + PartialOrd + Copy> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl From<Rectangle<i32>> for Rectangle<f64> {
    fn from(rectangle: Rectangle<i32>) -> Rectangle<f64> {
        Rectangle::<f64> {
            x: rectangle.x as f64,
            y: rectangle.y as f64,
            width: rectangle.width as f64,
            height: rectangle.height as f64,
        }
    }
}

impl From<Rectangle<f64>> for Rectangle<i32> {
    fn from(rectangle: Rectangle<f64>) -> Rectangle<i32> {
        Rectangle::<i32> {
            x: rectangle.x.round() as i32,
            y: rectangle.y.round() as i32,
            width: rectangle.width.round() as i32,
            height: rectangle.height.round() as i32,
        }
    }
}

impl From<gtk::Rectangle> for Rectangle<f64> {
    fn from(rectangle: gtk::Rectangle) -> Rectangle<f64> {
        Rectangle::<f64> {
            x: rectangle.x as f64,
            y: rectangle.y as f64,
            width: rectangle.width as f64,
            height: rectangle.height as f64,
        }
    }
}

impl From<Rectangle<f64>> for gtk::Rectangle {
    fn from(rectangle: Rectangle<f64>) -> gtk::Rectangle {
        gtk::Rectangle {
            x: rectangle.x.round() as i32,
            y: rectangle.y.round() as i32,
            width: rectangle.width.round() as i32,
            height: rectangle.height.round() as i32,
        }
    }
}

impl<T: Num + PartialOrd + Copy> Rectangle<T> {
    pub fn size(&self) -> Size<T> {
        Size::<T> {
            width: self.width,
            height: self.height,
        }
    }
}
