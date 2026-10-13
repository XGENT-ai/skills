//! Exact review decimals: 0–10, up to 4096 token bytes and 36 fractional digits.
//! Scores serialize as canonical decimal strings; JSON numbers keep their raw token.
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{cmp::Ordering, fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    coefficient: u128,
    scale: u32,
}

impl FromStr for Decimal {
    type Err = &'static str;
    fn from_str(token: &str) -> Result<Self, Self::Err> {
        if token.len() > 4096 {
            return Err("decimal token exceeds the parsing limit");
        }
        let token = token.trim();
        if !token
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_digit() || *c == b'-')
        {
            return Err("expected a finite decimal score");
        }
        serde_json::from_str::<Box<RawValue>>(token).map_err(|_| "invalid decimal token")?;
        let mut parts = token.split(['e', 'E']);
        let mantissa = parts.next().unwrap();
        let exponent = parts
            .next()
            .map(str::parse::<i32>)
            .transpose()
            .map_err(|_| "decimal exponent exceeds the parsing limit")?
            .unwrap_or(0);
        let fraction = mantissa
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        let negative = mantissa.starts_with('-');
        let digits = mantissa.trim_start_matches('-').replace('.', "");
        let significant = digits.trim_start_matches('0');
        if significant.is_empty() {
            return Ok(Self::zero());
        }
        if negative {
            return Err("score must be between 0 and 10");
        }
        let trimmed = significant.trim_end_matches('0');
        let scale =
            fraction as i64 - i64::from(exponent) - (significant.len() - trimmed.len()) as i64;
        if scale > 36 {
            return Err("decimal scale exceeds the parsing limit");
        }
        if scale < -1 {
            return Err("score must be between 0 and 10");
        }
        let mut coefficient = trimmed
            .parse::<u128>()
            .map_err(|_| "decimal coefficient exceeds the parsing limit")?;
        let scale = if scale < 0 {
            coefficient = coefficient
                .checked_mul(10)
                .ok_or("decimal coefficient exceeds the parsing limit")?;
            0
        } else {
            scale as u32
        };
        if coefficient > 10 * 10u128.pow(scale) {
            return Err("score must be between 0 and 10");
        }
        Ok(Self::normalized(coefficient, scale))
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.coefficient.to_string();
        let scale = self.scale as usize;
        if scale == 0 {
            f.write_str(&digits)
        } else if digits.len() <= scale {
            write!(f, "0.{}{digits}", "0".repeat(scale - digits.len()))
        } else {
            let split = digits.len() - scale;
            write!(f, "{}.{}", &digits[..split], &digits[split..])
        }
    }
}

impl Decimal {
    fn zero() -> Self {
        Self {
            coefficient: 0,
            scale: 0,
        }
    }

    fn normalized(mut coefficient: u128, mut scale: u32) -> Self {
        if coefficient == 0 {
            return Self::zero();
        }
        while scale > 0 && coefficient.is_multiple_of(10) {
            coefficient /= 10;
            scale -= 1;
        }
        Self { coefficient, scale }
    }

    fn scaled(self, scale: u32) -> u128 {
        self.coefficient * 10u128.pow(scale - self.scale)
    }

    fn add(self, other: Self) -> Result<Self, &'static str> {
        let scale = self.scale.max(other.scale);
        let coefficient = self.scaled(scale) + other.scaled(scale);
        if coefficient > 10 * 10u128.pow(scale) {
            return Err("score total exceeds 10");
        }
        Ok(Self::normalized(coefficient, scale))
    }

    pub fn growth_less_than(self, earlier: Self, minimum: Self) -> bool {
        let scale = self.scale.max(earlier.scale).max(minimum.scale);
        self.scaled(scale) < earlier.scaled(scale) + minimum.scaled(scale)
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let scale = self.scale.max(other.scale);
        self.scaled(scale).cmp(&other.scaled(scale))
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        let token = raw.get();
        if token.starts_with('"') {
            serde_json::from_str::<String>(token)
                .map_err(D::Error::custom)?
                .parse()
                .map_err(D::Error::custom)
        } else {
            token.parse().map_err(D::Error::custom)
        }
    }
}

pub fn sum_dimensions(scores: [Decimal; 4]) -> Result<Decimal, &'static str> {
    for (score, max) in scores.iter().zip([3, 3, 3, 1]) {
        if *score
            > (Decimal {
                coefficient: max,
                scale: 0,
            })
        {
            return Err("dimension score exceeds its 3/3/3/1 limit");
        }
    }
    scores.into_iter().try_fold(Decimal::zero(), Decimal::add)
}

pub mod scoring;

pub mod policy;

pub mod storage;

pub mod capture;

pub mod projection;

pub mod publication;

pub mod target;

pub mod matrix;

pub mod capture_evidence;

pub mod target_matrix;

pub mod protocol;

pub mod cli;
