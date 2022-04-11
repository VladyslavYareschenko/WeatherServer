# WeatherServer

WeatherServer is a toy project that allows you to receive weather forecasts from different online weather services. The following services are currently supported:
    - [OpenWeatherMap]
    - [WeatherApi]
    
The server uses the gRPC connection to communicate with the client. The implementation of the service over which the communication is going can be found [here](https://github.com/VladyslavYareschenko/weather_service_rpc).
    
To start server you need to create configuration file named `.weather_server_config` and add following configuration fields:
| Field | Description |
| ------ | ------ |
| OPENWEATHERMAP_AUTHORIZATION | API key for OpenWeatherMap service. You need to register and get the key. |
| WEATHER_API_AUTHORIZATION | API key for WeatherAPI service. You need to register and get the key. |
| WEATHER_SERVER_ADDRESS | The address on which the server will run. By default, the server will use the address [::1]:50051 (IPv6 loopback). |