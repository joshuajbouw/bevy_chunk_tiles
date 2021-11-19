//! Tiles organised into chunks for efficiency and performance.
//!
//! Mostly everything in this module is private API and not intended to be used
//! outside of this crate as a lot goes on under the hood that can cause issues.
//! With that being said, everything that can be used with helping a chunk get
//! created does live in here.
//!
//! These below examples have nothing to do with this library as all should be
//! done through the [`Tilemap`]. These are just more specific examples which
//! use the private API of this library.
//!
//! [`Tilemap`]: crate::tilemap::Tilemap
//!
//! # Simple chunk creation
//! ```
//! use bevy::asset::{prelude::*, HandleId};
//! use bevy::sprite::prelude::*;
//! use bevy_tilemap::prelude::*;
//!
//! // This must be set in Asset<TextureAtlas>.
//! let texture_atlas_handle = Handle::weak(HandleId::random::<TextureAtlas>());
//!
//! let mut tilemap = Tilemap::new(texture_atlas_handle, 32, 32);
//!
//! // There are two ways to create a new chunk. Either directly...
//!
//! tilemap.insert_chunk((0, 0));
//!
//! // Or indirectly...
//!
//! let point = (0, 0);
//! let sprite_index = 0;
//! let tile = Tile { point, sprite_index, ..Default::default() };
//! tilemap.insert_tile(tile);
//!
//! ```
//!
//! # Specifying what kind of chunk
//! ```
//! use bevy::asset::{prelude::*, HandleId};
//! use bevy::sprite::prelude::*;
//! use bevy_tilemap::prelude::*;
//!
//! // This must be set in Asset<TextureAtlas>.
//! let texture_atlas_handle = Handle::weak(HandleId::random::<TextureAtlas>());
//!
//! let mut tilemap = Tilemap::new(texture_atlas_handle, 32, 32);
//!
//! tilemap.insert_chunk((0, 0));
//!
//! let sprite_order = 0;
//! tilemap.add_layer(TilemapLayer { kind: LayerKind::Dense, ..Default::default() }, 1);
//!
//! let sprite_order = 1;
//! tilemap.add_layer(TilemapLayer { kind: LayerKind::Dense, ..Default::default() }, 1);
//! ```

/// Chunk entity.
pub(crate) mod entity;
/// Sparse and dense chunk layers.
mod layer;
/// Meshes for rendering to vertices.
pub(crate) mod mesh;
/// Files and helpers for rendering.
pub(crate) mod render;
/// Systems for chunks.
pub(crate) mod system;

use crate::{lib::*, tile::Tile};
pub use layer::LayerKind;
use layer::{DenseLayer, LayerKindInner, SparseLayer, SpriteLayer};

/// A type for sprite layers.
type SpriteLayers = Vec<Option<SpriteLayer>>;

/// A utility function that takes an array of `Tile`s and splits the indexes and
/// colors and returns them as separate vectors for use in the renderer.
pub(crate) fn dense_tiles_to_attributes(tiles: Vec<&Tile<Point3>>) -> (Vec<f32>, Vec<[f32; 4]>) {
    let capacity = tiles.len() * 4;
    let mut tile_indexes: Vec<f32> = Vec::with_capacity(capacity);
    let mut tile_colors: Vec<[f32; 4]> = Vec::with_capacity(capacity);
    for tile in tiles.iter() {
        tile_indexes.extend([tile.sprite_index as f32; 4].iter());
        tile_colors.extend([tile.tint.into(); 4].iter());
    }
    (tile_indexes, tile_colors)
}

