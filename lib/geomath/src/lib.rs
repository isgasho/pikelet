extern crate cgmath;
extern crate rand;

use cgmath::prelude::*;
use cgmath::BaseFloat;
use cgmath::{Point3, Quaternion, Rad, Vector3};
use rand::{Rand, Rng};
use rand::distributions::range::SampleRange;
use std::iter;
use std::ops::*;

/// A location on a unit sphere, described using latitude and longitude.
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct LatLong<A: Angle> {
    pub lat: A,
    pub long: A,
}

impl<A: Angle> From<GeoPoint<A::Unitless>> for LatLong<A> {
    #[inline]
    fn from(src: GeoPoint<A::Unitless>) -> LatLong<A> {
        // From https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
        LatLong {
            lat: A::atan(src.up.y / src.up.x),
            // Probably don't need `A::acos(src.up.z / src.up.magnitude())` because
            // `src.0` is a unit vector, barring rounding errors
            long: A::acos(src.up.z),
        }
    }
}

impl<A: Angle> Rand for LatLong<A>
where
    A::Unitless: BaseFloat + Rand + SampleRange,
{
    #[inline]
    fn rand<R: Rng>(rng: &mut R) -> LatLong<A> {
        // From http://mathworld.wolfram.com/SpherePointPicking.html

        let u = rng.gen_range(A::Unitless::zero(), A::Unitless::one());
        let v = rng.gen_range(A::Unitless::zero(), A::Unitless::one());

        LatLong {
            lat: A::acos((v + v) - A::Unitless::one()),
            long: A::full_turn() * u,
        }
    }
}

impl<A: Angle> From<LatLong<A>> for GeoPoint<A::Unitless> {
    #[inline]
    fn from(src: LatLong<A>) -> GeoPoint<A::Unitless> {
        // From https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
        let sin_lat = A::sin(src.lat);
        GeoPoint {
            up: Vector3 {
                x: sin_lat * A::cos(src.long),
                y: sin_lat * A::sin(src.long),
                z: A::cos(src.lat),
            },
        }
    }
}

#[inline]
pub fn arc_length<T: BaseFloat>(angle: Rad<T>, radius: T) -> Rad<T> {
    angle * radius
}

/// A point on the surface of a sphere.
///
/// This uses an underlying vector representation to reduce the amount of
/// expensive trigonometry needed and also to avoid problems at the poles.
///
/// # References
///
/// - http://www.movable-type.co.uk/scripts/latlong-vectors.html
/// - http://www.navlab.net/Publications/A_Nonsingular_Horizontal_Position_Representation.pdf
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GeoPoint<T> {
    /// The normalized up vector with respect to the origin
    up: Vector3<T>,
}

impl<T: BaseFloat> GeoPoint<T> {
    #[inline]
    pub fn from_up(up: Vector3<T>) -> GeoPoint<T> {
        GeoPoint {
            up: Vector3::normalize(up),
        }
    }

    #[inline]
    pub fn north() -> GeoPoint<T> {
        GeoPoint {
            up: Vector3::unit_x(),
        }
    }

    #[inline]
    pub fn south() -> GeoPoint<T> {
        GeoPoint::north().antipode()
    }

    #[inline]
    pub fn up(self) -> Vector3<T> {
        self.up
    }

    #[inline]
    pub fn midpoint(self, other: GeoPoint<T>) -> GeoPoint<T> {
        GeoPoint {
            up: Vector3::normalize(self.up + other.up),
        }
    }

    #[inline]
    pub fn antipode(self) -> GeoPoint<T> {
        GeoPoint { up: -self.up }
    }

    #[inline]
    pub fn distance(self, other: GeoPoint<T>) -> Rad<T> {
        Vector3::angle(self.up, other.up)
    }

    #[inline]
    pub fn to_point(self, radius: T) -> Point3<T> {
        Point3::from_vec(self.up) * radius
    }
}

