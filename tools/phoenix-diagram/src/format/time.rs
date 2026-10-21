use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Offset, TimeZone, Timelike};
use chrono_tz::{Tz, TZ_VARIANTS};

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// A display zone as ICU accepts it: a named zone or a fixed UTC offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zone {
    Named(Tz),
    Fixed(FixedOffset),
}

// ICU 77.1 accepts these identifiers although tzdata does not list them. Each
// target is the zone ICU resolves them to, or a zone with identical offsets.
// The SystemV daylight-saving zones carry ICU-only rules and stay unsupported.
const ICU_ALIASES: [(&str, &str); 33] = [
    ("ACT", "Australia/Darwin"),
    ("AET", "Australia/Sydney"),
    ("AGT", "America/Buenos_Aires"),
    ("ART", "Africa/Cairo"),
    ("AST", "America/Anchorage"),
    ("BET", "America/Sao_Paulo"),
    ("BST", "Asia/Dhaka"),
    ("CAT", "Africa/Maputo"),
    ("CNT", "America/St_Johns"),
    ("CST", "America/Chicago"),
    ("CTT", "Asia/Shanghai"),
    ("EAT", "Africa/Nairobi"),
    ("ECT", "Europe/Paris"),
    ("IET", "America/Indianapolis"),
    ("IST", "Asia/Calcutta"),
    ("JST", "Asia/Tokyo"),
    ("MIT", "Pacific/Apia"),
    ("NET", "Asia/Yerevan"),
    ("NST", "Pacific/Auckland"),
    ("PLT", "Asia/Karachi"),
    ("PNT", "America/Phoenix"),
    ("PRT", "America/Puerto_Rico"),
    ("PST", "America/Los_Angeles"),
    ("SST", "Pacific/Guadalcanal"),
    ("VST", "Asia/Saigon"),
    ("US/Pacific-New", "America/Los_Angeles"),
    ("Canada/East-Saskatchewan", "America/Regina"),
    ("SystemV/AST4", "Etc/GMT+4"),
    ("SystemV/CST6", "Etc/GMT+6"),
    ("SystemV/EST5", "Etc/GMT+5"),
    ("SystemV/HST10", "Etc/GMT+10"),
    ("SystemV/MST7", "Etc/GMT+7"),
    ("SystemV/PST8", "Etc/GMT+8"),
];

fn digits(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0, |value, digit| value * 10 + i64::from(digit - b'0')),
    )
}

