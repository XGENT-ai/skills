/// `Math.round`: the nearest integer, ties toward +∞, keeping the sign of zero.
pub fn js_round(value: f64) -> f64 {
    if !value.is_finite() || value.fract() == 0.0 {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    };
    if rounded == 0.0 && value < 0.0 {
        -0.0
    } else {
        rounded
    }
}

/// Decimal digits (no leading or trailing zeros except for zero itself) and
/// the position of the decimal point: value = 0.d1d2… × 10^point.
struct Decimal {
    digits: Vec<u8>,
    point: i32,
}

impl Decimal {
    // Rust's exponent formatting yields digits and a power of ten; precision
    // `None` asks for the shortest digits that round-trip, as V8 and ICU do.
    fn of(value: f64, precision: Option<usize>) -> Self {
        let text = match precision {
            Some(precision) => format!("{:.*e}", precision, value.abs()),
            None => format!("{:e}", value.abs()),
        };
        let (mantissa, exponent) = text.split_once('e').expect("exponent notation");
        let mut digits: Vec<u8> = mantissa
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(|digit| digit - b'0')
            .collect();
        while digits.len() > 1 && digits.last() == Some(&0) {
            digits.pop();
        }
        let exponent: i32 = exponent.parse().expect("integer exponent");
        Self {
            point: if digits == [0] { 1 } else { exponent + 1 },
            digits,
        }
    }

    fn is_zero(&self) -> bool {
        self.digits.iter().all(|digit| *digit == 0)
    }

    /// Keep `count` leading digits, rounding up when the first dropped digit
    /// is 5 or more. On an exact or shortest decimal this is half-up/half-expand.
    fn round_digits(&mut self, count: usize) {
        if self.digits.len() <= count {
            return;
        }
        let up = self.digits[count] >= 5;
        self.digits.truncate(count);
        if up {
            let mut index = count;
            loop {
                if index == 0 {
                    self.digits.insert(0, 1);
                    self.point += 1;
                    break;
                }
                index -= 1;
                if self.digits[index] == 9 {
                    self.digits[index] = 0;
                } else {
                    self.digits[index] += 1;
                    break;
                }
            }
        }
        while self.digits.len() > 1 && self.digits.last() == Some(&0) {
            self.digits.pop();
        }
        if self.digits.is_empty() {
            self.digits.push(0);
        }
    }

    /// Round to `fraction` digits after the decimal point.
    fn round_fraction(&mut self, fraction: i32) {
        let keep = self.point + fraction;
        if keep < 0 {
            self.digits = vec![0];
            self.point = 1;
        } else if keep == 0 {
            let up = self.digits.first().is_some_and(|digit| *digit >= 5);
            self.digits = vec![u8::from(up)];
            self.point = if up { self.point + 1 } else { 1 };
        } else {
            self.round_digits(keep as usize);
        }
        if self.is_zero() {
            self.digits = vec![0];
            self.point = 1;
        }
    }

    fn digit_text(&self) -> String {
        self.digits
            .iter()
            .map(|digit| char::from(b'0' + digit))
            .collect()
    }

    /// Plain positional notation with en-US thousands separators.
    fn grouped(&self) -> String {
        let digits = self.digit_text();
        let (integer, fraction) = if self.point <= 0 {
            (
                "0".to_owned(),
                format!("{}{digits}", "0".repeat((-self.point) as usize)),
            )
        } else if self.point as usize >= digits.len() {
            (
                format!("{digits}{}", "0".repeat(self.point as usize - digits.len())),
                String::new(),
            )
        } else {
            let (integer, fraction) = digits.split_at(self.point as usize);
            (integer.to_owned(), fraction.to_owned())
        };
        let mut result = String::new();
        for (index, digit) in integer.chars().enumerate() {
            if index > 0 && (integer.len() - index) % 3 == 0 {
                result.push(',');
            }
            result.push(digit);
        }
        let fraction = fraction.trim_end_matches('0');
        if !fraction.is_empty() {
            result.push('.');
            result.push_str(fraction);
        }
        result
    }
}

fn sign(value: f64) -> &'static str {
    if value.is_sign_negative() {
        "-"
    } else {
        ""
    }
}

/// ECMAScript `Number::toString(10)` for finite values.
pub fn js_to_string(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".into()
        } else {
            format!("{}Infinity", sign(value))
        };
    }
    let decimal = Decimal::of(value, None);
    let (digits, k, n) = (
        decimal.digit_text(),
        decimal.digits.len() as i32,
        decimal.point,
    );
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let exponent = n - 1;
        let mantissa = if k == 1 {
            digits
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!(
            "{mantissa}e{}{}",
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        )
    };
    format!("{}{body}", if value < 0.0 { "-" } else { "" })
}

/// ECMAScript `Number.prototype.toPrecision` (precision 1–100) for finite
/// values: the exact binary value rounded half-up to `precision` digits.
pub fn js_to_precision(value: f64, precision: u32) -> String {
    let precision = precision.clamp(1, 100) as usize;
    if !value.is_finite() {
        return js_to_string(value);
    }
    let prefix = if value < 0.0 { "-" } else { "" };
    let (digits, exponent) = if value == 0.0 {
        ("0".repeat(precision), 0)
    } else {
        // Every finite double has an exact decimal expansion within 800 digits.
        let mut decimal = Decimal::of(value, Some(800));
        decimal.round_digits(precision);
        let mut digits = decimal.digit_text();
        digits.push_str(&"0".repeat(precision - digits.len()));
        (digits, decimal.point - 1)
    };
    let p = precision as i32;
    if exponent < -6 || exponent >= p {
        let mantissa = if precision == 1 {
            digits
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        return format!(
            "{prefix}{mantissa}e{}{}",
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        );
    }
    if exponent == p - 1 {
        return format!("{prefix}{digits}");
    }
    if exponent >= 0 {
        let split = exponent as usize + 1;
        return format!("{prefix}{}.{}", &digits[..split], &digits[split..]);
    }
    format!(
        "{prefix}0.{}{digits}",
        "0".repeat((-(exponent + 1)) as usize)
    )
}

/// `toLocaleString('en-US', { maximumFractionDigits })`: ICU rounds the
/// shortest decimal half-expand, drops trailing zeros and groups thousands.
pub fn en_us_max_fraction(value: f64, digits: u32) -> String {
    if !value.is_finite() {
        return locale_non_finite(value);
    }
    let mut decimal = Decimal::of(value, None);
    decimal.round_fraction(digits as i32);
    format!("{}{}", sign(value), decimal.grouped())
}

/// `toLocaleString('en-US', { maximumSignificantDigits })` on the shortest
/// decimal, half-expand, in plain grouped notation.
pub fn en_us_max_significant(value: f64, digits: u32) -> String {
    if !value.is_finite() {
        return locale_non_finite(value);
    }
    let mut decimal = Decimal::of(value, None);
    decimal.round_digits(digits.max(1) as usize);
    format!("{}{}", sign(value), decimal.grouped())
}

fn locale_non_finite(value: f64) -> String {
    if value.is_nan() {
        "NaN".into()
    } else {
        format!("{}∞", sign(value))
    }
}
