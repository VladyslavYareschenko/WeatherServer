mod defs;
mod forecast;
mod location_search;
mod weather_service_impl;

use weather_service_impl::WeatherServiceImpl;

use tokio;
use tonic::transport::Server;
use weather_service_rpc::weather_service_server::WeatherServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: std::net::SocketAddr = 
        defs::CONFIG.general_section().get(defs::WEATHER_SERVER_ADDR_KEY).unwrap_or("[::1]:50051").parse().unwrap();
    println!("Running the server on address: {}", addr.to_string());

    Server::builder().add_service(WeatherServiceServer::new(WeatherServiceImpl{})).serve(addr).await?;

    Ok(())
}