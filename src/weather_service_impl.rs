use super::location_search;

use super::forecast::ErrorCode;
use super::forecast::WeatherForecaster;
use super::forecast::available_services::AvailableService;

use tonic::{Code, Request, Response, Status};

use std::str::FromStr;

use weather_service_rpc::weather_service_server::{WeatherService};
use weather_service_rpc::{Locations, LocationSearchParams, WeatherProviders, WeatherForecast, WeatherQueryParams};

/// An implementation of the WeatherService trait from the weather_service_rpc crate, 
/// which is the gRPC interface for communication between the client and the server.
pub struct WeatherServiceImpl;
#[tonic::async_trait]
impl WeatherService for WeatherServiceImpl {
    /// Returns a vector of available weather forecasting services as strings.
    async fn get_weather_providers(&self, _: Request<()>) -> Result<Response<WeatherProviders>, Status> { 
        let reply = WeatherProviders{ 
            providers: AvailableService::iter().map(|service| return format!("{}", service)).collect(), 
        };

        Ok(Response::new(reply))
    }

    /// Accepts an 'LocationSearchParams' which contains query string. 
    /// Query format is usually "City,State,Country", but it is also acceptable to omit some of the fields.
    /// Returns a 'Locations' message containing a vector of 'Location' structures.
    /// Returns an empty array if no locations were found for the specified query.
    async fn get_locations(&self, search_params: Request<LocationSearchParams>) -> Result<Response<Locations>, Status> {
        let reply = location_search::perform(search_params.into_inner()).await.or_else(
            |err| Err(Status::new(Code::Internal, format!("{}", err)))
        )?;

        Ok(Response::new(reply))
    }
    
    /// Accepts an 'WeatherQueryParams' which contains one of the 'AvailableServices' as string, 
    /// 'Location' struct and date-string with format mm.dd.yyyy. 
    /// If an unknown location or invalid date format is passed, an status with code 'InvalidArgument' will be returned.
    async fn get_weather(&self, query: Request<WeatherQueryParams>) -> Result<Response<WeatherForecast>, Status> {
        let params = query.into_inner();

        let service = match AvailableService::from_str(&params.provider) {
            Ok(matched) => matched,
            Err(_) => return Err(Status::new(Code::InvalidArgument, "Invalid weather provider passed."))
        };

        let weather_forecaster = WeatherForecaster::new(service);

        match weather_forecaster.get_weather(params.location.unwrap(), params.date).await {
            Ok(weather) => Ok(Response::new(weather)),
            Err(err) => Err(Status::new(
                    match err.code {
                        ErrorCode::Internal => Code::Internal,
                        ErrorCode::InvalidArgument => Code::InvalidArgument
                    },
                    format!("{}", err)))
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use weather_service_rpc::Location;

    #[tokio::test]
    pub async fn test_get_weather_providers() {
        let providers = WeatherServiceImpl.get_weather_providers(tonic::Request::new(())).await;
        assert!(providers.is_ok());

        let expected: Vec<String> = AvailableService::iter().map(|service| return format!("{}", service)).collect();
        assert_eq!(providers.unwrap().into_inner().providers, expected)
    }

    #[tokio::test]
    pub async fn test_get_locations_ok() {
        let loc_query = "London".to_string();
        let reply = WeatherServiceImpl.get_locations(tonic::Request::new(
            LocationSearchParams {
                query: loc_query
            })).await;
        assert!(reply.is_ok());
        assert!(!reply.unwrap().into_inner().locations.is_empty());
    }

    #[tokio::test]
    pub async fn test_get_locations_not_found() {
        let loc_query = "1LondonnodnoL1".to_string();
        let reply = WeatherServiceImpl.get_locations(tonic::Request::new(
            LocationSearchParams {
                query: loc_query
            })).await;

        assert!(reply.is_ok());
        assert!(reply.unwrap().into_inner().locations.is_empty());
    }

    async fn get_location() -> Location {
        let loc_query = "London".to_string();

        let loc_reply = WeatherServiceImpl.get_locations(tonic::Request::new(
            LocationSearchParams {
                query: loc_query
            })).await;
        assert!(loc_reply.is_ok());

        let locations = loc_reply.unwrap().into_inner().locations;
        assert!(!locations.is_empty());

        return locations[0].to_owned();
    }

    #[tokio::test]
    pub async fn test_get_weather_ok() {
        let today = chrono::offset::Local::today().format("%m.%d.%Y").to_string();
        let params = WeatherQueryParams { 
            provider: AvailableService::OpenWeatherMap.to_string(), 
            location: Some(get_location().await), 
            date: today };

        let weather_reply = WeatherServiceImpl.get_weather(tonic::Request::new(params)).await;

        assert!(weather_reply.is_ok());
    }

    #[tokio::test]
    pub async fn test_get_weather_wrong_date_format() {
        let today = chrono::offset::Local::today().format("%d.%m.%Y").to_string();
        let params = WeatherQueryParams { 
            provider: AvailableService::OpenWeatherMap.to_string(), 
            location: Some(get_location().await), 
            date: today };

        let weather_reply = WeatherServiceImpl.get_weather(tonic::Request::new(params)).await;

        assert!(weather_reply.is_err());
        assert_eq!(weather_reply.err().unwrap().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    pub async fn test_get_weather_wrong_location() {
        let today = chrono::offset::Local::today().format("%m.%d.%Y").to_string();
        let params = WeatherQueryParams { 
            provider: AvailableService::OpenWeatherMap.to_string(), 
            location: Some(Location {
                name: "Name".to_string(),
                state: "State".to_string(),
                country: "Country".to_string(),
                lon: 240.0, // <- That's impossible
                lat: 240.0, // <- That's impossible
            }), 
            date: today };

        let weather_reply = WeatherServiceImpl.get_weather(tonic::Request::new(params)).await;

        assert!(weather_reply.is_err());
        assert_eq!(weather_reply.err().unwrap().code(), tonic::Code::InvalidArgument);
    }
}