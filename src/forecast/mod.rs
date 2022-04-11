pub mod available_services;
use available_services::AvailableService;

mod forecast_services { 
    pub mod openweathermap; 
    pub mod weatherapi;
}
use forecast_services::{openweathermap, weatherapi};

use weather_service_rpc::{Location, WeatherForecast};

#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    /// Internal error when performing a request or parsing response.
    Internal = 1, 

    /// Client specified an invalid argument.
    InvalidArgument = 2,
}

#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    description: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "An error occured when performing forecast request: {}.", self.description)
    }
}

impl std::error::Error for Error { }

/// Type for all final weather forecast services.  
/// Represents an abscract interface that contains a set of methods required 
/// to execute a weather forecast request by WeatherForecaster. 
#[tonic::async_trait]
trait ForecastEndService: Send + Sync {
    fn get_url(&self, loc: Location) -> String;
    async fn handle_response(&self, response: reqwest::Response) -> Result<Vec<WeatherForecast>, Error>;
}

/// WeatherForecaster makes requests for weather forecasts to the final service from AvailableService enum.
pub struct WeatherForecaster {
    provider: Box<dyn ForecastEndService>,
}

fn make_invalid_date_error() -> Error {
    Error { 
        code: ErrorCode::InvalidArgument,
        description: 
            concat!("Can't find forecast for specified data.",
                    "Make sure that you use the format mm.dd.yyyy and do not specify a past date.",).to_string()
    }
}

/// WeatherForecaster makes requests about weather forecasts to the final service from AvailableService enum.
impl WeatherForecaster {
    /// Creates new WeatherForecaster with specified AvailableService.
    pub fn new(forecast_service: AvailableService) -> Self {
        Self { provider: get_forecast_integration(forecast_service) }
    }

    /// Performs request to the endpoint provided by ForecastEndService::get_url.
    /// Response is handled by the the same ForecastEndService::handle_response.
    /// Accepts a Location struct and date for forecast in the mm.dd.yyyy form.
    /// At this point, the method returns an error with InvalidArgument code if 
    /// an unknown location or invalid date is specified.
    pub async fn get_weather(&self, loc: Location, date_string : String) -> Result<WeatherForecast, Error> {
        let requested_date = 
            chrono::NaiveDate::parse_from_str(&date_string, "%m.%d.%Y").or_else(
                |_| Err(make_invalid_date_error()))?;

        let url_string = self.provider.get_url(loc);
        let url = reqwest::Url::parse(&*url_string)
        .unwrap_or_else(|_| panic!("There was a problem parsing the url: {}", url_string));

        let response = reqwest::Client::new().get(url).send().await.or_else(
            |err| 
            Err(Error {
                code: ErrorCode::Internal,
                description: format!("Unable to make request. {}", err)
            }))?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let mut forecasts = self.provider.handle_response(response).await?;
        
                let found = forecasts.iter().position( |item| {
                    return chrono::NaiveDateTime::from_timestamp(item.dt, 0).date() == requested_date;
                });
        
                if !found.is_none() {
                    Ok(forecasts.remove(found.unwrap()))
                }
                else {  
                    Err(make_invalid_date_error())
                }
            }
            reqwest::StatusCode::BAD_REQUEST => {
                Err(Error {
                    code: ErrorCode::InvalidArgument,
                    description: response.text().await.unwrap()
                })
            },
            _ => {
                Err(Error {
                    code: ErrorCode::Internal,
                    description: response.text().await.unwrap()
                })
            }
        }
    }
}

