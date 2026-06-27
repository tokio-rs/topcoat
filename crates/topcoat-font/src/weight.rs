#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(u8);

impl FontWeight {
    #[must_use]
    pub const fn new(weight: u8) -> Self {
        Self(weight)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self(400)
    }
}

pub struct FontWeightOutOfRangeError;

impl TryFrom<u8> for FontWeight {
    type Error = FontWeightOutOfRangeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 100 || value > 900 {
            return Err(FontWeightOutOfRangeError);
        }
        Ok(Self(value))
    }
}

impl From<FontWeight> for u8 {
    fn from(value: FontWeight) -> Self {
        value.0
    }
}

impl std::fmt::Display for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeightRange {
    start: FontWeight,
    end: FontWeight,
}

impl FontWeightRange {
    #[must_use]
    pub const fn new(start: FontWeight, end: FontWeight) -> Self {
        assert!(end.0 >= start.0, "font weight range must not be empty");
        Self { start, end }
    }

    #[must_use]
    pub const fn from_u8(start: u8, end: u8) -> Self {
        Self::new(FontWeight::new(start), FontWeight::new(end))
    }

    #[must_use]
    pub const fn start(&self) -> FontWeight {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> FontWeight {
        self.end
    }
}

impl std::fmt::Display for FontWeightRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            self.start.fmt(f)
        } else {
            write!(f, "{} {}", self.start, self.end)
        }
    }
}
