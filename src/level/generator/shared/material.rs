use super::CHUNK_HEIGHT;
use super::block_ids::BlockIds;
use super::quad_chunk_buffer::{QuadChunkBuffer, column_at};

const FULL_SKY_LIGHT: i32 = 15;
const OPAQUE: i32 = 255;

pub fn is_water(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.water || id == block_ids.water_flowing
}

pub fn is_liquid(block_ids: &BlockIds, id: i32) -> bool {
    is_water(block_ids, id) || id == block_ids.lava || id == block_ids.lava_still
}

fn is_plant(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.dandelion
        || id == block_ids.poppy
        || id == block_ids.tall_grass
        || id == block_ids.fern
        || id == block_ids.deadbush
        || id == block_ids.red_mushroom
        || id == block_ids.brown_mushroom
        || id == block_ids.reeds
}

pub fn is_solid(block_ids: &BlockIds, id: i32) -> bool {
    !(id == block_ids.air || is_liquid(block_ids, id) || is_plant(block_ids, id) || id == block_ids.snow_layer)
}

pub fn is_opaque_cube(block_ids: &BlockIds, id: i32) -> bool {
    is_solid(block_ids, id) && id != block_ids.ice && id != block_ids.cactus && id != block_ids.mob_spawner
}

fn light_opacity(block_ids: &BlockIds, id: i32) -> i32 {
    if is_water(block_ids, id) || id == block_ids.ice {
        3
    } else if id == block_ids.oak_leaves || id == block_ids.birch_leaves || id == block_ids.spruce_leaves {
        1
    } else if id == block_ids.lava || id == block_ids.lava_still || is_opaque_cube(block_ids, id) {
        OPAQUE
    } else {
        0
    }
}

pub fn height_value(buffer: &QuadChunkBuffer, block_ids: &BlockIds, x: i32, z: i32) -> i32 {
    let column = column_at(x, z);
    let mut height = CHUNK_HEIGHT as i32 - 1;
    while height > 0 && light_opacity(block_ids, column.read(buffer, height - 1)) == 0 {
        height -= 1;
    }
    height
}

pub fn can_see_sky(buffer: &QuadChunkBuffer, block_ids: &BlockIds, x: i32, y: i32, z: i32) -> bool {
    y >= height_value(buffer, block_ids, x, z)
}

pub fn sky_light(buffer: &QuadChunkBuffer, block_ids: &BlockIds, x: i32, y: i32, z: i32) -> i32 {
    let y = y.min(CHUNK_HEIGHT as i32 - 1);
    if y <= 0 {
        return 0;
    }

    let column = column_at(x, z);
    let mut light = FULL_SKY_LIGHT;
    for above in (y..CHUNK_HEIGHT as i32).rev() {
        light -= light_opacity(block_ids, column.read(buffer, above));
        if light <= 0 {
            return 0;
        }
    }
    light
}

pub fn has_sky_light(buffer: &QuadChunkBuffer, block_ids: &BlockIds, x: i32, y: i32, z: i32) -> bool {
    sky_light(buffer, block_ids, x, y, z) > 0
}
