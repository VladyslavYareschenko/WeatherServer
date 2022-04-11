pub enum AvailableService {
    OpenWeatherMap,
    WeatherApi
}

impl AvailableService {
    pub fn iter() -> std::slice::Iter<'static, AvailableService> {
        static SERVICES: [AvailableService; 2] = 
            [AvailableService::OpenWeatherMap, AvailableService::WeatherApi];
        return SERVICES.iter();
    }
}

impl std::fmt::Display for AvailableService {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AvailableService::OpenWeatherMap => write!(f, "OpenWeatherMap"),
            AvailableService::WeatherApi => write!(f, "WeatherApi"),
        }
    }
}

impl std::str::FromStr for AvailableService {
    type Err = ();

    fn from_str(input: &str) -> Result<AvailableService, Self::Err> {
        match input {
            "OpenWeatherMap"  => Ok(AvailableService::OpenWeatherMap),
            "WeatherApi"  => Ok(AvailableService::WeatherApi),
            _ => Err(()),
        }
    }
}