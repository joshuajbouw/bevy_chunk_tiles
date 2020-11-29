#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in float Vertex_Tile_Index;
layout(location = 2) in vec4 Vertex_Tile_Color;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

// TODO: merge dimensions into "sprites" buffer when that is supported in the Uniforms derive abstraction
layout(set = 1, binding = 0) uniform TextureAtlas_size {
    vec2 AtlasSize;
};

struct Rect {
    // Upper-left coordinate
    vec2 begin;
    // Bottom-right coordinate
    vec2 end;
};

layout(set = 1, binding = 1) buffer TextureAtlas_textures {
    Rect[] Textures;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 ChunkTransform;
};

layout(set = 2, binding = 1) uniform ChunkDimensions {
    vec3 Dimensions;
};

void main() {
    Rect sprite_rect = Textures[int(Vertex_Tile_Index)];
    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;

    int local_index = gl_VertexIndex % 4;

    vec3 vertex_position = vec3(
        Vertex_Position.xy * sprite_dimensions * Dimensions.xy,
        0.0
    );

    // get the current row; use the index to disambiguate coordinates
    int row = int((Vertex_Position.y + 0.501) * Dimensions.y);
    if (local_index == 0 || local_index == 3) {
        row += 1;
    }

    // row was relative to y=0, crow is relative to center
    float crow = float(row) - 0.5 - Dimensions.y * 0.5;

    // offset rows
    float xoffset = floor(0.5 * sprite_dimensions.x);
    vertex_position.x -= xoffset * crow;

    // compact (remove gaps between rows)
    vertex_position.y -= crow * ceil(0.25 * sprite_dimensions.y);

    vec2 atlas_positions[4] = vec2[](
        vec2(sprite_rect.begin.x, sprite_rect.end.y),
        sprite_rect.begin,
        vec2(sprite_rect.end.x, sprite_rect.begin.y),
        sprite_rect.end
    );
    v_Uv = (atlas_positions[local_index]) / AtlasSize;
    v_Color = Vertex_Tile_Color;
    gl_Position = ViewProj * ChunkTransform * vec4(ceil(vertex_position), 1.0);
}