/// A utility function that takes a sparse map of `Tile`s and splits the indexes
/// and colors and returns them as separate vectors for use in the renderer.
pub(crate) fn sparse_tiles_to_attributes(
    tile_query: &Query<&Tile<Point3>>,
    dimension: Dimension3,
    tiles: &HashMap<usize, Entity>,
) -> (Vec<f32>, Vec<[f32; 4]>) {
    let area = (dimension.width * dimension.height) as usize;
    let mut tile_indexes = vec![0.; area * 4];
    // If tiles are set with an alpha of 0, they are discarded.
    let mut tile_colors = vec![[0.0, 0.0, 0.0, 0.0]; area * 4];
    for (index, tile) in tiles.iter() {
        let tile: &Tile<Point3> = tile_query.get(*tile).expect("Can't fail");
        for i in 0..4 {
            if let Some(index) = tile_indexes.get_mut(index * 4 + i) {
                *index = tile.sprite_index as f32;
            }
            if let Some(index) = tile_colors.get_mut(index * 4 + i) {
                *index = tile.tint.into();
            }
        }
    }
    assert_eq!(area * 4, tile_colors.len());
    (tile_indexes, tile_colors)
}

#[derive(Debug, Serialize, Deserialize)]
/// A chunk which holds all the tiles to be rendered.
pub(crate) struct Chunk {
    /// The point coordinate of the chunk.
    point: Point2,
    /// The sprite layers of the chunk.
    z_layers: Vec<SpriteLayers>,
    /// Ephemeral user data that can be used for flags or other purposes.
    user_data: u128,
    /// A chunks mesh used for rendering.
    #[serde(skip)]
    mesh: Option<Handle<Mesh>>,
    /// An entity which is tied to this chunk.
    entity: Option<Entity>,
}

impl Chunk {
    /// A newly constructed chunk from a point and the maximum number of layers.
    pub(crate) fn new(
        point: Point2,
        sprite_layers: &[Option<LayerKind>],
        dimensions: Dimension3,
    ) -> Chunk {
        let mut chunk = Chunk {
            point,
            z_layers: vec![vec![None; sprite_layers.len()]; dimensions.depth as usize],
            user_data: 0,
            mesh: None,
            entity: None,
        };

        for (sprite_order, kind) in sprite_layers.iter().enumerate() {
            if let Some(kind) = kind {
                chunk.add_sprite_layer(kind, sprite_order, dimensions)
            }
        }

        chunk
    }

    /// Adds a layer from a layer kind, the z layer, and dimensions of the
    /// chunk.
    pub(crate) fn add_sprite_layer(
        &mut self,
        kind: &LayerKind,
        sprite_order: usize,
        dimensions: Dimension3,
    ) {
        for z in 0..dimensions.depth as usize {
            match kind {
                LayerKind::Dense => {
                    let tiles = vec![None; (dimensions.width * dimensions.height) as usize];
                    if let Some(z_layer) = self.z_layers.get_mut(z) {
                        if let Some(sprite_order_layer) = z_layer.get_mut(sprite_order) {
                            if !sprite_order_layer.is_some() {
                                *sprite_order_layer = Some(SpriteLayer {
                                    inner: LayerKindInner::Dense(DenseLayer::new(tiles)),
                                });
                            }
                        } else {
                            error!("sprite layer {} could not be added?", sprite_order);
                        }
                    } else {
                        error!("sprite layer {} is out of bounds", sprite_order);
                    }
                }
                LayerKind::Sparse => {
                    if let Some(z_layer) = self.z_layers.get_mut(z) {
                        if let Some(sprite_order_layer) = z_layer.get_mut(sprite_order) {
                            if !sprite_order_layer.is_some() {
                                *sprite_order_layer = Some(SpriteLayer {
                                    inner: LayerKindInner::Sparse(SparseLayer::new(
                                        HashMap::default(),
                                    )),
                                });
                            }
                        } else {
                            error!("sprite layer {} is out of bounds", sprite_order);
                        }
                    } else {
                        error!("sprite layer {} is out of bounds", sprite_order);
                    }
                }
            }
        }
    }

    /// Returns the point of the location of the chunk.
    pub(crate) fn point(&self) -> Point2 {
        self.point
    }

