use chrono::Days;
use chrono::{NaiveDate, TimeZone, Utc};
use slimlytics_backend::reporting::{compare_bounds, date_bounds, hour_slots, Compare};

#[test]
fn site_date_bounds_follow_daylight_saving_time() {
    let date = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
    let (start, end, prior) = date_bounds(date, date, "America/Denver").unwrap();

    assert_eq!(start, Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0).unwrap());
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 3, 9, 6, 0, 0).unwrap());
    assert_eq!(prior, Utc.with_ymd_and_hms(2026, 3, 7, 7, 0, 0).unwrap());
}

#[test]
fn rejects_unknown_timezones_and_invalid_ranges() {
    let from = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

    assert!(date_bounds(from, to, "UTC").is_err());
    assert!(date_bounds(to, to, "Not/A_Timezone").is_err());
}

#[test]
fn compares_today_so_far_with_the_same_time_yesterday_and_last_week() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
    // 8:57 PM in Denver.
    let now = Utc.with_ymd_and_hms(2026, 10, 22, 2, 57, 30).unwrap();

    let (start, end, full_end) =
        compare_bounds(today, today, "America/Denver", Compare::Previous, now).unwrap();
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 10, 20, 6, 0, 0).unwrap());
    // Cut to the same elapsed time, in whole five-minute steps.
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 21, 2, 55, 0).unwrap());
    assert_eq!(
        full_end,
        Utc.with_ymd_and_hms(2026, 10, 21, 6, 0, 0).unwrap()
    );

    let (start, end, _) =
        compare_bounds(today, today, "America/Denver", Compare::Week, now).unwrap();
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 10, 14, 6, 0, 0).unwrap());
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 15, 2, 55, 0).unwrap());
}

#[test]
fn compares_finished_ranges_with_whole_earlier_periods() {
    let from = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 22, 0, 0, 0).unwrap();

    let (start, end, full_end) = compare_bounds(from, to, "UTC", Compare::Previous, now).unwrap();
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 9, 24, 0, 0, 0).unwrap());
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap());
    assert_eq!(end, full_end);
}

#[test]
fn compares_at_the_same_local_clock_time_across_daylight_saving_changes() {
    // Noon in Denver on the day clocks spring forward (MDT, UTC-6) against noon the day
    // before (MST, UTC-7), not the same number of elapsed hours since midnight.
    let day = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
    let noon = Utc.with_ymd_and_hms(2026, 3, 8, 18, 0, 0).unwrap();
    let (_, end, _) = compare_bounds(day, day, "America/Denver", Compare::Previous, noon).unwrap();
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 3, 7, 19, 0, 0).unwrap());

    // The day clocks fall back (MST) against the day before (MDT).
    let day = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let noon = Utc.with_ymd_and_hms(2026, 11, 1, 19, 0, 0).unwrap();
    let (_, end, _) = compare_bounds(day, day, "America/Denver", Compare::Previous, noon).unwrap();
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 31, 18, 0, 0).unwrap());
}

#[test]
fn compares_with_days_next_to_a_skipped_midnight() {
    // Chile springs forward at midnight on September 6, 2026; that day is never needed here.
    let day = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
    let later = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let (start, end, _) =
        compare_bounds(day, day, "America/Santiago", Compare::Previous, later).unwrap();
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 9, 7, 3, 0, 0).unwrap());
    assert_eq!(end, Utc.with_ymd_and_hms(2026, 9, 8, 3, 0, 0).unwrap());

    // A comparison day whose own midnight was skipped starts when that day's clock began.
    let day = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
    let (start, _, _) =
        compare_bounds(day, day, "America/Santiago", Compare::Previous, later).unwrap();
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 9, 6, 4, 0, 0).unwrap());
}

#[test]
fn cutoffs_stay_identical_within_a_five_minute_step() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
    let early =
        Utc.with_ymd_and_hms(2026, 10, 22, 2, 55, 1).unwrap() + chrono::Duration::nanoseconds(123);
    let late =
        Utc.with_ymd_and_hms(2026, 10, 22, 2, 59, 59).unwrap() + chrono::Duration::nanoseconds(999);
    let bounds =
        |now| compare_bounds(today, today, "America/Denver", Compare::Previous, now).unwrap();
    assert_eq!(bounds(early), bounds(late));
}

#[test]
fn hour_slots_follow_the_real_clock_on_daylight_saving_days() {
    let at = |d, h| Utc.with_ymd_and_hms(2026, 3, d, h, 0, 0).unwrap();
    // March 8 in Denver: midnight MST (07:00Z) to midnight MDT (06:00Z next day), 23 hours.
    let slots = hour_slots("America/Denver", at(8, 7), at(9, 6), Days::new(1)).unwrap();
    assert_eq!(slots.len(), 23);
    assert_eq!(
        slots
            .iter()
            .map(|slot| slot.hour)
            .take(4)
            .collect::<Vec<_>>(),
        [0, 1, 3, 4]
    );
    // The 1 AM hour (which ends at 3 AM local) compares with one hour from 1 AM the day before.
    assert_eq!(
        (slots[1].prior_start, slots[1].prior_end),
        (at(7, 8), at(7, 9))
    );
    // 3 AM compares with 3 AM.
    assert_eq!(slots[2].prior_start, at(7, 10));

    let at = |d, h| {
        Utc.with_ymd_and_hms(2026, d, if d == 10 { 31 } else { 1 }, h, 0, 0)
            .unwrap()
    };
    // November 1: 25 hours; 1 AM appears twice, each beside one hour from 1 AM on October 31.
    let slots = hour_slots(
        "America/Denver",
        at(11, 6),
        at(11, 6) + chrono::Duration::hours(25),
        Days::new(1),
    )
    .unwrap();
    assert_eq!(slots.len(), 25);
    assert_eq!((slots[1].hour, slots[2].hour), (1, 1));
    assert_eq!(slots[1].prior_start, slots[2].prior_start);
    assert_eq!(
        slots[1].prior_end - slots[1].prior_start,
        chrono::Duration::hours(1)
    );
    assert_eq!(slots[1].prior_start, at(10, 7));
}
