#[derive(Debug, Clone)]
pub(super) struct SnoozeChoice {
    pub label: &'static str,
    pub deadline_unix_ms: i64,
    pub wake_label: String,
}

pub(super) fn snooze_choices(now_ms: i64) -> Result<Vec<SnoozeChoice>, String> {
    const MINUTE_MS: i64 = 60 * 1_000;
    const HOUR_MS: i64 = 60 * MINUTE_MS;
    const DAY_MS: i64 = 24 * HOUR_MS;

    let now = crate::platform::local_datetime_at(now_ms.div_euclid(1_000))
        .ok_or_else(|| "local clock cannot represent the picker time".to_string())?;
    let ten_am =
        time::Time::from_hms(10, 0, 0).map_err(|_| "invalid fixed snooze time".to_string())?;
    let mut choices = Vec::with_capacity(6);
    for (label, offset_ms) in [
        ("30 minutes", 30 * MINUTE_MS),
        ("1 hour", HOUR_MS),
        ("3 hours", 3 * HOUR_MS),
        ("3 days", 3 * DAY_MS),
    ] {
        let deadline = now_ms
            .checked_add(offset_ms)
            .ok_or_else(|| "snooze deadline exceeds the supported time range".to_string())?;
        choices.push(choice_at(label, deadline)?);
    }

    let tomorrow = now
        .date()
        .next_day()
        .ok_or_else(|| "tomorrow is outside the supported date range".to_string())?;
    choices.insert(
        3,
        choice_at_local(
            "Tomorrow 10:00",
            time::PrimitiveDateTime::new(tomorrow, ten_am),
        )?,
    );

    let next_monday = next_monday(now.date())
        .ok_or_else(|| "next Monday is outside the supported date range".to_string())?;
    choices.push(choice_at_local(
        "Next Monday 10:00",
        time::PrimitiveDateTime::new(next_monday, ten_am),
    )?);
    Ok(choices)
}

fn next_monday(date: time::Date) -> Option<time::Date> {
    let days_until_monday = match date.weekday() {
        time::Weekday::Monday => 7,
        weekday => 7_u8 - weekday.number_days_from_monday(),
    };
    let mut monday = date;
    for _ in 0..days_until_monday {
        monday = monday.next_day()?;
    }
    Some(monday)
}

fn choice_at(label: &'static str, deadline_unix_ms: i64) -> Result<SnoozeChoice, String> {
    let wake = crate::platform::local_datetime_at(deadline_unix_ms.div_euclid(1_000))
        .ok_or_else(|| "local clock cannot represent the snooze deadline".to_string())?;
    Ok(SnoozeChoice {
        label,
        deadline_unix_ms,
        wake_label: format_wake_label(wake),
    })
}

fn choice_at_local(
    label: &'static str,
    local_datetime: time::PrimitiveDateTime,
) -> Result<SnoozeChoice, String> {
    let deadline_seconds = crate::platform::local_timestamp(local_datetime)
        .ok_or_else(|| "local snooze time does not exist in the current timezone".to_string())?;
    let deadline_ms = deadline_seconds
        .checked_mul(1_000)
        .ok_or_else(|| "snooze deadline exceeds the supported time range".to_string())?;
    choice_at(label, deadline_ms)
}

fn format_wake_label(value: time::PrimitiveDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} local",
        value.year(),
        value.month() as u8,
        value.day(),
        value.hour(),
        value.minute(),
        value.second()
    )
}

pub(super) fn wake_label_for_deadline(deadline_unix_ms: i64) -> String {
    crate::platform::local_datetime_at(deadline_unix_ms.div_euclid(1_000))
        .map(format_wake_label)
        .unwrap_or_else(|| "unknown local time".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ms(year: i32, month: u8, day: u8, hour: u8) -> i64 {
        let date = time::Date::from_calendar_date(year, time::Month::try_from(month).unwrap(), day)
            .unwrap();
        let value = time::PrimitiveDateTime::new(date, time::Time::from_hms(hour, 0, 0).unwrap());
        crate::platform::local_timestamp(value).unwrap() * 1_000
    }

    #[test]
    fn elapsed_presets_keep_fixed_utc_offsets() {
        let now_ms = 1_767_000_000_000 + 123;
        let choices = snooze_choices(now_ms).unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.label)
                .collect::<Vec<_>>(),
            vec![
                "30 minutes",
                "1 hour",
                "3 hours",
                "Tomorrow 10:00",
                "3 days",
                "Next Monday 10:00",
            ]
        );
        assert_eq!(choices[0].deadline_unix_ms, now_ms + 30 * 60 * 1_000);
        assert_eq!(choices[1].deadline_unix_ms, now_ms + 60 * 60 * 1_000);
        assert_eq!(choices[2].deadline_unix_ms, now_ms + 3 * 60 * 60 * 1_000);
        assert_eq!(
            choices[4].deadline_unix_ms,
            now_ms + 3 * 24 * 60 * 60 * 1_000
        );
    }

    #[test]
    #[ignore = "requires TZ=America/Los_Angeles"]
    fn tomorrow_preserves_local_ten_am_across_dst() {
        let march_now = local_ms(2026, 3, 7, 10);
        let march = snooze_choices(march_now).unwrap();
        assert_eq!(march[3].deadline_unix_ms - march_now, 23 * 60 * 60 * 1_000);
        assert_eq!(march[3].wake_label, "2026-03-08 10:00:00 local");

        let november_now = local_ms(2026, 10, 31, 10);
        let november = snooze_choices(november_now).unwrap();
        assert_eq!(
            november[3].deadline_unix_ms - november_now,
            25 * 60 * 60 * 1_000
        );
        assert_eq!(november[3].wake_label, "2026-11-01 10:00:00 local");
    }

    #[test]
    fn next_monday_is_strictly_future() {
        let friday = time::Date::from_calendar_date(2026, time::Month::January, 9).unwrap();
        let sunday = time::Date::from_calendar_date(2026, time::Month::January, 11).unwrap();
        let monday = time::Date::from_calendar_date(2026, time::Month::January, 12).unwrap();
        assert_eq!(next_monday(friday).unwrap().day(), 12);
        assert_eq!(next_monday(sunday).unwrap().day(), 12);
        assert_eq!(next_monday(monday).unwrap().day(), 19);
    }
}
