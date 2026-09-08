mod calendar_models;
mod ics;
mod server;

mod storage;

use crate::ui::*;

use crate::calendar::calendar_models::{CalendarEvent, EventKind};
use crate::calendar::ics::fetch_ics_events;
use crate::calendar::storage::get_calendar;
use crate::StaticAssets;
use chrono::{Local, Locale, TimeZone};
use slint::{Color, Image, Rgba8Pixel, SharedPixelBuffer, VecModel, Weak};
use std::{cmp::min, env, rc::Rc, thread};
use tokio::runtime::Runtime;

fn get_ics_url() -> Option<String> {
    env::var("CALENDAR_ICS_URL").ok()
}

fn get_birthdays_ics_url() -> Option<String> {
    env::var("CALENDAR_BIRTHDAYS_ICS_URL").ok()
}

pub fn setup(window: &MainWindow) {
    let window_weak = window.as_weak();

    //spawn server worker thread
    thread::spawn(move || {
        Runtime::new()
            .unwrap()
            .block_on(server::calendar_endpoint_server())
    });
    //thread for displaying calendar events
    thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(calendar_worker_loop(window_weak))
    });
}

async fn calendar_worker_loop(window: Weak<MainWindow>) {
    loop {
        let current_calendar = get_calendar().await;
        let mut events = current_calendar.events;

        if let Some(ics_url) = get_ics_url() {
            events.extend(fetch_ics_events(&ics_url, EventKind::Event).await);
        }

        if let Some(birthdays_url) = get_birthdays_ics_url() {
            events.extend(fetch_ics_events(&birthdays_url, EventKind::Birthday).await);
        }

        display_calendar(&window, events).await;
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

fn get_icon(kind: EventKind) -> Image {
    let icon_path = match kind {
        EventKind::Event => "calendar/time.png",
        EventKind::Birthday => "calendar/birthday.png",
    };

    let icon_data = StaticAssets::get(icon_path)
        .or_else(|| StaticAssets::get("not-found.png"))
        .unwrap()
        .data
        .into_owned();

    let icon = image::load_from_memory_with_format(&icon_data, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        icon.as_raw(),
        icon.width(),
        icon.height(),
    );

    Image::from_rgba8(buffer)
}

fn get_colors(kind: EventKind) -> (Color, Color, Color) {
    match kind {
        EventKind::Event => (
            Color::from_rgb_u8(0x1F, 0x0D, 0x8D),
            Color::from_rgb_u8(0xCF, 0xCF, 0xFF),
            Color::from_argb_u8(0x80, 0xCF, 0xCF, 0xFF),
        ),
        EventKind::Birthday => (
            Color::from_rgb_u8(0xE2, 0xE8, 0xF0),
            Color::from_rgb_u8(0x00, 0x00, 0x00),
            Color::from_argb_u8(0x80, 0x00, 0x00, 0x00),
        ),
    }
}

async fn display_calendar(window_weak: &Weak<MainWindow>, calendar: Vec<CalendarEvent>) {
    window_weak
        .upgrade_in_event_loop(move |window: MainWindow| {
            let calendar_events: VecModel<Event> = VecModel::default();

            let mut upcoming_events: Vec<CalendarEvent> = calendar
                .into_iter()
                .filter(|x| x.stop_time >= Local::now())
                .collect();
            upcoming_events.sort_by(|a, b| a.start_time.cmp(&b.start_time));

            let take_count = min(3, upcoming_events.len());

            for event in &upcoming_events[0..take_count] {
                let date = if event.kind == EventKind::Birthday {
                    event
                        .start_time
                        .format_localized("%-d %B", Locale::nb_NO)
                        .to_string()
                } else {
                    let date_and_start_time = event
                        .start_time
                        .format_localized("%-d %B %H:%M", Locale::nb_NO);
                    let end_time = event.stop_time.format_localized("%H:%M", Locale::nb_NO);
                    format!("{0}-{1}", date_and_start_time, end_time)
                };
                let summary = &event.summary;
                let (background, text_color, description_color) = get_colors(event.kind);
                calendar_events.push(Event {
                    summary: summary.into(),
                    date: date.into(),
                    description: event.description.as_str().into(),
                    location: event.location.as_str().into(),
                    icon: get_icon(event.kind),
                    background,
                    textColor: text_color,
                    descriptionColor: description_color,
                });
            }

            window.set_events(Rc::new(calendar_events).into());
        })
        .unwrap();
}
