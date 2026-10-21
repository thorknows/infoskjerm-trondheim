/*
This module adds seasonal fluff:
Halloween bats and sppoky face in late october
Snow effect in December
....more? :D
*/

use crate::ui::*;
use chrono::{Datelike, Local};
use slint::{Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
mod december;
mod halloween;
use december::setup_snow;
use halloween::{setup_halloween_bat, setup_halloween_spooky_face};

// How often we check whether a season should start or end. The effects themselves run on
// their own, much more frequent timers -- this only needs to be frequent enough that a
// season reliably turns on/off without requiring the app to be restarted.
const SCHEDULE_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 15);

fn is_halloween_season(now: chrono::DateTime<Local>) -> bool {
    now.month() == 10 && now.day() >= 10 && now.day() <= 31
}

fn is_snow_season(now: chrono::DateTime<Local>) -> bool {
    now.month() == 12
}

fn hide_halloween_effects(main_window: &MainWindow) {
    let mut spooky_face = main_window.get_spooky_face();
    spooky_face.hidden = true;
    main_window.set_spooky_face(spooky_face);
    main_window.set_bat(Bat { pos: Default::default() });
}

fn clear_snow(main_window: &MainWindow) {
    let snowflakes: VecModel<Snowflake> = VecModel::default();
    main_window.set_snowflakes(Rc::new(snowflakes).into());
}

pub fn setup_seasons(main_window: &MainWindow) -> Vec<Timer> {
    let halloween_timers: Rc<RefCell<Option<(Timer, Timer)>>> = Rc::new(RefCell::new(None));
    let snow_timer: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));

    let scheduler = Timer::default();
    let window_weak = main_window.as_weak();

    // Run once immediately so the correct seasons are active right away, then keep
    // re-checking periodically so seasons turn on/off without needing an app restart.
    let check = move || {
        let Some(window) = window_weak.upgrade() else { return };
        let now = Local::now();

        if is_halloween_season(now) {
            if halloween_timers.borrow().is_none() {
                *halloween_timers.borrow_mut() = Some((
                    setup_halloween_spooky_face(&window),
                    setup_halloween_bat(&window),
                ));
            }
        } else if halloween_timers.borrow_mut().take().is_some() {
            hide_halloween_effects(&window);
        }

        if is_snow_season(now) {
            if snow_timer.borrow().is_none() {
                *snow_timer.borrow_mut() = Some(setup_snow(&window));
            }
        } else if snow_timer.borrow_mut().take().is_some() {
            clear_snow(&window);
        }
    };

    check();
    scheduler.start(TimerMode::Repeated, SCHEDULE_CHECK_INTERVAL, check);

    vec![scheduler]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn date(month: u32, day: u32) -> chrono::DateTime<Local> {
        Local.with_ymd_and_hms(2026, month, day, 12, 0, 0).unwrap()
    }

    #[test]
    fn halloween_window_boundaries() {
        assert!(!is_halloween_season(date(10, 9)));
        assert!(is_halloween_season(date(10, 10)));
        assert!(is_halloween_season(date(10, 31)));
        assert!(!is_halloween_season(date(11, 1)));
    }

    #[test]
    fn snow_window_boundaries() {
        assert!(!is_snow_season(date(11, 30)));
        assert!(is_snow_season(date(12, 1)));
        assert!(is_snow_season(date(12, 31)));
        assert!(!is_snow_season(date(1, 1)));
    }
}
