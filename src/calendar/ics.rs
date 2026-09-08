use crate::calendar::calendar_models::CalendarEvent;
use chrono::{DateTime, Local, NaiveTime, TimeZone};
use icalendar::{CalendarComponent, CalendarDateTime, Component, DatePerhapsTime};
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

pub async fn fetch_ics_events(url: &str) -> Vec<CalendarEvent> {
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
            let summary = event.get_summary()?.to_string();
            let start_time = to_local(&event.get_start()?, false)?;
            let stop_time = to_local(&event.get_end()?, true)?;

            Some(CalendarEvent {
                id: Uuid::new_v4(),
                summary,
                start_time,
                stop_time,
            })
        })
        .collect()
}
