use crate::calendar::calendar_models::{CalendarEvent, EventKind};
use chrono::{DateTime, Local, NaiveTime, TimeZone};
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
            let time = if end_of_day {
                NaiveTime::from_hms_opt(23, 59, 59).unwrap()
            } else {
                NaiveTime::from_hms_opt(0, 0, 0).unwrap()
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