    /// Moves a layer from a z layer to another.
    pub(crate) fn move_sprite_layer(&mut self, from_layer_z: usize, to_layer_z: usize) {
        for sprite_layers in &mut self.z_layers {
            if let Some(layer) = sprite_layers.get(to_layer_z) {
                if layer.is_some() {
                    error!("sprite layer {} exists and can not be moved", to_layer_z);
                    return;
                }
            }
            sprite_layers.swap(from_layer_z, to_layer_z);
        }
    }

    /// Removes a layer from the specified layer.
    pub(crate) fn remove_sprite_layer(&mut self, sprite_layer: usize) {
        info!("THIS SHOULD NOT!");
        for z_layer in &mut self.z_layers {
            z_layer.remove(sprite_layer);
        }
    }

    /// Sets the mesh for the chunk layer to use.
    pub(crate) fn set_mesh(&mut self, mesh: Handle<Mesh>) {
        self.mesh = Some(mesh);
    }

    /// Returns a reference to the chunk's mesh.
    pub(crate) fn mesh(&self) -> Option<&Handle<Mesh>> {
        self.mesh.as_ref()
    }

    /// Takes the mesh handle.
    pub(crate) fn take_mesh(&mut self) -> Option<Handle<Mesh>> {
        self.mesh.take()
    }


    // /// Sets a single raw tile to be added to a z layer and index.
    // pub(crate) fn set_tile(&mut self, index: usize, z: usize, sprite_order: usize, entity: Entity) {
    //     if let Some(z_depth) = self.z_layers.get_mut(z) {
    //         if let Some(maybe_layer) = z_depth.get_mut(sprite_order) {
    //             if let Some(layer) = maybe_layer {
    //                 layer.inner.as_mut().set_tile(index, entity);
    //             } else {
    //                 error!("sprite layer {} does not exist, cannot set tile", sprite_order);
    //             }
    //         } else {
    //             error!(
    //                 "{} exceeded max number of sprite layers: {}",
    //                 sprite_order,
    //                 z_depth.len()
    //             );
    //         }
    //     } else {
    //         error!("z layer {} does not exist, cannot set tile", z);
    //     }
    // }
    //
    // /// Removes a tile from a sprite layer with a given index and z order.
    // pub(crate) fn remove_tile(
    //     &mut self,
    //     index: usize,
    //     z: usize,
    //     sprite_order: usize,
    // ) -> Option<Entity> {
    //     if let Some(z_depth) = self.z_layers.get_mut(z) {
    //         if let Some(maybe_layer) = z_depth.get_mut(sprite_order) {
    //             if let Some(layer) = maybe_layer {
    //                 layer.inner.as_mut().remove_tile(index)
    //             } else {
    //                 error!("sprite layer {} does not exist, cannot remove tile", index);
    //                 None
    //             }
    //         } else {
    //             error!(
    //                 "{} exceeded max number of sprite layers: {}",
    //                 index,
    //                 z_depth.len()
    //             );
    //             None
    //         }
    //     } else {
    //         error!("z layer {} does not exist, cannot remove tile", sprite_order);
    //         None
    //     }
    // }

    /// Sets a single raw tile to be added to a z layer and index.
    pub(crate) fn set_tile(&mut self, index: usize, z: usize, sprite_order: usize, entity: Entity) {
        if let Some(z_depth) = self.z_layers.get_mut(z) {
            if let Some(layer) = z_depth.get_mut(sprite_order) {
                if let Some(layer) = layer {
                    layer.inner.as_mut().set_tile(index, entity);
                } else {
                    error!("sprite layer {} does not exist", sprite_order);
                }
            } else {
                error!(
                    "{} exceeded max number of sprite layers: {}",
                    sprite_order,
                    z_depth.len()
                );
            }
        } else {
            error!("z layer {} does not exist", z);
        }
    }

