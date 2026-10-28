use crate::calendar::calendar_models::{CalendarEvent, EventKind};
use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone};
use icalendar::{CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, EventLike};
use log::warn;
use uuid::Uuid;

fn to_local(value: &DatePerhapsTime, end_of_day: bool) -> Option<DateTime<Local>> {
    match value {
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(dt)) => Some(dt.with_timezone(&Local)),
        DatePerhapsTime::DateTime(CalendarDateTime::Floating(dt)) => {
            Local.from_local_datetime(dt).single()
        }
        // We don't resolve the named timezone database here, so treat it as local time.
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, .. }) => {
            Local.from_local_datetime(date_time).single()
        }
        DatePerhapsTime::Date(date) => {
            // RFC 5545: DTEND for an all-day event is exclusive (the day after the last day
            // of the event), so the inclusive last day is one day earlier.
            let (date, time) = if end_of_day {
                (*date - Duration::days(1), NaiveTime::from_hms_opt(23, 59, 59).unwrap())
            } else {
                (*date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            };
            Local.from_local_datetime(&date.and_time(time)).single()
        }
    }
}

pub async fn fetch_ics_events(url: &str, kind: EventKind) -> Vec<CalendarEvent> {
    let body = match reqwest::get(url).await {
        Ok(response) => match response.text().await {
            Ok(text) => text,
            Err(err) => {
                warn!("Failed to read calendar ICS response body: {err}");
                return Vec::new();
            }
        },
        Err(err) => {
            warn!("Failed to fetch calendar ICS feed: {err}");
            return Vec::new();
        }
    };

    let calendar: icalendar::Calendar = match body.parse() {
        Ok(calendar) => calendar,
        Err(err) => {
            warn!("Failed to parse calendar ICS feed: {err}");
            return Vec::new();
        }
    };

    calendar
        .components
        .iter()
        .filter_map(|component| match component {
            CalendarComponent::Event(event) => Some(event),
            _ => None,
        })
        .filter_map(|event| {
            let summary = match event.get_summary() {
                Some(summary) => summary.to_string(),
                None => {
                    warn!("Skipping calendar event with no summary: {event:?}");
                    return None;
                }
            };

            let start_time = match event.get_start().and_then(|d| to_local(&d, false)) {
                Some(start_time) => start_time,
                None => {
                    warn!("Skipping calendar event '{summary}' with no usable start time");
                    return None;
                }
            };

            let stop_time = match event.get_end().and_then(|d| to_local(&d, true)) {
                Some(stop_time) => stop_time,
                None => {
                    warn!("Skipping calendar event '{summary}' with no usable end time");
                    return None;
                }
            };

            let description = event.get_description().unwrap_or_default().to_string();
            let location = event.get_location().unwrap_or_default().to_string();

            Some(CalendarEvent {
                id: Uuid::new_v4(),
                summary,
                start_time,
                stop_time,
                kind,
                description,
                location,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn single_day_all_day_event_stays_within_one_day() {
        // Outlook/Google encode a single-day all-day event (e.g. a birthday on 11 Jan) as
        // DTSTART=20260111, DTEND=20260112 -- DTEND is exclusive per RFC 5545.
        let start = to_local(
            &DatePerhapsTime::Date(NaiveDate::from_ymd_opt(2026, 1, 11).unwrap()),
            false,
        )
        .unwrap();
        let end = to_local(
            &DatePerhapsTime::Date(NaiveDate::from_ymd_opt(2026, 1, 12).unwrap()),
            true,
        )
        .unwrap();

        assert_eq!(start.date_naive(), NaiveDate::from_ymd_opt(2026, 1, 11).unwrap());
        assert_eq!(end.date_naive(), NaiveDate::from_ymd_opt(2026, 1, 11).unwrap());
    }

    #[test]
    fn multi_day_all_day_event_gets_correct_inclusive_end() {
        // A 2-day all-day event spanning 25-26 Dec is encoded as DTSTART=20261225,
        // DTEND=20261227 (exclusive).
        let start = to_local(
            &DatePerhapsTime::Date(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
            false,
        )
        .unwrap();
        let end = to_local(
            &DatePerhapsTime::Date(NaiveDate::from_ymd_opt(2026, 12, 27).unwrap()),
            true,
        )
        .unwrap();

        assert_eq!(start.date_naive(), NaiveDate::from_ymd_opt(2026, 12, 25).unwrap());
        assert_eq!(end.date_naive(), NaiveDate::from_ymd_opt(2026, 12, 26).unwrap());
    }
}
