use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

pub type DateBounds = (DateTime<Utc>, DateTime<Utc>, DateTime<Utc>);

pub fn date_bounds(
    from: NaiveDate,
    to: NaiveDate,
    timezone: &str,
) -> Result<DateBounds, &'static str> {
    if to < from || (to - from).num_days() > 366 {
        return Err("invalid date range");
    }
    let timezone: Tz = timezone.parse().map_err(|_| "invalid site timezone")?;
    let day_count = (to - from).num_days() as u64 + 1;
    let end_date = to
        .checked_add_days(Days::new(1))
        .ok_or("invalid date range")?;
    let prior_date = from
        .checked_sub_days(Days::new(day_count))
        .ok_or("invalid date range")?;
    let at_midnight = |date: NaiveDate| {
        local_instant(timezone, date.and_time(NaiveTime::MIN)).ok_or("invalid local date")
    };
    Ok((
        at_midnight(from)?,
        at_midnight(end_date)?,
        at_midnight(prior_date)?,
    ))
}

/// Which earlier period a range's metrics are compared against.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compare {
    /// The same number of days immediately before the range.
    #[default]
    Previous,
    /// The same days one week earlier.
    Week,
}

/// The instant a local wall-clock time names. An ambiguous time (clocks falling back) is its
/// first occurrence; a time skipped by clocks springing forward moves to just after the gap.
pub fn local_instant(timezone: Tz, time: NaiveDateTime) -> Option<DateTime<Utc>> {
    let resolve = |time: NaiveDateTime| timezone.from_local_datetime(&time).earliest();
    resolve(time)
        .or_else(|| resolve(time + chrono::Duration::hours(1)))
        .map(|value| value.with_timezone(&Utc))
}

/// How many days back `compare` moves a range.
pub fn compare_shift(from: NaiveDate, to: NaiveDate, compare: Compare) -> Days {
    Days::new(match compare {
        Compare::Previous => (to - from).num_days() as u64 + 1,
        Compare::Week => 7,
    })
}

/// The comparison window for a range: the same local days `compare` shifts back to. While the
/// range is still in progress, the window ends at the same local clock time (in whole
/// five-minute steps, so it can be cached briefly), so "today so far" is measured against
/// yesterday (or last week) up to the same time of day. Returns the window's start, its
/// (possibly cut) end, and the end of its full last day.
pub fn compare_bounds(
    from: NaiveDate,
    to: NaiveDate,
    timezone: &str,
    compare: Compare,
    now: DateTime<Utc>,
) -> Result<DateBounds, &'static str> {
    let (start, end, _) = date_bounds(from, to, timezone)?;
    let zone: Tz = timezone.parse().map_err(|_| "invalid site timezone")?;
    let shift = compare_shift(from, to, compare);
    let midnight = |date: NaiveDate| {
        let date = date.checked_sub_days(shift).ok_or("invalid date range")?;
        local_instant(zone, date.and_time(NaiveTime::MIN)).ok_or("invalid local date")
    };
    let next_day = to.succ_opt().ok_or("invalid date range")?;
    let (prior_start, prior_end) = (midnight(from)?, midnight(next_day)?);
    let cut_end = if start < now && now < end {
        let local = now.with_timezone(&zone).naive_local();
        let step = local.time().num_seconds_from_midnight() % 300;
        let floored = local
            .with_nanosecond(0)
            .expect("zero is a valid nanosecond");
        let cut = (floored - chrono::Duration::seconds(step.into()))
            .checked_sub_days(shift)
            .and_then(|time| local_instant(zone, time))
            .ok_or("invalid local date")?;
        cut.clamp(prior_start, prior_end)
    } else {
        prior_end
    };
    Ok((prior_start, cut_end, prior_end))
}

/// One hour of a day and the matching hour of its comparison day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HourSlot {
    /// The local clock hour the slot starts at; it repeats on the day clocks fall back.
    pub hour: i32,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub prior_start: DateTime<Utc>,
    pub prior_end: DateTime<Utc>,
}

/// The real hours from `start` to `end` (a local day: 23 or 25 of them on daylight saving
/// days), each beside one hour starting at the same local clock time `shift` days earlier.
pub fn hour_slots(
    timezone: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    shift: Days,
) -> Result<Vec<HourSlot>, &'static str> {
    let zone: Tz = timezone.parse().map_err(|_| "invalid site timezone")?;
    let earlier = |instant: DateTime<Utc>| {
        instant
            .with_timezone(&zone)
            .naive_local()
            .checked_sub_days(shift)
            .and_then(|time| local_instant(zone, time))
            .ok_or("invalid local date")
    };
    let prior_day_end = earlier(end)?;
    let mut slots = vec![];
    let mut slot_start = start;
    while slot_start < end {
        let slot_end = (slot_start + chrono::Duration::hours(1)).min(end);
        let prior_start = earlier(slot_start)?.min(prior_day_end);
        slots.push(HourSlot {
            hour: slot_start.with_timezone(&zone).hour() as i32,
            start: slot_start,
            end: slot_end,
            prior_start,
            prior_end: (prior_start + chrono::Duration::hours(1)).min(prior_day_end),
        });
        slot_start = slot_end;
    }
    Ok(slots)
}
