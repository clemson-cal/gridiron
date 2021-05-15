use std::ops::{Add, Sub, Mul, Div};
use super::error::Error;
use super::geometry::{Direction, Vector3d};




// ============================================================================
pub struct Conserved(f64, f64, f64, f64, f64);
pub struct Primitive(f64, f64, f64, f64, f64);




// ============================================================================
impl Conserved {

    fn from_slice(cons: &[f64]) -> Self {
        Self(cons[0], cons[1], cons[2], cons[3], cons[4])
    }

    pub fn write_to_slice(&self, cons: &mut [f64]) {
        cons[0] = self.0;
        cons[1] = self.1;
        cons[2] = self.2;
        cons[3] = self.3;
        cons[4] = self.4;
    }

    pub fn as_array(&self) -> [f64; 5] {
        [self.0, self.1, self.2, self.3, self.4]
    }

    pub fn mass_density(&self) -> f64 {
        self.0
    }

    pub fn momentum_1(&self) -> f64 {
        self.1
    }

    pub fn momentum_2(&self) -> f64 {
        self.2
    }

    pub fn momentum_3(&self) -> f64 {
        self.3
    }

    pub fn energy_density(&self) -> f64 {
        self.4
    }

    pub fn momentum_vector(&self)  -> Vector3d {
        Vector3d::new(self.momentum_1(), self.momentum_2(), self.momentum_3())
    }

    pub fn momentum(&self, direction: Direction) -> f64 {
        match direction {
            Direction::I => self.momentum_1(),
            Direction::J => self.momentum_2(),
            Direction::K => self.momentum_3(),
        }
    }

    pub fn momentum_squared(&self) -> f64 {
        self.1 * self.1 + self.2 * self.2 + self.3 * self.3
    }

    pub fn to_primitive(&self, gamma_law_index: f64) -> Result<Primitive, Error> {
        let ek = 0.5 * self.momentum_squared() / self.mass_density();
        let et = self.energy_density() - ek;
        let pg = et * (gamma_law_index - 1.0);
        let v1 = self.momentum_1() / self.mass_density();
        let v2 = self.momentum_2() / self.mass_density();
        let v3 = self.momentum_3() / self.mass_density();

        if self.mass_density() < 0.0 {
            Err(Error::NegativeMassDensity(self.mass_density()))
        } else if pg < 0.0 {
            Err(Error::NegativeGasPressure(pg))
        } else {
            Ok(Primitive(self.mass_density(), v1, v2, v3, pg))
        }
    }
}




// ============================================================================
impl Primitive {

    fn from_slice(prim: &[f64]) -> Self {
        Self(prim[0], prim[1], prim[2], prim[3], prim[4])
    }

    pub fn write_to_slice(&self, prim: &mut [f64]) {
        prim[0] = self.0;
        prim[1] = self.1;
        prim[2] = self.2;
        prim[3] = self.3;
        prim[4] = self.4;
    }

    pub fn new(d0: f64, u0: f64, v0: f64, w0: f64, e0: f64) -> Self {
        Self(d0, u0, v0, w0, e0)
    }

    pub fn as_array(&self) -> [f64; 5] {
        [self.0, self.1, self.2, self.3, self.4]
    }

    pub fn mass_density(&self) -> f64 {
        self.0
    }

    pub fn velocity_1(&self) -> f64 {
        self.1
    }

    pub fn velocity_2(&self) -> f64 {
        self.2
    }

    pub fn velocity_3(&self) -> f64 {
        self.3
    }

    pub fn gas_pressure(&self) -> f64 {
        self.4
    }

    pub fn velocity(&self, direction: Direction) -> f64 {
        match direction {
            Direction::I => self.velocity_1(),
            Direction::J => self.velocity_2(),
            Direction::K => self.velocity_3(),
        }
    }

    pub fn velocity_squared(&self) -> f64 {
        self.1 * self.1 + self.2 * self.2 + self.3 * self.3
    }

    pub fn sound_speed_squared(&self, gamma_law_index: f64) -> f64 {
        gamma_law_index * self.gas_pressure() / self.mass_density()
    }

    pub fn specific_kinetic_energy(&self) -> f64 {
        0.5 * self.velocity_squared()
    }

