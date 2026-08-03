const FXAA_VERTEX_SHADER: &str = r#"
#version 330 core

out vec2 vUv;

void main()
{
    vec2 position;

    if (gl_VertexID == 0) {
        position = vec2(-1.0, -1.0);
    } else if (gl_VertexID == 1) {
        position = vec2(3.0, -1.0);
    } else {
        position = vec2(-1.0, 3.0);
    }

    gl_Position = vec4(position, 0.0, 1.0);
    vUv = position * 0.5 + 0.5;
}
"#;

const FXAA_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform sampler2D uScene;
uniform vec2 uInverseResolution;

in vec2 vUv;

out vec4 fragColor;

const float EDGE_THRESHOLD_MIN = 0.0312;
const float EDGE_THRESHOLD_MAX = 0.125;
const float SUBPIXEL_QUALITY = 0.75;

float luminance(vec3 color)
{
    return dot(color, vec3(0.299, 0.587, 0.114));
}

void main()
{
    vec4 centerSample = texture(uScene, vUv);
    float lumaCenter = luminance(centerSample.rgb);

    float lumaNorth = luminance(
        texture(uScene, vUv + vec2(0.0, uInverseResolution.y)).rgb
    );

    float lumaSouth = luminance(
        texture(uScene, vUv - vec2(0.0, uInverseResolution.y)).rgb
    );

    float lumaEast = luminance(
        texture(uScene, vUv + vec2(uInverseResolution.x, 0.0)).rgb
    );

    float lumaWest = luminance(
        texture(uScene, vUv - vec2(uInverseResolution.x, 0.0)).rgb
    );

    float lumaMinimum = min(
        lumaCenter,
        min(
            min(lumaNorth, lumaSouth),
            min(lumaEast, lumaWest)
        )
    );

    float lumaMaximum = max(
        lumaCenter,
        max(
            max(lumaNorth, lumaSouth),
            max(lumaEast, lumaWest)
        )
    );

    float lumaRange = lumaMaximum - lumaMinimum;
    float edgeThreshold = max(
        EDGE_THRESHOLD_MIN,
        lumaMaximum * EDGE_THRESHOLD_MAX
    );

    if (lumaRange < edgeThreshold) {
        fragColor = centerSample;
        return;
    }

    float lumaNorthWest = luminance(
        texture(
            uScene,
            vUv + vec2(-uInverseResolution.x, uInverseResolution.y)
        ).rgb
    );

    float lumaNorthEast = luminance(
        texture(
            uScene,
            vUv + vec2(uInverseResolution.x, uInverseResolution.y)
        ).rgb
    );

    float lumaSouthWest = luminance(
        texture(
            uScene,
            vUv + vec2(-uInverseResolution.x, -uInverseResolution.y)
        ).rgb
    );

    float lumaSouthEast = luminance(
        texture(
            uScene,
            vUv + vec2(uInverseResolution.x, -uInverseResolution.y)
        ).rgb
    );

    float horizontalEdge = abs(
        lumaNorthWest
            + 2.0 * lumaNorth
            + lumaNorthEast
            - 2.0 * lumaCenter
    ) + abs(
        lumaSouthWest
            + 2.0 * lumaSouth
            + lumaSouthEast
            - 2.0 * lumaCenter
    );

    float verticalEdge = abs(
        lumaNorthWest
            + 2.0 * lumaWest
            + lumaSouthWest
            - 2.0 * lumaCenter
    ) + abs(
        lumaNorthEast
            + 2.0 * lumaEast
            + lumaSouthEast
            - 2.0 * lumaCenter
    );

    bool isHorizontal = horizontalEdge >= verticalEdge;

    float lumaNegative = isHorizontal ? lumaNorth : lumaWest;
    float lumaPositive = isHorizontal ? lumaSouth : lumaEast;

    float gradientNegative = abs(lumaNegative - lumaCenter);
    float gradientPositive = abs(lumaPositive - lumaCenter);

    bool useNegativeDirection = gradientNegative >= gradientPositive;
    float gradient = max(gradientNegative, gradientPositive);

    vec2 stepDirection = isHorizontal
        ? vec2(uInverseResolution.x, 0.0)
        : vec2(0.0, uInverseResolution.y);

    vec2 normalDirection = isHorizontal
        ? vec2(0.0, uInverseResolution.y)
        : vec2(uInverseResolution.x, 0.0);

    if (useNegativeDirection) {
        normalDirection = -normalDirection;
    }

    float lumaReference = 0.5 * (
        lumaCenter
            + (useNegativeDirection ? lumaNegative : lumaPositive)
    );

    vec2 edgeUv = vUv + normalDirection * 0.5;
    vec2 negativeUv = edgeUv - stepDirection;
    vec2 positiveUv = edgeUv + stepDirection;

    float gradientThreshold = gradient * 0.25;
    float negativeDelta = luminance(
        texture(uScene, negativeUv).rgb
    ) - lumaReference;

    float positiveDelta = luminance(
        texture(uScene, positiveUv).rgb
    ) - lumaReference;

    bool negativeReached = abs(negativeDelta) >= gradientThreshold;
    bool positiveReached = abs(positiveDelta) >= gradientThreshold;

    for (int i = 0; i < 8; ++i) {
        if (!negativeReached) {
            negativeUv -= stepDirection;
            negativeDelta = luminance(
                texture(uScene, negativeUv).rgb
            ) - lumaReference;
            negativeReached = abs(negativeDelta) >= gradientThreshold;
        }

        if (!positiveReached) {
            positiveUv += stepDirection;
            positiveDelta = luminance(
                texture(uScene, positiveUv).rgb
            ) - lumaReference;
            positiveReached = abs(positiveDelta) >= gradientThreshold;
        }

        if (negativeReached && positiveReached) {
            break;
        }
    }

    float negativeDistance = isHorizontal
        ? vUv.x - negativeUv.x
        : vUv.y - negativeUv.y;

    float positiveDistance = isHorizontal
        ? positiveUv.x - vUv.x
        : positiveUv.y - vUv.y;

    negativeDistance = abs(negativeDistance);
    positiveDistance = abs(positiveDistance);

    float nearestDistance = min(
        negativeDistance,
        positiveDistance
    );

    float totalDistance = max(
        negativeDistance + positiveDistance,
        0.000001
    );

    float edgeOffset = 0.5 - nearestDistance / totalDistance;

    bool nearestIsNegative = negativeDistance < positiveDistance;
    float nearestDelta = nearestIsNegative
        ? negativeDelta
        : positiveDelta;

    bool centerIsDarker = lumaCenter < lumaReference;
    bool nearestIsDarker = nearestDelta < 0.0;

    if (centerIsDarker == nearestIsDarker) {
        edgeOffset = 0.0;
    }

    float averageLuma = (
        2.0 * (
            lumaNorth
                + lumaSouth
                + lumaEast
                + lumaWest
        )
            + lumaNorthWest
            + lumaNorthEast
            + lumaSouthWest
            + lumaSouthEast
    ) / 12.0;

    float subpixelContrast = clamp(
        abs(averageLuma - lumaCenter) / max(lumaRange, 0.000001),
        0.0,
        1.0
    );

    float subpixelOffset = smoothstep(
        0.0,
        1.0,
        subpixelContrast
    );

    subpixelOffset = subpixelOffset
        * subpixelOffset
        * SUBPIXEL_QUALITY;

    float finalOffset = max(
        edgeOffset,
        subpixelOffset
    );

    vec2 finalUv = vUv + normalDirection * finalOffset;

    vec4 filteredSample = texture(uScene, finalUv);

    fragColor = vec4(
        filteredSample.rgb,
        centerSample.a
    );
}
"#;

