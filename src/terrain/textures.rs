use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

use crate::model::SquareGrid;

use super::mesh::HeightOnlyCell;

#[allow(dead_code)]
pub fn square() -> Image {
    const TEXTURE_SIZE: usize = 32;

    let white = [255; 4];

    let mut data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for x in 0..TEXTURE_SIZE {
        let offset = x * 4;
        data[offset..(offset + 4)].copy_from_slice(&white);
        let offset = x * 4 + ((TEXTURE_SIZE - 1) * TEXTURE_SIZE * 4);
        data[offset..(offset + 4)].copy_from_slice(&white);
    }
    for y in 0..TEXTURE_SIZE {
        let offset = y * TEXTURE_SIZE * 4;
        data[offset..(offset + 4)].copy_from_slice(&white);
        let offset = y * TEXTURE_SIZE * 4 + ((TEXTURE_SIZE - 1) * 4);
        data[offset..(offset + 4)].copy_from_slice(&white);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub struct TerrainTextureBuilder<'g> {
    grid: &'g SquareGrid<HeightOnlyCell>,
    tile_size: UVec2,
}

struct Layer(f32, Color);

struct Layers(Vec<Layer>);

impl Layers {
    fn get(&self, v: f32) -> Color {
        for layer in self.0.iter() {
            if v <= layer.0 {
                return layer.1;
            }
        }

        Color::RED
    }
}

impl Default for Layers {
    fn default() -> Self {
        Self(vec![
            // water 2
            Layer(-0.50, Color::rgb_u8(51, 100, 197)),
            // water 1
            Layer(-0.95, Color::rgb_u8(57, 106, 203)),
            // sand
            Layer(0.04, Color::rgb_u8(210, 208, 125)),
            // grass 1
            Layer(0.55, Color::rgb_u8(86, 152, 23)),
            // grass 2
            Layer(0.85, Color::rgb_u8(62, 107, 18)),
            // grass 3
            Layer(1.00, Color::rgb_u8(0x1b, 0x37, 0x20)),
            // rock 1
            // Layer(0.7, Color::rgb_u8(90, 69, 60)),
            // rock 2
            // Layer(0.9, Color::rgb_u8(75, 60, 53)),
            // snow
            // Layer(1.0, Color::ANTIQUE_WHITE),
        ])
    }
}

impl<'g> TerrainTextureBuilder<'g> {
    pub fn new(grid: &'g SquareGrid<HeightOnlyCell>, tile_size: UVec2) -> Self {
        Self { grid, tile_size }
    }

    pub fn build(self) -> Image {
        let layers = Layers::default();
        let image_size = self.grid.size() * self.tile_size;
        let mut data = vec![0; (image_size.x * image_size.y * 4) as usize];

        for y in 0..self.grid.size().y {
            for x in 0..self.grid.size().x {
                let cell = self.grid.get(IVec2::new(x as i32, y as i32)).unwrap();

                for ty in 0..self.tile_size.y {
                    for tx in 0..self.tile_size.x {
                        let p = cell.interpolate(UVec2::new(tx, ty), self.tile_size);

                        let color = layers.get(p as f32);
                        let color = color.as_rgba_u8();

                        let iy = (y * self.tile_size.y) + ty;
                        let ix = (x * self.tile_size.x) + tx;
                        let pixel = ((iy * image_size.x * 4) + ix * 4) as usize;
                        data[pixel..pixel + 4].copy_from_slice(&color);
                    }
                }
            }
        }

        if false {
            image::save_buffer_with_format(
                "terrain.png",
                &data,
                image_size.x,
                image_size.y,
                image::ExtendedColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .unwrap();
        }

        Image::new(
            Extent3d {
                width: image_size.x,
                height: image_size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}
