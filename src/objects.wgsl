struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Camera {
    transform: Multivector,
    vertical_height: f32,
    aspect: f32,
    line_thickness: f32,
    point_radius: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Object {
    value: Multivector,
    color: vec3<f32>,
    layer: f32,
}

struct Objects {
    count: u32,
    data: array<Object>,
}

@group(1) @binding(0)
var<storage, read> objects: Objects;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.uv = vec2<f32>(f32((input.vertex_index >> 0u) & 1u) * 2.0 - 1.0, f32((input.vertex_index >> 1u) & 1u) * 2.0 - 1.0);
    output.clip_position = vec4<f32>(output.uv, 0.0, 1.0);

    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_distance = length(input.uv);
    let pixel_direction = input.uv / pixel_distance;

    var pixel_line: Multivector;
    pixel_line.e1 = pixel_direction.x * camera.aspect;
    pixel_line.e2 = pixel_direction.y;

    var e0: Multivector;
    e0.e0 = 1.0;

    let inf_point = wedge(pixel_line, e0);
    let point_rotor = normalized(mexp(muls(inf_point, pixel_distance * camera.vertical_height * 0.5)));

    var pixel_point: Multivector;
    pixel_point.e12 = 1.0;

    let transform = mul(normalized(camera.transform), point_rotor);
    pixel_point = normalized(mul(mul(transform, pixel_point), reverse(transform)));

    var color: vec3<f32>;
    var depth = 0.0;
    var rendering = false;

    for (var i = 0u; i < objects.count; i += 1) {
        let object = objects.data[i];
        if object.layer < depth {
            continue;
        }

        let line = grade1(object.value);
        if sqr_magnitude(line) > 0.0001 && magnitude(regressive(normalized(line), pixel_point)) <= camera.line_thickness * 0.5 {
            rendering = true;
            color = object.color;
            depth = object.layer;
        }

        let point = grade2(object.value);
        if sqr_magnitude(point) > 0.0001 && magnitude(regressive(normalized(point), pixel_point)) <= camera.point_radius {
            rendering = true;
            color = object.color;
            depth = object.layer;
        }
    }

    if !rendering {
        discard;
    }
    return vec4<f32>(color, 1.0);
}

struct Multivector {
    s: f32,
    e0: f32,
    e1: f32,
    e2: f32,
    e01: f32,
    e02: f32,
    e12: f32,
    e012: f32,
}

fn grade0(m: Multivector) -> Multivector {
    var result: Multivector;
    result.s = m.s;
    return result;
}

fn grade1(m: Multivector) -> Multivector {
    var result: Multivector;
    result.e0 = m.e0;
    result.e1 = m.e1;
    result.e2 = m.e2;
    return result;
}

fn grade2(m: Multivector) -> Multivector {
    var result: Multivector;
    result.e01 = m.e01;
    result.e02 = m.e02;
    result.e12 = m.e12;
    return result;
}

fn grade3(m: Multivector) -> Multivector {
    var result: Multivector;
    result.e012 = m.e012;
    return result;
}

fn grade(m: Multivector, grade: u32) -> Multivector {
    var result: Multivector;
    if grade == 0 {
        result = grade0(m);
    }
    else if grade == 1 {
        result = grade1(m);
    }
    else if grade == 2 {
        result = grade2(m);
    }
    else if grade == 3 {
        result = grade3(m);
    }
    return result;
}

fn wedge(left: Multivector, right: Multivector) -> Multivector {
    var result: Multivector;
    for (var j = 0u; j <= 3u; j += 1u) {
        for (var k = 0u; k <= 3u; k += 1u) {
            result = add(result, grade(mul(grade(left, j), grade(right, k)), j + k));
        }
    }
    return result;
}

fn inner(left: Multivector, right: Multivector) -> Multivector {
    var result: Multivector;
    for (var j = 0u; j <= 3u; j += 1u) {
        for (var k = 0u; k <= 3u; k += 1u) {
            var difference: u32;
            if j > k {
                difference = j - k;
            }
            else {
                difference = k - j;
            }
            result = add(result, grade(mul(grade(left, j), grade(right, k)), difference));
        }
    }
    return result;
}

fn regressive(left: Multivector, right: Multivector) -> Multivector {
    return dual_inverse(wedge(dual(left), dual(right)));
}

fn reverse(m: Multivector) -> Multivector {
    var result: Multivector;
    result.s = m.s;
    result.e0 = m.e0;
    result.e1 = m.e1;
    result.e2 = m.e2;
    result.e01 = - m.e01;
    result.e02 = - m.e02;
    result.e12 = - m.e12;
    result.e012 = - m.e012;
    return result;
}

