mod transportation_models;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;
use std::thread;

use chrono::{Local, TimeDelta, Utc};
use graphql_client::{GraphQLQuery, Response};
use log::{debug, info};
use reqwest;
use reqwest::header;
use slint::{
    ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel, Weak,
};
use query::Variables;
use crate::StaticAssets;
use crate::transportation::query::{QueryStopPlace, QueryStopPlaceQuays, QueryStopPlaceQuaysEstimatedCalls, ResponseData, TransportMode};
use crate::transportation::transportation_models::{TrackedQuay, TrackedStop, TrackedStops};
use crate::ui::{MainWindow, StopPlaceData, StopPlaceDataRow};

const BASE_URL: &str = "https://api.entur.io/journey-planner/v3/graphql";
// https://developer.entur.org/pages-intro-authentication
const ET_CLIENT: &str = "knowit-objectnet-trd-infoscreen";
const DEFAULT_NUMBER_OF_DEPARTURES: i64 = 8;
const POLLING_TIME: u64 = 60; // In seconds

type Date = String;
type DateTime = chrono::DateTime<Utc>;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/transportation/journey-planner_schema.json",
    query_path = "src/transportation/journey-planner_query.graphql",
    response_derives = "Debug, Clone"
)]
struct Query;

pub fn setup(window: &MainWindow) {
    let window_weak = window.as_weak();
    thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(transportation_worker_loop(window_weak))
    });
}

async fn transportation_worker_loop(window: Weak<MainWindow>) {
    loop {
        match read_tracked_stops() {
            Ok(tracked_stops) => {
                let mut stop_groups: Vec<Vec<QueryStopPlaceQuays>> = Vec::new();

                for tracked_stop in tracked_stops.stops {
                    // Run GraphQL Query
                    let result = get_stop_place(tracked_stop.clone()).await;
                    let stop_place = match result {
                        Ok(response) => match response.stop_place {
                            Some(stop_place) => stop_place,
                            None => {
                                info!("No stop places returned from server. Check if the StopPlace-ID exists.");
                                continue
                            }
                        }
                        Err(_) => {
                            info!("Error from server. Check query.");
                            continue
                        }
                    };

                    // Filter on tracked quays and lines
                    let filtered_quays = match filter_by_quays_and_lines(stop_place, tracked_stop) {
                        None => {
                            info!("No quays left after filtering. Check if Quay-IDs exist on this StopPlace.");
                            continue
                        },
                        Some(filtered_quays) => filtered_quays
                    };

                    stop_groups.push(filtered_quays);
                }

                display_transportation(&window, stop_groups);
            }
            Err(_) => {
                info! { "Unable to read file containing tracked stops information." }
            }
        };
        
        tokio::time::sleep(std::time::Duration::from_secs(POLLING_TIME)).await;
    }
}

fn display_transportation(window: &Weak<MainWindow>, stop_groups: Vec<Vec<QueryStopPlaceQuays>>) {
    let _ = window.upgrade_in_event_loop(|window: MainWindow| {

        let all_groups: VecModel<ModelRc<StopPlaceData>> = VecModel::default();

        for group in stop_groups {
            let group_data: VecModel<StopPlaceData> = VecModel::default();

            for quay in group {
                let quay_name = quay.name;
                let quay_public_code = quay.public_code.unwrap_or(String::from(""));

                debug!("Processing quay {} {}", quay_name, quay_public_code);
                let stop_place_data_rows: VecModel<StopPlaceDataRow> = VecModel::default();

                for estimated_call in quay.estimated_calls {
                    match extract_relevant_values(estimated_call) {
                        None => {
                            info!("Unable to extract values from estimated call. Skipping.");
                            continue
                        }
                        Some(row) => stop_place_data_rows.push(row)
                    };
                }

                let stop_place_data = StopPlaceData {
                    stopName: SharedString::from(format!("{} {}", quay_name, quay_public_code)),
                    stopDataRows: Rc::new(stop_place_data_rows).into()
                };

                group_data.push(stop_place_data);
            }

            all_groups.push(Rc::new(group_data).into());
        }

        window.set_stopPlacesData(Rc::new(all_groups).into());
    });
}

const DEFAULT_TRACKED_STOPS: &str = include_str!("trackedStops.json");

fn read_tracked_stops() -> Result<TrackedStops, Box<dyn Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not determine home directory")?;
    let path = home_dir.join(".transportation").join("trackedStops.json");

    if !path.try_exists()? {
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, DEFAULT_TRACKED_STOPS)?;
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let tracked_stops = serde_json::from_reader(reader)?;

    Ok(tracked_stops)
}

