use derive_more::{Add, AddAssign, Neg, Sub, SubAssign};
use std::ops::{Div, Mul};

#[derive(Debug, Clone, Copy, Add, AddAssign, Sub, SubAssign, Neg)]
pub struct Multivector {
    pub s: f32,
    pub e0: f32,
    pub e1: f32,
    pub e2: f32,
    pub e01: f32,
    pub e02: f32,
    pub e12: f32,
    pub e012: f32,
}

impl Multivector {
    pub const ZERO: Self = Self {
        s: 0.0,
        e0: 0.0,
        e1: 0.0,
        e2: 0.0,
        e01: 0.0,
        e02: 0.0,
        e12: 0.0,
        e012: 0.0,
    };

    pub fn grade0(self) -> Self {
        Self {
            s: self.s,
            ..Self::ZERO
        }
    }

    pub fn grade1(self) -> Self {
        Self {
            e0: self.e0,
            e1: self.e1,
            e2: self.e2,
            ..Self::ZERO
        }
    }

    pub fn grade2(self) -> Self {
        Self {
            e01: self.e01,
            e02: self.e02,
            e12: self.e12,
            ..Self::ZERO
        }
    }

    pub fn grade3(self) -> Self {
        Self {
            e012: self.e012,
            ..Self::ZERO
        }
    }

    pub fn grade(self, grade: usize) -> Multivector {
        match grade {
            0 => self.grade0(),
            1 => self.grade1(),
            2 => self.grade2(),
            3 => self.grade3(),
            _ => Self::ZERO,
        }
    }

    pub fn wedge(self, other: Self) -> Self {
        let mut result = Self::ZERO;
        for j in 0..=3 {
            for k in 0..=3 {
                result += (self.grade(j) * other.grade(k)).grade(j + k);
            }
        }
        result
    }

    pub fn inner(self, other: Self) -> Self {
        let mut result = Self::ZERO;
        for j in 0..=3 {
            for k in 0..=3 {
                result += (self.grade(j) * other.grade(k)).grade(j.abs_diff(k));
            }
        }
        result
    }

    pub fn regressive(self, other: Self) -> Self {
        self.dual().wedge(other.dual()).dual_inverse()
    }

    pub fn reverse(self) -> Self {
        let Self {
            s,
            e0,
            e1,
            e2,
            e01,
            e02,
            e12,
            e012,
        } = self;
        Self {
            s,
            e0,
            e1,
            e2,
            e01: -e01,
            e02: -e02,
            e12: -e12,
            e012: -e012,
        }
    }

    pub fn dual(self) -> Self {
        let Self {
            s,
            e0,
            e1,
            e2,
            e01,
            e02,
            e12,
            e012,
        } = self;
        Self {
            s: e012,
            e0: e12,
            e1: -e02,
            e2: e01,
            e01: e2,
            e02: -e1,
            e12: e0,
            e012: s,
        }
    }

    pub fn dual_inverse(self) -> Self {
        let Self {
            s,
            e0,
            e1,
            e2,
            e01,
            e02,
            e12,
            e012,
        } = self;
        Self {
            s: e012,
            e0: e12,
            e1: -e02,
            e2: e01,
            e01: e2,
            e02: -e1,
            e12: e0,
            e012: s,
        }
    }

    pub fn sqr_magnitude(self) -> f32 {
        (self * self.reverse()).s
    }

    pub fn magnitude(self) -> f32 {
        self.sqr_magnitude().sqrt()
    }

    pub fn normalised(self) -> Self {
        let magnitude = self.magnitude();
        if magnitude >= 0.0001 {
            self / magnitude
        } else {
            self
        }
    }
}

impl Mul<Multivector> for Multivector {
    type Output = Self;

    #[rustfmt::skip]
    #[allow(clippy::just_underscores_and_digits)]
    fn mul(self, other: Self) -> Self::Output {
        let Self {
            s: _0,
            e0: _1,
            e1: _2,
            e2: _3,
            e01: _4,
            e02: _5,
            e12: _6,
            e012: _7,
        } = self;
        let Self {
            s: _8,
            e0: _9,
            e1: _10,
            e2: _11,
            e01: _12,
            e02: _13,
            e12: _14,
            e012: _15,
        } = other;
        Self {
            s: ((((_0 * _8) + (_10 * _2)) + (_11 * _3)) + -(_14 * _6)),
            e0: ((((((((_0 * _9) + (_1 * _8)) + -(_12 * _2)) + -(_13 * _3)) + (_10 * _4)) + (_11 * _5)) + -(_15 * _6)) + -(_14 * _7)),
            e1: ((((_0 * _10) + (_2 * _8)) + -(_14 * _3)) + (_11 * _6)),
            e2: ((((_0 * _11) + (_14 * _2)) + (_3 * _8)) + -(_10 * _6)),
            e01: ((((((((_0 * _12) + (_1 * _10)) + -(_2 * _9)) + (_15 * _3)) + (_4 * _8)) + -(_14 * _5)) + (_13 * _6)) + (_11 * _7)),
            e02: ((((((((_0 * _13) + (_1 * _11)) + -(_15 * _2)) + -(_3 * _9)) + (_14 * _4)) + (_5 * _8)) + -(_12 * _6)) + -(_10 * _7)),
            e12: ((((_0 * _14) + (_11 * _2)) + -(_10 * _3)) + (_6 * _8)),
            e012: ((((((((_0 * _15) + (_1 * _14)) + -(_13 * _2)) + (_12 * _3)) + (_11 * _4)) + -(_10 * _5)) + (_6 * _9)) + (_7 * _8)),
        }
    }
}

impl Mul<f32> for Multivector {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        let Self {
            s,
            e0,
            e1,
            e2,
            e01,
            e02,
            e12,
            e012,
        } = self;
        Self {
            s: s * other,
            e0: e0 * other,
            e1: e1 * other,
            e2: e2 * other,
            e01: e01 * other,
            e02: e02 * other,
            e12: e12 * other,
            e012: e012 * other,
        }
    }
}

impl Div<f32> for Multivector {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, other: f32) -> Self::Output {
        self * other.recip()
    }
}