/// `Date.parse` for strings matching the timeline `at` schema pattern:
/// `YYYY-MM-DDTHH:MM[:SS[.fff]]` followed by `Z` or `±HH:MM`. Like V8, a day up
/// to 31 rolls into the next month; `valid_calendar_date` rejects it separately.
pub fn parse_timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() < 17
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
    {
        return None;
    }
    let (year, month, day) = (
        digits(&bytes[0..4])?,
        digits(&bytes[5..7])?,
        digits(&bytes[8..10])?,
    );
    let (hour, minute) = (digits(&bytes[11..13])?, digits(&bytes[14..16])?);
    let mut rest = &bytes[16..];
    let (mut second, mut millisecond) = (0, 0);
    if rest.first() == Some(&b':') {
        second = digits(rest.get(1..3)?)?;
        rest = &rest[3..];
        if rest.first() == Some(&b'.') {
            let end = rest[1..]
                .iter()
                .position(|byte| !byte.is_ascii_digit())
                .map_or(rest.len(), |at| at + 1);
            let fraction = &rest[1..end];
            if fraction.is_empty() || fraction.len() > 3 {
                return None;
            }
            millisecond = digits(fraction)? * 10_i64.pow(3 - fraction.len() as u32);
            rest = &rest[end..];
        }
    }
    let offset = match rest {
        [b'Z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let (hours, minutes) = (digits(&[*h1, *h2])?, digits(&[*m1, *m2])?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            let offset = hours * HOUR + minutes * MINUTE;
            if *sign == b'-' {
                -offset
            } else {
                offset
            }
        }
        _ => return None,
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || minute > 59 || second > 59 {
        return None;
    }
    if hour > 24 || (hour == 24 && (minute, second, millisecond) != (0, 0, 0)) {
        return None;
    }
    let first = NaiveDate::from_ymd_opt(year as i32, month as u32, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp_millis();
    Some(
        first + (day - 1) * DAY + hour * HOUR + minute * MINUTE + second * 1000 + millisecond
            - offset,
    )
}

/// The reference's independent calendar check on the authored `YYYY-MM-DD`.
pub fn valid_calendar_date(value: &str) -> bool {
    let mut parts = value
        .get(..10)
        .unwrap_or_default()
        .split('-')
        .map(|part| part.parse::<i64>().ok());
    let (Some(Some(year)), Some(Some(month)), Some(Some(day))) =
        (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    (1..=12).contains(&month) && day >= 1 && day <= days[month as usize - 1]
}

fn named(value: &str) -> Option<Tz> {
    TZ_VARIANTS
        .iter()
        .copied()
        .find(|zone| zone.name().eq_ignore_ascii_case(value))
}

/// ICU's time zone selection: a case-insensitive zone name, else a fixed
/// offset written `±HH`, `±HHMM` or `±HH:MM` (also with U+2212 as minus).
pub fn zone(value: &str) -> Option<Zone> {
    if let Some(zone) = named(value).or_else(|| {
        ICU_ALIASES
            .iter()
            .find(|(alias, _)| alias.eq_ignore_ascii_case(value))
            .and_then(|(_, target)| named(target))
    }) {
        return Some(Zone::Named(zone));
    }
    let (negative, rest) = match value.strip_prefix('+') {
        Some(rest) => (false, rest),
        None => (
            true,
            value
                .strip_prefix('-')
                .or_else(|| value.strip_prefix('\u{2212}'))?,
        ),
    };
    let bytes = rest.as_bytes();
    let (hours, minutes) = match bytes {
        [h1, h2] => (digits(&[*h1, *h2])?, 0),
        [h1, h2, m1, m2] | [h1, h2, b':', m1, m2] => (digits(&[*h1, *h2])?, digits(&[*m1, *m2])?),
        _ => return None,
    };
    if hours > 23 || minutes > 59 {
        return None;
    }
    let seconds = (hours * 3600 + minutes * 60) as i32;
    FixedOffset::east_opt(if negative { -seconds } else { seconds }).map(Zone::Fixed)
}

/// The wall clock of an instant in a display zone, as ICU's proleptic
/// Gregorian en-US formatter reports it. Years before 1 AD are astronomical:
/// 1 BC is year 0, matching the reference's `1 - year` for the BC era.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Wall {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

fn offset_seconds(ms: i64, zone: &Zone) -> i64 {
    match zone {
        Zone::Fixed(offset) => i64::from(offset.local_minus_utc()),
        Zone::Named(tz) => DateTime::from_timestamp_millis(ms).map_or(0, |instant| {
            i64::from(
                tz.offset_from_utc_datetime(&instant.naive_utc())
                    .fix()
                    .local_minus_utc(),
            )
        }),
    }
}

pub fn wall(ms: i64, zone: &Zone) -> Wall {
    let local = DateTime::from_timestamp_millis(ms + offset_seconds(ms, zone) * 1000)
        .expect("timeline instants lie within the supported calendar range")
        .naive_utc();
    Wall {
        year: i64::from(local.year()),
        month: local.month(),
        day: local.day(),
        hour: local.hour(),
        minute: local.minute(),
        second: local.second(),
    }
}

/// `UTC±HH:MM` of the zone's offset at an instant, rounded to whole minutes
/// the way `Math.round` rounds a seconds-precision historical offset.
pub fn offset_label(ms: i64, zone: &Zone) -> String {
    let minutes = crate::format::number::js_round(offset_seconds(ms, zone) as f64 / 60.0) as i64;
    let sign = if minutes < 0 { '-' } else { '+' };
    let minutes = minutes.abs();
    format!("UTC{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}
