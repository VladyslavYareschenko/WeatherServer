use crate::defs;
use crate::forecast;

use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use weather_service_rpc::{Location, WeatherForecast};

#[derive(Serialize, Deserialize)]
struct JSONReply {
    daily: Vec<Forecast>,
}

#[derive(Serialize, Deserialize)]
struct Forecast {
    dt: i64,
    temp: Temp,
    weather: Vec<Condition>,
}

#[derive(Serialize, Deserialize)]
struct Temp {
    min: f32,
    max: f32,
    day: f32,
}

#[derive(Serialize, Deserialize)]
struct Condition {
main: String,
    description: String
}

lazy_static! {
    static ref OPENWEATHERMAP_AUTHORIZATION: String = {
        return defs::CONFIG.general_section().get("OPENWEATHERMAP_AUTHORIZATION").unwrap().to_string();
    };
}

// An implementation of ForecastEndService to provide nessesary data and hanle reply from WeatherAPI service.
pub struct Integration;
#[tonic::async_trait]
impl forecast::ForecastEndService for Integration {
    fn get_url(&self, loc: Location) -> String {
        let forecast_excludes = "current,minutely,hourly";
        let units = "metric";

        return format!(
            "https://api.openweathermap.org/data/2.5/onecall?lat={}&lon={}&units={}&exclude={}&appid={}",
            loc.lat, loc.lon, units, forecast_excludes, *OPENWEATHERMAP_AUTHORIZATION);
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Vec<WeatherForecast>, forecast::Error> {
        let openweather_reply = response.json::<JSONReply>().await.or_else(
            |err| return Err(forecast::Error { 
                code: forecast::ErrorCode::Internal,
                description: format!("Unable to process the response from OpenWeatherMap. {}", err) 
            }))?;

        let received_locations: Vec<WeatherForecast> = openweather_reply.daily.into_iter().map(|item| {
            return WeatherForecast {
                dt: item.dt,
                min_t: item.temp.min,
                max_t: item.temp.max,
                avg_t: item.temp.day,
                condition: format!("{}, {}", item.weather[0].main, item.weather[0].description)
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
            format!(concat!("https://api.openweathermap.org/data/2.5/onecall?",
                            "lat=2.2&lon=1.1&units=metric&exclude=current,minutely,hourly&appid={}"),
                    *OPENWEATHERMAP_AUTHORIZATION))
    }

    #[tokio::test]
    pub async fn parse_response_test() {
        fn service_mock(server: &MockServer) -> httpmock::Mock {
            let data = r#"
            {
                "daily": [
                    {
                        "dt": 946684800,
                        "temp": {
                            "day": 20.0,
                            "min": 19.5,
                            "max": 20.5
                        },
                        "weather": [
                            {
                                "main": "Sky is clear",
                                "description": "warm and good"
                            }
                        ]
                    },
                    {
                        "dt": 946771200,
                        "temp": {
                            "day": 23.0,
                            "min": 22.5,
                            "max": 23.5
                        },
                        "weather": [
                            {
                                "main": "Sky is clear",
                                "description": "warm and good too"
                            }
                        ]
                    }
                ]
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
        assert_eq!(forecasts[0].condition, "Sky is clear, warm and good");

        assert_eq!(forecasts[1].dt, 946771200);
        assert_eq!(forecasts[1].max_t, 23.5);
        assert_eq!(forecasts[1].min_t, 22.5);
        assert_eq!(forecasts[1].avg_t, 23.0);
        assert_eq!(forecasts[1].condition, "Sky is clear, warm and good too");
    }
}