fn dual(m: Multivector) -> Multivector {
    var result: Multivector;
    result.s = m.e012;
    result.e0 = m.e12;
    result.e1 = - m.e02;
    result.e2 = m.e01;
    result.e01 = m.e2;
    result.e02 = - m.e1;
    result.e12 = m.e0;
    result.e012 = m.s;
    return result;
}

fn dual_inverse(m: Multivector) -> Multivector {
    var result: Multivector;
    result.s = m.e012;
    result.e0 = m.e12;
    result.e1 = - m.e02;
    result.e2 = m.e01;
    result.e01 = m.e2;
    result.e02 = - m.e1;
    result.e12 = m.e0;
    result.e012 = m.s;
    return result;
}

fn sqr_magnitude(m: Multivector) -> f32 {
    return mul(m, reverse(m)).s;
}

fn magnitude(m: Multivector) -> f32 {
    return sqrt(abs(sqr_magnitude(m)));
}

fn normalized(m: Multivector) -> Multivector {
    let magnitude = magnitude(m);
    if magnitude > 0.0 {
        return muls(m, 1.0 / magnitude);
    }
    else {
        return m;
    }
}

fn mexp(m: Multivector) -> Multivector {
    var result: Multivector;

    let squared = mul(m, m).s;
    if squared < 0.0 {
        let magnitude = magnitude(m);
        result.s = cos(magnitude);
        result = add(result, muls(m, sin(magnitude) / magnitude));
    }
    else if squared > 0.0 {
        let magnitude = magnitude(m);
        result.s = cosh(magnitude);
        result = add(result, muls(m, sinh(magnitude) / magnitude));
    }
    else {
        result.s = 1.0;
        result = add(result, m);
    }
    return result;
}

fn add(left: Multivector, right: Multivector) -> Multivector {
    var result: Multivector;
    result.s = left.s + right.s;
    result.e0 = left.e0 + right.e0;
    result.e1 = left.e1 + right.e1;
    result.e2 = left.e2 + right.e2;
    result.e01 = left.e01 + right.e01;
    result.e02 = left.e02 + right.e02;
    result.e12 = left.e12 + right.e12;
    result.e012 = left.e012 + right.e012;
    return result;
}

fn muls(left: Multivector, right: f32) -> Multivector {
    var result: Multivector;
    result.s = left.s * right;
    result.e0 = left.e0 * right;
    result.e1 = left.e1 * right;
    result.e2 = left.e2 * right;
    result.e01 = left.e01 * right;
    result.e02 = left.e02 * right;
    result.e12 = left.e12 * right;
    result.e012 = left.e012 * right;
    return result;
}

fn mul(left: Multivector, right: Multivector) -> Multivector {
    let _0 = left.s;
    let _1 = left.e0;
    let _2 = left.e1;
    let _3 = left.e2;
    let _4 = left.e01;
    let _5 = left.e02;
    let _6 = left.e12;
    let _7 = left.e012;
    let _8 = right.s;
    let _9 = right.e0;
    let _10 = right.e1;
    let _11 = right.e2;
    let _12 = right.e01;
    let _13 = right.e02;
    let _14 = right.e12;
    let _15 = right.e012;
    var result: Multivector;
    result.s = ((((_0 * _8) + (_10 * _2)) + (_11 * _3)) + - (_14 * _6));
    result.e0 = ((((((((_0 * _9) + (_1 * _8)) + - (_12 * _2)) + - (_13 * _3)) + (_10 * _4)) + (_11 * _5)) + - (_15 * _6)) + - (_14 * _7));
    result.e1 = ((((_0 * _10) + (_2 * _8)) + - (_14 * _3)) + (_11 * _6));
    result.e2 = ((((_0 * _11) + (_14 * _2)) + (_3 * _8)) + - (_10 * _6));
    result.e01 = ((((((((_0 * _12) + (_1 * _10)) + - (_2 * _9)) + (_15 * _3)) + (_4 * _8)) + - (_14 * _5)) + (_13 * _6)) + (_11 * _7));
    result.e02 = ((((((((_0 * _13) + (_1 * _11)) + - (_15 * _2)) + - (_3 * _9)) + (_14 * _4)) + (_5 * _8)) + - (_12 * _6)) + - (_10 * _7));
    result.e12 = ((((_0 * _14) + (_11 * _2)) + - (_10 * _3)) + (_6 * _8));
    result.e012 = ((((((((_0 * _15) + (_1 * _14)) + - (_13 * _2)) + (_12 * _3)) + (_11 * _4)) + - (_10 * _5)) + (_6 * _9)) + (_7 * _8));
    return result;
}
