#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Biome {
    #[default]
    Void,
    RainForest,
    Swampland,
    SeasonalForest,
    Forest,
    Savanna,
    ShrubLand,
    Taiga,
    Desert,
    Plains,
    IceDesert,
    Tundra,
    Nether,
    Sky,
}

pub fn biome_from_climate(temperature_index: usize, humidity_index: usize) -> Biome {
    let temperature = temperature_index as f32 / 63.0;
    let humidity = humidity_index as f32 / 63.0 * temperature;

    if temperature < 0.1 {
        Biome::Tundra
    } else if humidity < 0.2 {
        if temperature < 0.5 {
            Biome::Tundra
        } else if temperature < 0.95 {
            Biome::Savanna
        } else {
            Biome::Desert
        }
    } else if humidity > 0.5 && temperature < 0.7 {
        Biome::Swampland
    } else if temperature < 0.5 {
        Biome::Taiga
    } else if temperature < 0.97 {
        if humidity < 0.35 { Biome::ShrubLand } else { Biome::Forest }
    } else if humidity < 0.45 {
        Biome::Plains
    } else if humidity < 0.9 {
        Biome::SeasonalForest
    } else {
        Biome::RainForest
    }
}