    pub fn specific_internal_energy(&self, gamma_law_index: f64) -> f64 {
        self.gas_pressure() / self.mass_density() / (gamma_law_index - 1.0)
    }

    pub fn mach_number(&self, gamma_law_index: f64) -> f64 {
        (self.velocity_squared() / self.sound_speed_squared(gamma_law_index)).sqrt()
    }

    pub fn outer_wavespeeds(&self, direction: Direction, gamma_law_index: f64) -> (f64, f64) {
        let cs = self.sound_speed_squared(gamma_law_index).sqrt();
        let vn = self.velocity(direction);
        (vn - cs, vn + cs)
    }

    pub fn max_signal_speed(&self, gamma_law_index: f64) -> f64 {
        f64::sqrt(self.velocity_squared()) + f64::sqrt(self.sound_speed_squared(gamma_law_index))
    }

    pub fn to_conserved(&self, gamma_law_index: f64) -> Conserved {
        let d   = self.mass_density();
        let p   = self.gas_pressure();
        let vsq = self.velocity_squared();

        Conserved(
            d,
            d * self.velocity_1(),
            d * self.velocity_2(),
            d * self.velocity_3(),
            d * vsq * 0.5 + p / (gamma_law_index - 1.0)
        )
    }

    pub fn flux_vector(&self, direction: Direction, gamma_law_index: f64) -> Conserved {
        let pg = self.gas_pressure();
        let vn = self.velocity(direction);
        let u = self.to_conserved(gamma_law_index);

        Conserved(
             u.0 * vn,
             u.1 * vn + pg * direction.along(Direction::I),
             u.2 * vn + pg * direction.along(Direction::J),
             u.3 * vn + pg * direction.along(Direction::K),
             u.4 * vn + pg * vn)
    }

    pub fn reflect(&self, direction: Direction) -> Primitive {
        match direction {
            Direction::I => Primitive(self.0, -self.1, self.2, self.3, self.4),
            Direction::J => Primitive(self.0, self.1, -self.2, self.3, self.4),
            Direction::K => Primitive(self.0, self.1, self.2, -self.3, self.4),
        }
    }
}




// ============================================================================
impl From<&[f64]> for Conserved {
    fn from(prim: &[f64]) -> Self {
        Self::from_slice(prim)
    }
}

impl From<&[f64]> for Primitive {
    fn from(prim: &[f64]) -> Self {
        Self::from_slice(prim)
    }
}




// ============================================================================
impl Add<Conserved> for Conserved {
    type Output = Conserved;
    fn add(self, u: Self) -> Conserved {
        Conserved(self.0 + u.0, self.1 + u.1, self.2 + u.2, self.3 + u.3, self.4 + u.4)
    }
}

impl Sub<Conserved> for Conserved {
    type Output = Self;
    fn sub(self, u: Self) -> Self {
        Self(self.0 - u.0, self.1 - u.1, self.2 - u.2, self.3 - u.3, self.4 - u.4)
    }
}

impl Mul<f64> for Conserved {
    type Output = Self;
    fn mul(self, a: f64) -> Self {
        Self(self.0 * a, self.1 * a, self.2 * a, self.3 * a, self.4 * a)
    }
}

impl Div<f64> for Conserved {
    type Output = Self;
    fn div(self, a: f64) -> Self {
        Self(self.0 / a, self.1 / a, self.2 / a, self.3 / a, self.4 / a)
    }
}




// ============================================================================
pub fn riemann_hlle(pl: Primitive, pr: Primitive, direction: Direction, gamma_law_index: f64) -> Conserved {
    let ul = pl.to_conserved(gamma_law_index);
    let ur = pr.to_conserved(gamma_law_index);
    let fl = pl.flux_vector(direction, gamma_law_index);
    let fr = pr.flux_vector(direction, gamma_law_index);

    let (alm, alp) = pl.outer_wavespeeds(direction, gamma_law_index);
    let (arm, arp) = pr.outer_wavespeeds(direction, gamma_law_index);
    let ap = alp.max(arp).max(0.0);
    let am = alm.min(arm).min(0.0);

    (fl * ap - fr * am - (ul - ur) * ap * am) / (ap - am)
}
