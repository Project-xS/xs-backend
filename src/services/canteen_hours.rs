use chrono::{DateTime, FixedOffset, NaiveTime, TimeZone, Utc};

pub fn parse_tz_offset_from_env() -> FixedOffset {
    let raw = std::env::var("CANTEEN_TZ_OFFSET").unwrap_or_else(|_| "+00:00".to_string());
    parse_tz_offset(&raw).unwrap_or_else(|| {
        warn!("Invalid CANTEEN_TZ_OFFSET '{}', defaulting to +00:00", raw);
        FixedOffset::east_opt(0).expect("fixed offset")
    })
}

pub fn compute_close_at(
    opened_at_utc: DateTime<Utc>,
    opening_time: NaiveTime,
    closing_time: NaiveTime,
    tz: FixedOffset,
) -> DateTime<FixedOffset> {
    let opened_local = opened_at_utc.with_timezone(&tz);
    let opened_date = opened_local.date_naive();
    let close_date = if closing_time > opening_time {
        opened_date
    } else if opened_local.time() >= opening_time {
        opened_date.succ_opt().expect("date overflow")
    } else {
        opened_date
    };

    let close_naive = close_date.and_time(closing_time);
    tz.from_local_datetime(&close_naive)
        .single()
        .unwrap_or_else(|| {
            tz.from_local_datetime(&close_naive)
                .earliest()
                .expect("close_at")
        })
}

fn parse_tz_offset(value: &str) -> Option<FixedOffset> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let sign_char = trimmed.chars().next()?;
    let sign = match sign_char {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let rest = &trimmed[1..];
    let mut parts = rest.split(':');
    let hours: i32 = parts.next()?.parse().ok()?;
    let minutes: i32 = parts.next().unwrap_or("0").parse().ok()?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    let total_seconds = sign * (hours * 3600 + minutes * 60);
    FixedOffset::east_opt(total_seconds)
}