pub(crate) struct FxaaRenderer {
    program: u32,
    vao: u32,
    scene_location: i32,
    inverse_resolution_location: i32,
}

impl FxaaRenderer {
    pub(crate) fn new() -> Result<Self, String> {

        let program = crate::compile_shader::build_program(
            FXAA_VERTEX_SHADER,
            FXAA_FRAGMENT_SHADER,
        )
        .map_err(|error| {
            format!(
                "Unable to build FXAA presentation program: {}",
                error,
            )
        })?;

        let mut vao = 0_u32;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }

        if vao == 0 {
            unsafe {
                gl::DeleteProgram(program);
            }

            return Err(
                "OpenGL failed to allocate the FXAA vertex array"
                    .to_string(),
            );
        }

        let scene_location = unsafe {
            gl::GetUniformLocation(
                program,
                b"uScene\0".as_ptr().cast(),
            )
        };

        let inverse_resolution_location = unsafe {
            gl::GetUniformLocation(
                program,
                b"uInverseResolution\0".as_ptr().cast(),
            )
        };

        if scene_location == -1 || inverse_resolution_location == -1 {
            unsafe {
                gl::DeleteVertexArrays(1, &vao);
                gl::DeleteProgram(program);
            }

            return Err(
                "FXAA presentation program is missing a required uniform"
                    .to_string(),
            );
        }

        Ok(Self {
            program,
            vao,
            scene_location,
            inverse_resolution_location,
        })
    }

    pub(crate) fn render(
        &self,
        scene_texture: u32,
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        let inverse_width = 1.0 / width as f32;
        let inverse_height = 1.0 / height as f32;

        unsafe {
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, scene_texture);
            gl::Uniform1i(self.scene_location, 0);
            gl::Uniform2f(
                self.inverse_resolution_location,
                inverse_width,
                inverse_height,
            );
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for FxaaRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
            }

            if self.program != 0 {
                gl::DeleteProgram(self.program);
            }
        }

        self.vao = 0;
        self.program = 0;
    }
}