impl<T: BaseFloat> Add<GeoVector<T>> for GeoPoint<T> {
    type Output = GeoPoint<T>;

    #[inline]
    fn add(self, other: GeoVector<T>) -> GeoPoint<T> {
        GeoPoint::from_up(other.rotation * self.up)
    }
}

impl<T: BaseFloat> Sub for GeoPoint<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn sub(self, other: GeoPoint<T>) -> GeoVector<T> {
        GeoVector {
            rotation: Quaternion::from_arc(other.up, self.up, None),
        }
    }
}

impl<T: BaseFloat> Rand for GeoPoint<T>
where
    T: Rand + SampleRange,
{
    #[inline]
    fn rand<R: Rng>(rng: &mut R) -> GeoPoint<T> {
        GeoPoint::from(LatLong::<Rad<T>>::rand(rng))
    }
}

/// A tangent vector on the unit sphere
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GeoVector<T> {
    rotation: Quaternion<T>,
}

impl<T: BaseFloat> GeoVector<T> {
    #[inline]
    pub fn rotation(self) -> Quaternion<T> {
        self.rotation
    }
}

impl<T: BaseFloat> Add for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn add(self, other: GeoVector<T>) -> GeoVector<T> {
        GeoVector {
            rotation: self.rotation + other.rotation,
        }
    }
}

impl<T: BaseFloat> Sub for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn sub(self, other: GeoVector<T>) -> GeoVector<T> {
        GeoVector {
            rotation: self.rotation - other.rotation,
        }
    }
}

impl<T: BaseFloat> Neg for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn neg(self) -> GeoVector<T> {
        GeoVector {
            rotation: -self.rotation,
        }
    }
}

impl<T: BaseFloat> Mul<T> for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn mul(self, other: T) -> GeoVector<T> {
        GeoVector {
            rotation: self.rotation * other,
        }
    }
}

impl<T: BaseFloat> Div<T> for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn div(self, other: T) -> GeoVector<T> {
        GeoVector {
            rotation: self.rotation / other,
        }
    }
}

impl<T: BaseFloat> Rem<T> for GeoVector<T> {
    type Output = GeoVector<T>;

    #[inline]
    fn rem(self, other: T) -> GeoVector<T> {
        GeoVector {
            rotation: self.rotation % other,
        }
    }
}

impl<T: BaseFloat> Zero for GeoVector<T> {
    #[inline]
    fn is_zero(&self) -> bool {
        self.rotation.is_zero()
    }

    #[inline]
    fn zero() -> GeoVector<T> {
        GeoVector {
            rotation: Quaternion::<T>::zero(),
        }
    }
}

impl<S: BaseFloat> iter::Sum<GeoVector<S>> for GeoVector<S> {
    #[inline]
    fn sum<I: Iterator<Item = GeoVector<S>>>(iter: I) -> GeoVector<S> {
        iter.fold(GeoVector::zero(), Add::add)
    }
}

impl<T: BaseFloat> VectorSpace for GeoVector<T> {
    type Scalar = T;
}

/// A great circle on a sphere.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GreatCircle<T> {
    /// The normal vector of the great-circle plane.
    normal: Vector3<T>,
}

impl<T: BaseFloat> GreatCircle<T> {
    /// Construct a great-circle from two points on a sphere. Note that this
    /// will result in an invalid value if the points are on opposite sides
    /// of the sphere.
    #[inline]
    pub fn from_points(a: GeoPoint<T>, b: GeoPoint<T>) -> GreatCircle<T> {
        GreatCircle {
            normal: Vector3::cross(a.up, b.up).normalize(),
        }
    }

    /// Construct a great-circle from a points on a sphere and a direction.
    #[inline]
    pub fn from_point_vector(a: GeoPoint<T>, direction: GeoVector<T>) -> GreatCircle<T> {
        GreatCircle::from_points(a, a + direction)
    }

    /// The normal vector of the great-circle plane.
    #[inline]
    pub fn normal(self) -> Vector3<T> {
        self.normal
    }
}
