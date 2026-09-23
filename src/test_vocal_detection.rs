use std::path::{
    Path,
    PathBuf,
};


const TEST_MODEL_FILENAME:
    &str =
    "test_singing_voice.onnx";


pub fn run(
) -> Result<(), String> {

    println!(
        "[VOCAL DETECTION TEST] ONNX model-loading proof of concept"
    );


    let model_path =
        model_path()?;


    println!(
        "[VOCAL DETECTION TEST] Model path: {}",
        model_path.display()
    );


    if !model_path.is_file() {

        return Err(
            format!(
                "Model file not found: {}",
                model_path.display()
            )
        );
    }


    let runtime_path =
        onnxruntime_library_path()?;


    println!(
        "[VOCAL DETECTION TEST] ONNX Runtime library: {}",
        runtime_path.display()
    );


    if !runtime_path.is_file() {

        return Err(
            format!(
                "ONNX Runtime shared library not found: {}",
                runtime_path.display()
            )
        );
    }


    ort::init_from(
        &runtime_path
    )
    .map_err(
        |error| {
            format!(
                "Unable to load ONNX Runtime from '{}': {}",
                runtime_path.display(),
                error,
            )
        }
    )?
    .commit();


    println!(
        "[VOCAL DETECTION TEST] ONNX Runtime initialized successfully"
    );


    let session =
        ort::session::Session::builder()
            .map_err(
                |error| {
                    format!(
                        "Unable to create ONNX Runtime session builder: {}",
                        error
                    )
                }
            )?
            .with_intra_threads(
                1
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to configure ONNX Runtime CPU thread count: {}",
                        error
                    )
                }
            )?
            .commit_from_file(
                &model_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to load ONNX model '{}': {}",
                        model_path.display(),
                        error,
                    )
                }
            )?;


    println!(
        "[VOCAL DETECTION TEST] Model loaded successfully"
    );


    println!(
        "[VOCAL DETECTION TEST] Inputs: {}",
        session.inputs().len()
    );


    for (
        index,
        input,
    ) in session.inputs()
        .iter()
        .enumerate()
    {
        println!(
            "[VOCAL DETECTION TEST]   input[{}]: name='{}' type={}",
            index,
            input.name(),
            input.dtype(),
        );
    }


    println!(
        "[VOCAL DETECTION TEST] Outputs: {}",
        session.outputs().len()
    );


    for (
        index,
        output,
    ) in session.outputs()
        .iter()
        .enumerate()
    {
        println!(
            "[VOCAL DETECTION TEST]   output[{}]: name='{}' type={}",
            index,
            output.name(),
            output.dtype(),
        );
    }


    println!(
        "[VOCAL DETECTION TEST] Checkpoint 1 passed: runtime and model are loadable."
    );


    Ok(())
}


fn model_path(
) -> Result<PathBuf, String> {

    Ok(
        screenshaver_data_directory()?
            .join(
                "models"
            )
            .join(
                TEST_MODEL_FILENAME
            )
    )
}


fn screenshaver_data_directory(
) -> Result<PathBuf, String> {

    if let Some(
        xdg_data_home
    ) =
        std::env::var_os(
            "XDG_DATA_HOME"
        )
    {
        if !xdg_data_home.is_empty() {

            return Ok(
                PathBuf::from(
                    xdg_data_home
                )
                .join(
                    "screenshaver"
                )
            );
        }
    }


    let home =
        std::env::var_os(
            "HOME"
        )
        .ok_or_else(
            || {
                "Unable to resolve Screenshaver data directory because neither XDG_DATA_HOME nor HOME is set"
                    .to_string()
            }
        )?;


    Ok(
        PathBuf::from(
            home
        )
        .join(
            ".local"
        )
        .join(
            "share"
        )
        .join(
            "screenshaver"
        )
    )
}


fn onnxruntime_library_path(
) -> Result<PathBuf, String> {

    if let Some(
        runtime_path
    ) =
        environment_runtime_path(
            "SCREENSHAVER_ONNXRUNTIME_DYLIB"
        )
    {
        return Ok(
            runtime_path
        );
    }


    if let Some(
        runtime_path
    ) =
        environment_runtime_path(
            "ORT_DYLIB_PATH"
        )
    {
        return Ok(
            runtime_path
        );
    }


    if let Some(
        runtime_path
    ) =
        option_env!(
            "SCREENSHAVER_ONNXRUNTIME_DYLIB"
        )
    {
        let path =
            PathBuf::from(
                runtime_path
            );

        if path.is_file() {

            return Ok(
                path
            );
        }
    }


    for candidate in [
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/usr/lib64/libonnxruntime.so",
        "/usr/local/lib64/libonnxruntime.so",
    ] {
        let path =
            Path::new(
                candidate
            );

        if path.is_file() {

            return Ok(
                path.to_path_buf()
            );
        }
    }


    Err(
        "Unable to locate libonnxruntime.so. Set SCREENSHAVER_ONNXRUNTIME_DYLIB or ORT_DYLIB_PATH to the full shared-library path."
            .to_string()
    )
}


fn environment_runtime_path(
    variable_name: &str,
) -> Option<PathBuf> {

    std::env::var_os(
        variable_name
    )
    .filter(
        |value| {
            !value.is_empty()
        }
    )
    .map(
        PathBuf::from
    )
}
