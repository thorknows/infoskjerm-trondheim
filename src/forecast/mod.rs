use chrono::{Local, Locale, Timelike};
use log::{error, info};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, VecModel, Weak};
use std::rc::Rc;
use std::string::ToString;
use std::thread;

use self::forecast_models::{ForecastModel, ForecastRaw};

use crate::{ui::*, StaticAssets};

mod forecast_models;

const API_URL: &str =
    "https://api.met.no/weatherapi/locationforecast/2.0/complete?lat=63.2549&lon=10.2342";
const USER_AGENT_STR: &str = "Knowit Infoskjerm - github.com/Knowit-Objectnet/infoskjerm-trondheim";
const FUTURE_DAYS: i64 = 2;

fn get_empty_forecast() -> ForecastModel {
    ForecastModel {
        icon_name: "None".to_string(),
        temp: "x".to_string(),
        precip: "x".to_string(),
    }
}

pub fn setup(window: &MainWindow) {
    let window_weak = window.as_weak();
    thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(weather_worker_loop(window_weak))
    });
}

async fn weather_worker_loop(window: Weak<MainWindow>) {
    loop {
        let response = get_forecast_data().await;
        let days = match response {
            None => (0..=FUTURE_DAYS)
                .map(|offset| (heading_for_offset(offset), get_empty_forecast()))
                .collect::<Vec<_>>(),
            Some(data) => build_forecast_days(&data),
        };
        display_forecast(&window, days);
        tokio::time::sleep(std::time::Duration::from_secs(60 * 15)).await;
    }
}

fn heading_for_offset(offset: i64) -> String {
    match offset {
        0 => "Nå".to_string(),
        1 => "I morgen".to_string(),
        _ => {
            let date = Local::now().date_naive() + chrono::Duration::days(offset);
            let weekday = date.format_localized("%A", Locale::nb_NO).to_string();
            let mut chars = weekday.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => weekday,
            }
        }
    }
}

fn build_forecast_days(data: &ForecastRaw) -> Vec<(String, ForecastModel)> {
    let mut days = vec![(heading_for_offset(0), get_forecast_now(data))];

    for offset in 1..=FUTURE_DAYS {
        let forecast = get_forecast_for_day_offset(data, offset).unwrap_or_else(get_empty_forecast);
        days.push((heading_for_offset(offset), forecast));
    }

    days
}

fn display_forecast(window: &Weak<MainWindow>, days: Vec<(String, ForecastModel)>) {
    let _ = window.upgrade_in_event_loop(move |window: MainWindow| {
        let model: VecModel<DayForecast> = VecModel::default();
        for (heading, forecast) in days {
            model.push(DayForecast {
                heading: heading.into(),
                forecast: forecast.into(),
            });
        }
        window.set_forecasts(Rc::new(model).into());
    });
}

impl From<ForecastModel> for Forecast {
    fn from(val: ForecastModel) -> Self {
        Forecast {
            icon: get_icon(val.icon_name),
            precipitation: val.precip.into(),
            temp: val.temp.into(),
        }
    }
}

async fn get_forecast_data() -> Option<ForecastRaw> {
    info! { "Fetching weather data" }

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT_STR),
    );

    let response = client.get(API_URL).headers(headers).send().await;

    let forecast_json = match response {
        Ok(res) => res.json().await,
        Err(err) => {
            error!("Failed to fetch weather data: {}", err);
            return None;
        }
    };

    match forecast_json {
        Ok(res) => Some(res),
        Err(err) => {
            error!("Failed to deserialize json: {}", err);
            None
        }
    }
}

fn get_forecast_now(data: &ForecastRaw) -> ForecastModel {
    let first_forecast = &data.properties.timeseries[0];

    let temp = first_forecast.data.instant.details.air_temperature;

    let next_hour = first_forecast
        .data
        .next_1_hours
        .as_ref()
        .expect("next_1_hours should be in forecast");

    ForecastModel {
        icon_name: next_hour.summary.symbol_code.to_owned(),
        temp: std::format!("{:.0}", temp),
        precip: std::format!("{:.0}", next_hour.details.precipitation_amount),
    }
}

// MET only provides hourly resolution for the near term; forecasts several days out are
// only available every 6 hours, so we pick the entry closest to midday on the target date
// rather than requiring an exact timestamp match.
fn get_forecast_for_day_offset(data: &ForecastRaw, days_ahead: i64) -> Option<ForecastModel> {
    const PREFERRED_HOUR: i64 = 12;

    let target_date = Local::now().date_naive() + chrono::Duration::days(days_ahead);

    let closest = data
        .properties
        .timeseries
        .iter()
        .filter_map(|series| {
            let time = chrono::DateTime::parse_from_rfc3339(&series.time).ok()?;
            let local_time = time.with_timezone(&Local);
            if local_time.date_naive() != target_date {
                return None;
            }
            let next_6_hours = series.data.next_6_hours.as_ref()?;
            let hour_diff = (local_time.hour() as i64 - PREFERRED_HOUR).abs();
            Some((hour_diff, next_6_hours))
        })
        .min_by_key(|(hour_diff, _)| *hour_diff)?
        .1;

    Some(ForecastModel {
        icon_name: closest.summary.symbol_code.to_owned(),
        temp: std::format!("{:.0}", closest.details.air_temperature_max),
        precip: std::format!("{:.0}", closest.details.precipitation_amount),
    })
}

fn get_icon(icon_name: String) -> Image {
    let icon_path = std::format!("weather/{}.png", icon_name);
    let icon_data = match StaticAssets::get(&icon_path) {
        Some(icon_data) => icon_data.data.into_owned(),
        None => StaticAssets::get("not-found.png")
            .unwrap()
            .data
            .into_owned(),
    };

    let weather_icon = image::load_from_memory_with_format(&icon_data, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        weather_icon.as_raw(),
        weather_icon.width(),
        weather_icon.height(),
    );

    Image::from_rgba8(buffer)
}
