#version 300 es

precision highp float;

#define gl_VertexIndex gl_VertexID

in vec3 Vertex_Position;
in float Vertex_Tile_Index;
in vec4 Vertex_Tile_Color;

out vec2 v_Uv;
out vec4 v_Color;

layout(std140) uniform Camera {  // set = 0, binding = 0
    mat4 ViewProj;
};

// TODO: merge dimensions into "sprites" buffer when that is supported in the Uniforms derive abstraction
layout(std140) uniform TextureAtlas_size {  // set = 1, binding = 0
    vec2 AtlasSize;
};

struct Rect {
    // Upper-left coordinate
    vec2 begin;
    // Bottom-right coordinate
    vec2 end;
};

layout(std140) uniform TextureAtlas_textures {  // set = 1, binding = 1
    Rect[256] Textures;
};

layout(std140) uniform Transform {  // set = 2, binding = 0
    mat4 ChunkTransform;
};

void main() {
    Rect sprite_rect = Textures[int(Vertex_Tile_Index)];
    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;

    int local_index = gl_VertexIndex % 4;

    vec3 vertex_position = vec3(
        Vertex_Position.xy * sprite_dimensions,
        0.0
    );

    // get the current row; use the index to disambiguate coordinates
    int row = int(floor(Vertex_Position.y + 0.01));
    if (local_index == 0 || local_index == 3) {
        row += 1;
    }

    // offset alternating rows
    float xoffset = floor(0.25 * sprite_dimensions.x);
    if (row % 2 == 0) {
        vertex_position.x += xoffset;
    } else {
        vertex_position.x -= xoffset;
    }

    // compact (remove gaps between rows)
    vertex_position.y -= float(row) * ceil(0.25 * sprite_dimensions.y);

    vec2 atlas_positions[4] = vec2[](
        vec2(sprite_rect.begin.x, sprite_rect.end.y),
        sprite_rect.begin,
        vec2(sprite_rect.end.x, sprite_rect.begin.y),
        sprite_rect.end
    );
    v_Uv = floor(atlas_positions[local_index]) / AtlasSize;
    v_Color = Vertex_Tile_Color;
    gl_Position = ViewProj * ChunkTransform * vec4(ceil(vertex_position), 1.0);
}