fn get_forecast_integration(forecast_type: AvailableService) -> Box<dyn ForecastEndService>  {
    match forecast_type {
        AvailableService::OpenWeatherMap => Box::new(openweathermap::Integration),
        AvailableService::WeatherApi => Box::new(weatherapi::Integration),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast;
    use httpmock::prelude::*;

    struct StubForecastEndpoint {
        url: String,
        ok: bool,
    }

    impl StubForecastEndpoint {
        fn new(url: String, is_ok: bool) -> Self {
            Self { url: url, ok: is_ok }
        }
    }

    #[tonic::async_trait]
    impl ForecastEndService for StubForecastEndpoint {
        fn get_url(&self, _:Location) -> String {
            self.url.clone()
        }

        async fn handle_response(&self, _: reqwest::Response) -> Result<Vec<WeatherForecast>, forecast::Error> {
            if self.ok {
                Ok(vec![
                    WeatherForecast {
                        dt: 946684800, // 01.01.2000
                        min_t: 19.5,
                        max_t: 20.5,
                        avg_t: 20.0,
                        condition: "Warm and cool".to_string(),
                    },
                    WeatherForecast {
                        dt: 946771200, // 01.02.2000
                        min_t: 21.5,
                        max_t: 22.5,
                        avg_t: 22.0,
                        condition: "Warm and cool too".to_string(),
                    }
                ])
            }
            else
            {
                Err(forecast::Error{code: forecast::ErrorCode::Internal, description: "An error in stub.".to_string()})
            }
        }
    }

    fn get_any_location() -> Location {
        Location { 
            name: "Name".to_string(), 
            state: "State".to_string(), 
            country: "Country".to_string(), 
            lon: 1.1, 
            lat: 2.2,
        }
    }

    fn create_stub_forecaster(is_ok: bool) -> WeatherForecaster {
        let server = MockServer::start();

        let _ = server.mock(|when, then| {
            when.method(GET)
                .path("/forecast");
            then.status(if is_ok { reqwest::StatusCode::OK.as_u16() } 
                        else { reqwest::StatusCode::BAD_REQUEST.as_u16() });
        });

        let stub = Box::new(StubForecastEndpoint::new(format!("{}/forecast", server.base_url()), is_ok));
        WeatherForecaster { provider: stub }
    }

    #[tokio::test]
    pub async fn test_get_weather_ok_found() {
        let forecaster = create_stub_forecaster(true);

        let result = forecaster.get_weather(get_any_location(), "01.01.2000".to_string()).await;
        assert!(result.is_ok());

        let weather = result.unwrap();
        assert_eq!(weather.min_t, 19.5);
        assert_eq!(weather.max_t, 20.5);
        assert_eq!(weather.avg_t, 20.0);
        assert_eq!(weather.condition, "Warm and cool");
    }

    #[tokio::test]
    pub async fn test_get_weather_ok_not_found() {
        let forecaster = create_stub_forecaster(true);

        let result = forecaster.get_weather(get_any_location(), "01.01.1999".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, forecast::ErrorCode::InvalidArgument);
    }

    #[tokio::test]
    pub async fn test_get_weather_err() {
        let forecaster = create_stub_forecaster(false);

        let result = forecaster.get_weather(get_any_location(), "01.01.2000".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, forecast::ErrorCode::InvalidArgument);
    }

    #[tokio::test]
    pub async fn test_invalid_date_format() {
        let forecaster = create_stub_forecaster(true);

        let result = forecaster.get_weather(get_any_location(), "2000.01.01".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, forecast::ErrorCode::InvalidArgument);
    }

    #[tokio::test]
    #[should_panic]
    pub async fn test_invalid_url() {
        let stub = Box::new(StubForecastEndpoint::new("this is not an url".to_string(), true));
        let _ = WeatherForecaster { provider: stub }.
            get_weather(get_any_location(), "01.01.2000".to_string()).await;
    }

    #[tokio::test]
    pub async fn test_cant_make_request() {
        let stub = Box::new(StubForecastEndpoint::new("http://127.0.0.1:55555".to_string(), true));
        let result = WeatherForecaster { provider: stub }.
            get_weather(get_any_location(), "01.01.2000".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, forecast::ErrorCode::Internal);
    }
}