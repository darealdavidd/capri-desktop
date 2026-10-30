use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_time_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

pub fn rand_range(min: f64, max: f64) -> f64 {
    min + rand::random::<f64>() * (max - min)
}

pub fn pick<'a, T>(list: &'a [T]) -> &'a T {
    let idx = rand::random::<usize>() % list.len();
    &list[idx]
}

pub fn clamp(v: f64, min: f64, max: f64) -> f64 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub mod ease {
    pub fn linear(t: f64) -> f64 {
        t
    }

    /// power4.out
    pub fn out(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(4)
    }

    /// cubic in
    pub fn in_ease(t: f64) -> f64 {
        t * t * t
    }

    /// quartic in-out
    pub fn in_out(t: f64) -> f64 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
        }
    }
}

pub fn duration_str(ms: f64, lang: &str) -> String {
    let ru = lang == "ru";
    let s = (ms / 1000.0).round().max(0.0) as u64;
    if s < 60 {
        return if ru {
            format!("{s} с")
        } else {
            format!("{s} s")
        };
    }
    let m = s / 60;
    let ss = s % 60;
    if m < 60 {
        return match lang {
            "en" => format!("{m}m {ss:02}s"),
            "ru" => format!("{m} мин {ss:02} с"),
            _ => format!("{m} min {ss:02}"),
        };
    }
    let h = m / 60;
    let mm = m % 60;
    if ru {
        format!("{h} ч {mm:02}")
    } else {
        format!("{h} h {mm:02}")
    }
}

pub fn today_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    // Simple calendar day calculation from unix epoch
    let mut year = 1970;
    let mut d = days as i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            let mut month = 1;
            let days_in_months = [
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
            for dim in days_in_months {
                if d < dim {
                    return format!("{year:04}-{month:02}-{:02}", d + 1);
                }
                d -= dim;
                month += 1;
            }
            break;
        }
        d -= days_in_year;
        year += 1;
    }
    format!("{year:04}-01-01")
}

pub fn basename(p: &str) -> &str {
    let clean = p.trim_end_matches(['/', '\\']);
    clean.rsplit(['/', '\\']).next().unwrap_or(p)
}
