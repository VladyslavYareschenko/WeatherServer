use crate::defs;
use crate::forecast;

use serde::{Serialize, Deserialize};
use weather_service_rpc::{Location, WeatherForecast};

use lazy_static::lazy_static;

#[derive(Serialize, Deserialize)]
struct JSONReply {
    forecast: ForecastDay,
}

#[derive(Serialize, Deserialize)]
struct ForecastDay {
    forecastday: Vec<DailyForecast>,
}

#[derive(Serialize, Deserialize)]
struct DailyForecast {
    date_epoch: i64,
    day: WeatherData,
}

#[derive(Serialize, Deserialize)]
struct WeatherData {
    maxtemp_c: f32,
    mintemp_c: f32,
    avgtemp_c: f32,
    condition: Condition,
}

#[derive(Serialize, Deserialize)]
struct Condition {
    text: String,
}

lazy_static! {
    static ref WEATHER_API_AUTHORIZATION: String = {
        return defs::CONFIG.general_section().get("WEATHER_API_AUTHORIZATION").unwrap().to_string();
    };
}

// An implementation of ForecastEndService to provide nessesary data and hanle reply from WeatherAPI service.
pub struct Integration;
#[tonic::async_trait]
impl forecast::ForecastEndService for Integration {
    fn get_url(&self, loc: Location) -> String {
        return format!(
            "http://api.weatherapi.com/v1/forecast.json?key={}&q={},{}&days=10&aqi=no&alerts=no",
            *WEATHER_API_AUTHORIZATION, loc.lat, loc.lon
        );
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Vec<WeatherForecast>, forecast::Error> {
        let openweather_reply = response.json::<JSONReply>().await.or_else(
                |err| return Err(forecast::Error { 
                    code: forecast::ErrorCode::Internal,
                    description: format!("Unable to process the response from WeatherApi. {}", err) 
                }))?;

        let received_locations: Vec<WeatherForecast> = openweather_reply.forecast.forecastday.into_iter().map(
            |item| {
                return WeatherForecast {
                    dt: item.date_epoch,
                    min_t: item.day.mintemp_c,
                    max_t: item.day.maxtemp_c,
                    avg_t: item.day.avgtemp_c,
                    condition: item.day.condition.text
            }
        }).collect();

        return Ok(received_locations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::{Location, ForecastEndService};
    use httpmock::prelude::*;

    #[test]
    pub fn test_url_is_valid() {
        let loc = Location {
            name: "Name".to_string(),
            state: "State".to_string(),
            country: "Country".to_string(),
            lon: 1.1,
            lat: 2.2,
        };

        let url = Integration.get_url(loc);

        assert_eq!(url, 
            format!("http://api.weatherapi.com/v1/forecast.json?key={}&q=2.2,1.1&days=10&aqi=no&alerts=no",
                    *WEATHER_API_AUTHORIZATION))
    }

    #[tokio::test]
    pub async fn parse_response_test() {
        fn service_mock(server: &MockServer) -> httpmock::Mock {
            let data = r#"
            {
                "forecast": {
                    "forecastday": [
                        {
                            "date_epoch": 946684800,
                            "day" : {
                                "maxtemp_c": 20.5,
                                "mintemp_c": 19.5,
                                "avgtemp_c": 20.0,
                                "condition": {
                                    "text" : "It's warm and good!"
                                }
                            }
                        },
                        {
                            "date_epoch": 946771200,
                            "day" : {
                                "maxtemp_c": 23.5,
                                "mintemp_c": 22.5,
                                "avgtemp_c": 23.0,
                                "condition": {
                                    "text" : "It's warm and good too!"
                                }
                            }
                        }
                    ]
                }
            }"#;
    
            server.mock(|when, then| {
                when.method(GET)
                    .path("/forecast");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(data);
            })
        }

        let server = MockServer::start();
        service_mock(&server);
        
        let url = reqwest::Url::parse(&format!("{}/forecast", server.base_url())).unwrap();
        let response = reqwest::Client::new().get(url).send().await.unwrap();
        
        let forecasts = Integration.handle_response(response).await.unwrap();

        assert_eq!(forecasts.len(), 2);
        
        assert_eq!(forecasts[0].dt, 946684800);
        assert_eq!(forecasts[0].max_t, 20.5);
        assert_eq!(forecasts[0].min_t, 19.5);
        assert_eq!(forecasts[0].avg_t, 20.0);
        assert_eq!(forecasts[0].condition, "It's warm and good!");

        assert_eq!(forecasts[1].dt, 946771200);
        assert_eq!(forecasts[1].max_t, 23.5);
        assert_eq!(forecasts[1].min_t, 22.5);
        assert_eq!(forecasts[1].avg_t, 23.0);
        assert_eq!(forecasts[1].condition, "It's warm and good too!");
    }
}