fn filter_by_quays_and_lines(query_stop_place: QueryStopPlace, tracked_stop: TrackedStop) -> Option<Vec<QueryStopPlaceQuays>> {
    let all_query_quays: Vec<QueryStopPlaceQuays> = match query_stop_place.quays {
        None => {
            info!("No quays for stop with ID {}", query_stop_place.id);
            return None
        },
        Some(quays) => quays.into_iter().flatten().collect::<Vec<QueryStopPlaceQuays>>()
    };
    
    let tracked_quays: Vec<TrackedQuay> = tracked_stop.quays.into_iter().flatten().collect();
    let mut picked_query_quays: Vec<QueryStopPlaceQuays> = Vec::new();
    
    for tracked_quay in tracked_quays {
        for query_quay in all_query_quays.clone() {
            if tracked_quay.id == query_quay.id {
                picked_query_quays.push(query_quay)
            }
        }
    }
    
    // TODO:
    // - Implement filtering by specific lines per quay

    return Some(picked_query_quays)
}

fn extract_relevant_values(estimated_call: QueryStopPlaceQuaysEstimatedCalls) -> Option<StopPlaceDataRow> {
    let transport_mode = match &estimated_call.service_journey.journey_pattern {
        None => {
            info!("No journey pattern found for service journey '{}'", estimated_call.service_journey.public_code.unwrap_or(String::from("unknown")));
            return None;
        },
        Some(journey_pattern) => {
            match &journey_pattern.line.transport_mode {
                None => {
                    info!("No transport mode found for line '{}'", journey_pattern.line.id);
                    return None
                },
                Some(transport_mode) => {
                    transport_mode
                }
            }
        }
    };

    // TODO: 
    // - More error handling and less unwrapping.
    
    let public_code = estimated_call
        .service_journey.journey_pattern.as_ref().unwrap()
        .line.public_code.as_ref().unwrap();

    let destination_front_text = estimated_call
        .destination_display.as_ref().unwrap()
        .front_text.as_ref().unwrap();

    let aimed_departure_time = estimated_call.aimed_departure_time;
    
    let expected_departure_time = estimated_call.expected_departure_time;

    let departure_time_formatted = match format_departure_time(expected_departure_time) {
        None => {
            info!("Unable to format departure time: {}", aimed_departure_time.to_string());
            return None
        },
        Some(value) => value,
    };
    
    let realtime = estimated_call.realtime;

    let stop_place_data_row = StopPlaceDataRow {
        transportMode: get_icon(transport_mode.as_str()),
        publicCode: SharedString::from(public_code),
        destinationFrontText: SharedString::from(destination_front_text),
        departureTime: SharedString::from(departure_time_formatted),
        realtime: realtime,
    };
    
    return Some(stop_place_data_row)
}

async fn get_stop_place(tracked_stop: TrackedStop) -> Result<ResponseData, Box<dyn Error>> {

    let mut headers = header::HeaderMap::new();
    headers.insert("Accept", header::HeaderValue::from_static("application/json"));
    headers.insert("ET-Client-Name",header::HeaderValue::from_static(ET_CLIENT));

    let variables = Variables {
        id: tracked_stop.id,
        number_of_departures: Some(DEFAULT_NUMBER_OF_DEPARTURES),
    };
    
    let request_body = Query::build_query(variables);
    
    let client = reqwest::Client::new();

    let res = client.post(BASE_URL)
        .headers(headers)
        .json(&request_body)
        .send().await?;
    
    let response_body: Response<ResponseData> = res.json().await?;

    debug!("{:#?}", response_body);

    Ok(response_body.data.expect("Missing response data"))
}

fn format_departure_time(departure_time: DateTime) -> Option<String> {
    let local_now = Local::now();
    let local_departure_time = departure_time.with_timezone(&Local);

    let time_delta = local_departure_time.signed_duration_since(local_now);

    return
    if time_delta < TimeDelta::zero() {
        info!("Data error: Expected departure time is in the past.");
        None
    } else if time_delta < TimeDelta::minutes(1) {
        Some(String::from("Nå"))
    } else if time_delta < TimeDelta::minutes(10) {
        Some(format!("{} min", time_delta.num_minutes()))
    } else {
        Some(format!("{}", local_departure_time.format("%H:%M")))
    }
}

fn get_icon(icon_name: &str) -> Image {
    let icon_path = std::format!("transport/{}.png", icon_name);
    let icon_data = match StaticAssets::get(&icon_path) {
        Some(icon_data) => icon_data.data.into_owned(),
        None => StaticAssets::get("not-found.png")
            .unwrap()
            .data
            .into_owned(),
    };

    let transport_icon = image::load_from_memory_with_format(&icon_data, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        transport_icon.as_raw(),
        transport_icon.width(),
        transport_icon.height(),
    );

    Image::from_rgba8(buffer)
}


impl TransportMode {
    fn as_str(&self) -> &'static str {
        match self {
            TransportMode::bus => "bus",
            TransportMode::tram => "tram",
            TransportMode::rail => "rail",
            _ => "unknown"
        }
    }
}