use crate::calendar::calendar_models::{CalendarEvent, EventKind};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};
use uuid::Uuid;

// Single-day holidays are shown this many days in advance.
const SINGLE_DAY_LEAD_DAYS: i64 = 7;
// Multi-day holidays (e.g. Christmas, Easter) are shown this many days in advance.
const MULTI_DAY_LEAD_DAYS: i64 = 21;

struct HolidayPeriod {
    start: NaiveDate,
    end: NaiveDate,
    name: String,
}

// Meeus/Jones/Butcher Gregorian algorithm.
fn easter_sunday(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

// Norway's official "red days" (statutory non-working holidays) for a given year.
fn norwegian_red_days(year: i32) -> Vec<(NaiveDate, String)> {
    let easter = easter_sunday(year);
    let relative_to_easter = |offset: i64| easter + Duration::days(offset);

    vec![
        (
            NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
            "Første nyttårsdag".to_string(),
        ),
        (relative_to_easter(-3), "Skjærtorsdag".to_string()),
        (relative_to_easter(-2), "Langfredag".to_string()),
        (relative_to_easter(0), "Første påskedag".to_string()),
        (relative_to_easter(1), "Andre påskedag".to_string()),
        (
            NaiveDate::from_ymd_opt(year, 5, 1).unwrap(),
            "Arbeidernes dag".to_string(),
        ),
        (
            NaiveDate::from_ymd_opt(year, 5, 17).unwrap(),
            "Grunnlovsdagen".to_string(),
        ),
        (relative_to_easter(39), "Kristi himmelfartsdag".to_string()),
        (relative_to_easter(49), "Første pinsedag".to_string()),
        (relative_to_easter(50), "Andre pinsedag".to_string()),
        (
            NaiveDate::from_ymd_opt(year, 12, 25).unwrap(),
            "Første juledag".to_string(),
        ),
        (
            NaiveDate::from_ymd_opt(year, 12, 26).unwrap(),
            "Andre juledag".to_string(),
        ),
    ]
}

// Builds a collective name for a group of consecutive holidays, e.g.
// ["Første juledag", "Andre juledag"] -> "Jul".
fn collective_name(names: &[String]) -> String {
    if names.len() == 1 {
        return names[0].clone();
    }

    fn base(name: &str) -> &str {
        name.split_once(' ').map(|(_, rest)| rest).unwrap_or(name)
    }

    let first_base = base(&names[0]);

    if names.iter().all(|name| base(name) == first_base) {
        return match first_base {
            "juledag" => "Jul".to_string(),
            "påskedag" => "Påske".to_string(),
            "pinsedag" => "Pinse".to_string(),
            other => capitalize(other),
        };
    }

    names.join(" og ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => s.to_string(),
    }
}

// Groups consecutive red days (e.g. Christmas Day + Boxing Day) into single periods.
fn holiday_periods(years: &[i32]) -> Vec<HolidayPeriod> {
    let mut days: Vec<(NaiveDate, String)> =
        years.iter().flat_map(|&year| norwegian_red_days(year)).collect();
    days.sort_by_key(|(date, _)| *date);

    let mut periods = Vec::new();
    let mut current: Option<(NaiveDate, NaiveDate, Vec<String>)> = None;

    for (date, name) in days {
        match &mut current {
            Some((_, end, names)) if date == *end + Duration::days(1) => {
                *end = date;
                names.push(name);
            }
            _ => {
                if let Some((start, end, names)) = current.take() {
                    periods.push(HolidayPeriod { start, end, name: collective_name(&names) });
                }
                current = Some((date, date, vec![name]));
            }
        }
    }
    if let Some((start, end, names)) = current {
        periods.push(HolidayPeriod { start, end, name: collective_name(&names) });
    }

    periods
}

pub fn build_holiday_events() -> Vec<CalendarEvent> {
    build_holiday_events_for_day(Local::now().date_naive())
}

fn build_holiday_events_for_day(today: NaiveDate) -> Vec<CalendarEvent> {
    let year = today.year();

    holiday_periods(&[year, year + 1])
        .into_iter()
        .filter_map(|period| {
            let is_single_day = period.start == period.end;

            // A single-day holiday that falls on a weekend grants no extra time off, so skip it.
            if is_single_day && matches!(period.start.weekday(), Weekday::Sat | Weekday::Sun) {
                return None;
            }

            let lead_days = if is_single_day { SINGLE_DAY_LEAD_DAYS } else { MULTI_DAY_LEAD_DAYS };
            let show_from = period.start - Duration::days(lead_days);

            if today < show_from {
                return None;
            }

            let start_time = Local
                .from_local_datetime(&period.start.and_hms_opt(0, 0, 0).unwrap())
                .single()?;
            let stop_time = Local
                .from_local_datetime(&period.end.and_hms_opt(23, 59, 59).unwrap())
                .single()?;

            Some(CalendarEvent {
                id: Uuid::new_v4(),
                summary: period.name,
                start_time,
                stop_time,
                kind: EventKind::Holiday,
                description: String::new(),
                location: String::new(),
            })
        })
        .collect()
}

pub fn countdown_text(target_date: NaiveDate) -> String {
    let today = Local::now().date_naive();
    let days = (target_date - today).num_days();

    match days {
        d if d < 0 => "Pågår".to_string(),
        0 => "I dag".to_string(),
        1 => "I morgen".to_string(),
        n => format!("{n} dager"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easter_matches_known_dates() {
        assert_eq!(easter_sunday(2023), NaiveDate::from_ymd_opt(2023, 4, 9).unwrap());
        assert_eq!(easter_sunday(2024), NaiveDate::from_ymd_opt(2024, 3, 31).unwrap());
        assert_eq!(easter_sunday(2025), NaiveDate::from_ymd_opt(2025, 4, 20).unwrap());
        assert_eq!(easter_sunday(2026), NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
        assert_eq!(easter_sunday(2027), NaiveDate::from_ymd_opt(2027, 3, 28).unwrap());
    }

    #[test]
    fn christmas_groups_into_one_period() {
        let periods = holiday_periods(&[2024]);
        let christmas = periods
            .iter()
            .find(|p| p.start == NaiveDate::from_ymd_opt(2024, 12, 25).unwrap())
            .expect("Christmas period should exist");

        assert_eq!(christmas.end, NaiveDate::from_ymd_opt(2024, 12, 26).unwrap());
        assert_eq!(christmas.name, "Jul");
    }

    #[test]
    fn easter_sunday_and_monday_group_separately_from_thu_fri() {
        // 2024: Skjærtorsdag=28/3 (Thu), Langfredag=29/3 (Fri), 1.påskedag=31/3 (Sun),
        // 2.påskedag=1/4 (Mon) -- Saturday 30/3 is not a red day, so the Thu+Fri pair
        // and the Sun+Mon pair must NOT be merged into one period.
        let periods = holiday_periods(&[2024]);

        let thu_fri = periods
            .iter()
            .find(|p| p.start == NaiveDate::from_ymd_opt(2024, 3, 28).unwrap())
            .expect("Skjærtorsdag/Langfredag period should exist");
        assert_eq!(thu_fri.end, NaiveDate::from_ymd_opt(2024, 3, 29).unwrap());
        assert_eq!(thu_fri.name, "Skjærtorsdag og Langfredag");

        let sun_mon = periods
            .iter()
            .find(|p| p.start == NaiveDate::from_ymd_opt(2024, 3, 31).unwrap())
            .expect("Påske period should exist");
        assert_eq!(sun_mon.end, NaiveDate::from_ymd_opt(2024, 4, 1).unwrap());
        assert_eq!(sun_mon.name, "Påske");
    }

    #[test]
    fn single_day_on_weekend_is_excluded() {
        // 17 May 2025 is a Saturday.
        assert_eq!(
            NaiveDate::from_ymd_opt(2025, 5, 17).unwrap().weekday(),
            Weekday::Sat
        );

        let events = build_holiday_events_for_day(NaiveDate::from_ymd_opt(2025, 5, 10).unwrap());
        assert!(!events.iter().any(|e| e.summary == "Grunnlovsdagen"));
    }

    #[test]
    fn single_day_lead_time_is_seven_days() {
        // 1 May 2025 is a Thursday -- a weekday, so it should normally show.
        let target = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();

        let too_early = build_holiday_events_for_day(target - Duration::days(8));
        assert!(!too_early.iter().any(|e| e.summary == "Arbeidernes dag"));

        let in_window = build_holiday_events_for_day(target - Duration::days(7));
        assert!(in_window.iter().any(|e| e.summary == "Arbeidernes dag"));
    }

    #[test]
    fn multi_day_lead_time_is_three_weeks() {
        let target = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();

        let too_early = build_holiday_events_for_day(target - Duration::days(22));
        assert!(!too_early.iter().any(|e| e.summary == "Jul"));

        let in_window = build_holiday_events_for_day(target - Duration::days(21));
        assert!(in_window.iter().any(|e| e.summary == "Jul"));
    }

    #[test]
    fn countdown_text_variants() {
        let today = Local::now().date_naive();
        assert_eq!(countdown_text(today), "I dag");
        assert_eq!(countdown_text(today + Duration::days(1)), "I morgen");
        assert_eq!(countdown_text(today + Duration::days(5)), "5 dager");
        assert_eq!(countdown_text(today - Duration::days(1)), "Pågår");
    }
}