    /// Removes a tile from a sprite layer with a given index and z order.
    pub(crate) fn remove_tile(
        &mut self,
        index: usize,
        sprite_layer: usize,
        z_depth: usize,
    ) -> Option<Entity> {
        if let Some(layers) = self.z_layers.get_mut(z_depth) {
            if let Some(layer) = layers.get_mut(sprite_layer) {
                if let Some(layer) = layer {
                    layer.inner.as_mut().remove_tile(index)
                } else {
                    error!("sprite layer {} does not exist", index);
                    None
                }
            } else {
                error!(
                    "{} exceeded max number of sprite layers: {}",
                    index,
                    layers.len()
                );
                None
            }
        } else {
            error!("sprite layer {} does not exist", sprite_layer);
            None
        }
    }

    /// Adds an entity to a z layer, always when it is spawned.
    pub(crate) fn set_entity(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }

    /// Gets the mesh entity of the chunk.
    pub(crate) fn get_entity(&self) -> Option<Entity> {
        self.entity
    }

    /// Gets the layers entity, if any. Useful for despawning.
    pub(crate) fn take_entity(&mut self) -> Option<Entity> {
        self.entity.take()
    }

    /// Gets a reference to a tile from a provided z order and index.
    pub(crate) fn get_tile(
        &self,
        index: usize,
        sprite_order: usize,
        z_depth: usize,
    ) -> Option<Entity> {
        self.z_layers.get(z_depth).and_then(|z_depth| {
            z_depth.get(sprite_order).and_then(|layer| {
                layer
                    .as_ref()
                    .and_then(|layer| layer.inner.as_ref().get_tile(index))
            })
        })
    }

    /// Clears a given layer of all sprites.
    pub(crate) fn clear_layer(&mut self, commands: &mut Commands, layer: usize) {
        // TODO: Delete all tiles from world.
        if let Some(sprite_layer) = self.z_layers.get_mut(layer) {
            for layer in sprite_layer.iter_mut().flatten() {
                layer.inner.as_mut().clear(commands);
            }
        }
    }

    /// At the given z layer, changes the tiles into attributes for use with
    /// the renderer using the given dimensions.
    ///
    /// Easier to pass in the dimensions opposed to storing it everywhere.
    pub(crate) fn tiles_to_renderer_parts(
        &self,
        tile_query: &Query<&Tile<Point3>>,
        dimensions: Dimension3,
    ) -> (Vec<f32>, Vec<[f32; 4]>) {
        let mut tile_indices = Vec::new();
        let mut tile_colors = Vec::new();
        for depth in &self.z_layers {
            for layer in depth.iter().flatten() {
                let (mut indices, mut colors) = layer
                    .inner
                    .as_ref()
                    .tiles_to_attributes(tile_query, dimensions);
                tile_indices.append(&mut indices);
                tile_colors.append(&mut colors);
            }
        }
        (tile_indices, tile_colors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer() {
        let point = Point2::new(0, 0);
        let layers = vec![
            Some(LayerKind::Dense),
            Some(LayerKind::Sparse),
            None,
            Some(LayerKind::Sparse),
            None,
        ];
        let dimensions = Dimension3::new(5, 5, 3);
        let mut chunk = Chunk::new(point, &[None, None, None, None, None], dimensions);
        for (x, layer) in layers.iter().enumerate() {
            if let Some(layer) = layer {
                chunk.add_sprite_layer(layer, x, dimensions);
            }
        }

        assert_eq!(chunk.z_layers.len(), 3);
        for layer in &chunk.z_layers {
            assert_eq!(layer.len(), 5);
        }

        chunk.move_sprite_layer(1, 2);
        let sprite_layers = chunk.z_layers.get(0).unwrap();
        assert_eq!(sprite_layers.get(1).unwrap().as_ref(), None);
        assert!(sprite_layers.get(0).unwrap().as_ref().is_some());

        chunk.remove_sprite_layer(0);
        assert_eq!(chunk.z_layers.len(), 3);
        for layer in &chunk.z_layers {
            assert_eq!(layer.len(), 4);
        }
    }
}
