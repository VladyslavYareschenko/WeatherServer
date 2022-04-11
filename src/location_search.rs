use crate::defs;

use lazy_static::lazy_static;
use reqwest;

use serde::{Deserialize};
use weather_service_rpc::{Location, Locations, LocationSearchParams};

lazy_static! {
    static ref OPENWEATHERMAP_AUTHORIZATION: String = {
        return defs::CONFIG.general_section().get("OPENWEATHERMAP_AUTHORIZATION").unwrap().to_string();
    };
}

#[derive(Deserialize)]
struct JSONItem {
    name: String,
    state: String,
    country: String,
    lon: f32,
    lat: f32,
}

async fn parse_response(response: reqwest::Response) -> Result<Vec<Location>, reqwest::Error> {
    let deserialized = response.json::<Vec<JSONItem>>().await?;
    
    Ok(deserialized.into_iter().map(|item| {
        Location {
            name: item.name,
            state: item.state,
            country: item.country,
            lon: item.lon,
            lat: item.lat
        }
    }).collect())
}

/// Function performs a simple 'GET' request from a geoservice. For now 'OpenWeatherMap' is used as the geoservice, 
/// since it provides simple way to get geolocation by "city,state,country" request.
/// The 'Locations' structure is returned, which simply contains the 'Location' vector.
/// An empty vector is returned if no locations were found for the given search parameters.
/// Returns with an error if it is impossible to perform request or to parse json-result.
pub async fn perform(search_params: LocationSearchParams) -> Result<Locations, reqwest::Error> {
    let url_string = format!(
        "http://api.openweathermap.org/geo/1.0/direct?q={}&limit=5&appid={}",
        search_params.query,
        *OPENWEATHERMAP_AUTHORIZATION
    );

    let url = reqwest::Url::parse(&*url_string)
        .unwrap_or_else(|_| panic!("There was a problem parsing the url: {}", url_string));

    let response = reqwest::Client::new().get(url).send().await?;

    Ok(Locations {
        locations: parse_response(response).await?,
    })
}

#[cfg(test)]
mod tests {
    use crate::location_search::*;
    use httpmock::prelude::*;

    fn location_mock(server: &MockServer) -> httpmock::Mock {
        let data = r#"
        [
            {
                "name": "Name",
                "state": "State",
                "country": "Country",
                "lon": 0.0,
                "lat": 1.0
            }
        ]"#;

        server.mock(|when, then| {
            when.method(GET)
                .path("/location");
            then.status(200)
                .header("content-type", "application/json")
                .body(data);
        })
    }

    #[tokio::test]
    pub async fn test_parse_response() {
        let server = MockServer::start();
        location_mock(&server);
        
        let url = reqwest::Url::parse(&format!("{}/location", server.base_url())).unwrap();
        let response = reqwest::Client::new().get(url).send().await.unwrap();
        
        let locations = parse_response(response).await.unwrap();

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].name, "Name");
        assert_eq!(locations[0].state, "State");
        assert_eq!(locations[0].country, "Country");
        assert_eq!(locations[0].lon, 0.0);
        assert_eq!(locations[0].lat, 1.0);
    }

    #[tokio::test]
    pub async fn test_perform_search() {
        let query_string = "London".to_string();

        let search_res = perform(LocationSearchParams{ query: query_string }).await;
        assert!(search_res.is_ok());
        assert!(search_res.unwrap().locations.len() > 0);
    }

    #[tokio::test]
    pub async fn perform_search_expect_not_found_test() {
        let query = "1LondonnodnoL1".to_string();

        let search_res = perform(LocationSearchParams{ query: query }).await;
        assert!(search_res.is_ok());
        assert!(search_res.unwrap().locations.len() == 0);
    }
}